#![allow(clippy::too_many_lines)]

use hm_serve::config::{ActorCapability, ServerConfig};
use hm_serve::grpc::{Gateway, ListenerRole, TlsIdentity};
use hm_serve::rest::RestServer;
use hm_serve::uds::UdsServer;
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use std::sync::Arc;
use std::time::Duration;

struct Certificates {
    tls: TlsIdentity,
    client_cert: String,
    client_key: String,
}

fn certificates() -> Certificates {
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();
    let mut server_params = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let server_key = KeyPair::generate().unwrap();
    let server = server_params.signed_by(&server_key, &ca).unwrap();
    let mut client_params = CertificateParams::new(vec!["console-client".to_owned()]).unwrap();
    client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let client_key = KeyPair::generate().unwrap();
    let client = client_params.signed_by(&client_key, &ca).unwrap();
    Certificates {
        tls: TlsIdentity {
            certificate_pem: server.pem().into_bytes(),
            private_key_pem: server_key.serialize_pem().into_bytes(),
            client_ca_pem: ca.pem().into_bytes(),
        },
        client_cert: client.pem(),
        client_key: client_key.serialize_pem(),
    }
}

fn config(path: &std::path::Path) -> ServerConfig {
    ServerConfig {
        socket_path: path.join("daemon.sock"),
        data_directory: path.join("data"),
        user: [1; 16],
        kek: [2; 32],
        admin_token: [3; 32],
        actors: vec![ActorCapability {
            actor: 7,
            token: [4; 32],
        }],
        maximum_connections: 32,
        maximum_output_frames: 16,
        maximum_output_bytes: 1024 * 1024,
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn http(certs: &Certificates) -> reqwest::Client {
    reqwest::Client::builder()
        .use_rustls_tls()
        .tls_built_in_root_certs(false)
        .add_root_certificate(reqwest::Certificate::from_pem(&certs.tls.client_ca_pem).unwrap())
        .identity(
            reqwest::Identity::from_pem(
                format!("{}{}", certs.client_cert, certs.client_key).as_bytes(),
            )
            .unwrap(),
        )
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap()
}

async fn fetch(client: &reqwest::Client, url: String) -> reqwest::Response {
    client.get(url).send().await.unwrap()
}

fn header(response: &reqwest::Response, name: &str) -> String {
    response
        .headers()
        .get(name)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn console_assets_served_over_mutual_tls() {
    let temporary = tempfile::tempdir().unwrap();
    let site = temporary.path().join("site");
    std::fs::create_dir_all(&site).unwrap();
    let index = "<!doctype html><title>HyperMind console</title>";
    let script = "export const ready = true;\n";
    let style = ":root{color-scheme:light dark}\n";
    let data = "{\"schema\":\"hypermind.console-upload-session.v1\"}\n";
    let mark = "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>\n";
    std::fs::write(site.join("index.html"), index).unwrap();
    std::fs::write(site.join("console.js"), script).unwrap();
    std::fs::write(site.join("console.css"), style).unwrap();
    std::fs::write(site.join("manifest.json"), data).unwrap();
    std::fs::write(site.join("mark.svg"), mark).unwrap();
    std::fs::write(
        site.join("large.js"),
        vec![b'a'; usize::try_from(hm_serve::rest::MAXIMUM_CONSOLE_ASSET_BYTES).unwrap() + 1],
    )
    .unwrap();

    let config = Arc::new(config(temporary.path()));
    let daemon = UdsServer::bind((*config).clone())
        .await
        .unwrap()
        .with_tool_dispatcher(Arc::new(hm_mcp::dispatcher::McpToolDispatcher::default()));
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let shutdown = |mut receiver: tokio::sync::watch::Receiver<bool>| async move {
        let _ = receiver.changed().await;
    };
    let daemon_task = tokio::spawn(daemon.serve_until(shutdown(stopped.clone())));
    let certs = certificates();

    let actor = RestServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        Gateway::new(config.clone(), ListenerRole::Actor),
        certs.tls.clone(),
    )
    .await
    .unwrap()
    .with_console_directory(site.clone());
    let actor_base = format!("https://localhost:{}", actor.local_addr().unwrap().port());
    let actor_task = tokio::spawn(actor.serve_until(shutdown(stopped.clone())));

    let admin = RestServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        Gateway::new(config.clone(), ListenerRole::Admin),
        certs.tls.clone(),
    )
    .await
    .unwrap()
    .with_console_directory(site.clone());
    let admin_base = format!("https://localhost:{}", admin.local_addr().unwrap().port());
    let admin_task = tokio::spawn(admin.serve_until(shutdown(stopped.clone())));

    let bare = RestServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        Gateway::new(config.clone(), ListenerRole::Actor),
        certs.tls.clone(),
    )
    .await
    .unwrap();
    let bare_base = format!("https://localhost:{}", bare.local_addr().unwrap().port());
    let bare_task = tokio::spawn(bare.serve_until(shutdown(stopped.clone())));

    let client = http(&certs);

    let response = fetch(&client, format!("{actor_base}/console")).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        header(&response, "content-type"),
        "text/html; charset=utf-8"
    );
    assert_eq!(header(&response, "cache-control"), "no-store");
    assert_eq!(response.bytes().await.unwrap().as_ref(), index.as_bytes());

    for (name, content_type, expected) in [
        ("console.js", "text/javascript; charset=utf-8", script),
        ("console.css", "text/css; charset=utf-8", style),
        ("manifest.json", "application/json", data),
        ("mark.svg", "image/svg+xml", mark),
        ("index.html", "text/html; charset=utf-8", index),
    ] {
        let response = fetch(&client, format!("{actor_base}/console/{name}")).await;
        assert_eq!(response.status(), reqwest::StatusCode::OK, "{name}");
        assert_eq!(header(&response, "content-type"), content_type, "{name}");
        assert_eq!(header(&response, "cache-control"), "no-store", "{name}");
        assert_eq!(
            response.bytes().await.unwrap().as_ref(),
            expected.as_bytes(),
            "{name}"
        );
    }

    for name in [
        "missing.js",
        "Secret.PEM",
        "a_b.js",
        "-leading.js",
        "console.js.map",
        "console",
        "large.js",
    ] {
        let response = fetch(&client, format!("{actor_base}/console/{name}")).await;
        assert_eq!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "{name} must not be served"
        );
    }

    for path in ["console", "console/console.js"] {
        let response = fetch(&client, format!("{admin_base}/{path}")).await;
        assert_eq!(
            response.status(),
            reqwest::StatusCode::FORBIDDEN,
            "admin listener must refuse {path}"
        );
    }

    for path in ["console", "console/console.js"] {
        let response = fetch(&client, format!("{bare_base}/{path}")).await;
        assert_eq!(
            response.status(),
            reqwest::StatusCode::NOT_FOUND,
            "unconfigured listener must not serve {path}"
        );
    }

    assert!(
        hm_serve::rest::CONSOLE_ASSET_TYPES
            .iter()
            .all(|(extension, content_type)| !extension.is_empty() && !content_type.is_empty())
    );

    let _ = stop.send(true);
    let _ = actor_task.await.unwrap();
    let _ = admin_task.await.unwrap();
    let _ = bare_task.await.unwrap();
    let _ = daemon_task.await.unwrap();
}
