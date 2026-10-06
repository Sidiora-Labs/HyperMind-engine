use hm_context::Scope;
use hm_core::{ActorId, ErrorCode};
use hm_mcp::{ActivateInput, McpServer};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    config::{self, ServerConfig},
    context_config::{self, TrustedContextConfig},
};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn server_config(root: &Path) -> ServerConfig {
    let path = root.join("server.conf");
    fs::write(&path, format!(
        "socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\nprojection_map_bytes={}\n",
        root.join("hm.sock").display(), root.join("actors").display(), "11".repeat(16),
        "22".repeat(32), "33".repeat(32), "44".repeat(32), 16 * 1024 * 1024,
    )).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    config::load(path).unwrap()
}

fn mapping() -> TrustedContextConfig {
    TrustedContextConfig {
        version: 1,
        actor: 7,
        scope: Scope {
            owner_id: "owner".into(),
            project_id: "project".into(),
            workspace_id: Some("worktree".into()),
        },
    }
}

fn write(path: &Path, value: &serde_json::Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn owner_only_mapping_is_explicit_and_survives_reload() {
    let root = tempfile::tempdir().unwrap();
    let config = server_config(root.path());
    let path = root.path().join("scope.json");
    write(&path, &json!(mapping()));
    assert_eq!(context_config::load(&path, &config, 7).unwrap(), mapping());
    assert_eq!(
        context_config::load_for_server(&path, &config).unwrap(),
        mapping()
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    assert_eq!(context_config::load(&path, &config, 7).unwrap(), mapping());
    assert_eq!(
        context_config::load(&path, &config, 8).unwrap_err().code,
        ErrorCode::CapabilityDenied
    );
    assert_eq!(
        context_config::load(&path, &config, 0).unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    let mut multiple = config.clone();
    multiple.actors.push(config::ActorCapability {
        actor: 8,
        token: [0x55; 32],
    });
    let mut second = mapping();
    second.actor = 8;
    write(&path, &json!(second));
    assert_eq!(
        context_config::load_for_server(&path, &multiple).unwrap(),
        second
    );
    assert_eq!(
        context_config::load(&path, &multiple, 7).unwrap_err().code,
        ErrorCode::CapabilityDenied
    );
}

#[test]
fn strict_schema_and_configured_actor_are_required() {
    let root = tempfile::tempdir().unwrap();
    let config = server_config(root.path());
    let path = root.path().join("scope.json");
    for (field, value, code) in [
        ("version", json!(2), ErrorCode::SchemaVersion),
        ("actor", json!(0), ErrorCode::InvalidArgument),
        ("actor", json!(8), ErrorCode::CapabilityDenied),
        ("actor", json!(65536), ErrorCode::InvalidArgument),
        ("extra", json!(true), ErrorCode::InvalidArgument),
    ] {
        let mut payload = json!(mapping());
        payload[field] = value;
        write(&path, &payload);
        assert_eq!(
            context_config::load(&path, &config, 7).unwrap_err().code,
            code
        );
        assert_eq!(
            context_config::load_for_server(&path, &config)
                .unwrap_err()
                .code,
            code
        );
    }
    let mut empty_scope = json!(mapping());
    empty_scope["scope"]["owner_id"] = json!("");
    write(&path, &empty_scope);
    assert_eq!(
        context_config::load(&path, &config, 7).unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    let mut config = config;
    config.actors.clear();
    write(&path, &json!(mapping()));
    assert_eq!(
        context_config::load(&path, &config, 7).unwrap_err().code,
        ErrorCode::CapabilityDenied
    );
    fs::write(&path,b"{\"version\":1,\"version\":1,\"actor\":7,\"scope\":{\"owner_id\":\"owner\",\"project_id\":\"project\",\"workspace_id\":null}}").unwrap();
    assert_eq!(
        context_config::load(&path, &config, 7).unwrap_err().code,
        ErrorCode::InvalidArgument
    );
}

#[test]
fn public_modes_symlinks_nonfiles_and_oversized_files_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let config = server_config(root.path());
    let path = root.path().join("scope.json");
    write(&path, &json!(mapping()));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        context_config::load(&path, &config, 7).unwrap_err().code,
        ErrorCode::CapabilityDenied
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let alias = root.path().join("alias.json");
    std::os::unix::fs::symlink(&path, &alias).unwrap();
    assert!(context_config::load(&alias, &config, 7).is_err());
    assert_eq!(
        context_config::load(root.path(), &config, 7)
            .unwrap_err()
            .code,
        ErrorCode::CapabilityDenied
    );
    fs::write(
        &path,
        vec![b' '; context_config::MAX_CONTEXT_CONFIG_BYTES as usize + 1],
    )
    .unwrap();
    assert_eq!(
        context_config::load(&path, &config, 7).unwrap_err().code,
        ErrorCode::InvalidLength
    );
    write(&path, &json!(mapping()));
    if rustix::process::geteuid().as_raw() == 0 {
        rustix::fs::chown(&path, Some(rustix::process::Uid::from_raw(1)), None).unwrap();
        assert_eq!(
            context_config::load(&path, &config, 7).unwrap_err().code,
            ErrorCode::CapabilityDenied
        );
    }
}

#[tokio::test]
async fn file_loaded_scope_controls_real_context_requests() {
    let root = tempfile::tempdir().unwrap();
    let config = server_config(root.path());
    let path = root.path().join("scope.json");
    write(&path, &json!(mapping()));
    let trusted = context_config::load(&path, &config, 7).unwrap();
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(7),
        actor: ActorId::new(7),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await
    .unwrap();
    let server = McpServer::new(actor.clone()).with_context_scope(trusted.scope.clone());
    let request = json!({"version":1,"scope":trusted.scope,"session_id":"session","budget":{"context_tokens":4096,"reserved_output_tokens":256,"required_tokens":0},"generation":1});
    let input = |context| ActivateInput {
        conversation: "session".into(),
        query: String::new(),
        turn_text: String::new(),
        budget_tokens: 4096,
        context: Some(context),
    };
    let accepted = server.activate_envelope(input(request.clone())).await;
    assert!(accepted.ok, "{accepted:?}");
    let mut foreign = request;
    foreign["scope"]["workspace_id"] = json!("another-tree");
    let denied = server.activate_envelope(input(foreign)).await;
    assert!(!denied.ok, "{denied:?}");
    drop(server);
    actor.shutdown().await.unwrap();
}
