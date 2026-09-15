use hm_core::ActorId;
use hm_serve::embedded::{
    EmbeddedConfig, HyperMind, MemoryKind, RenderAuthority, RenderModel, render,
};

#[tokio::test]
async fn embedded_session_remembers_recalls_activates_and_renders_safely() {
    let temporary = tempfile::tempdir().unwrap();
    let hypermind = HyperMind::open(
        temporary.path(),
        EmbeddedConfig {
            actor: ActorId::new(9),
            user: [1; 16],
            kek: [2; 32],
            projection_map_bytes: 16 * 1024 * 1024,
        },
    )
    .await
    .unwrap();
    let session = hypermind.session("embedded-conversation");
    assert_eq!(
        session
            .remember(MemoryKind::User, "embedded heliotrope fact")
            .await
            .unwrap()
            .get(),
        1
    );
    assert_eq!(session.recall("heliotrope", 10).await.unwrap().len(), 1);
    let bundle = session.activate("heliotrope", 1024).await.unwrap();
    let rendered = render(&bundle, RenderModel::OpenAi).unwrap();
    let items: Vec<_> = rendered
        .sections
        .iter()
        .flat_map(|section| &section.items)
        .collect();
    assert!(!items.is_empty());
    assert!(items.iter().all(|item| {
        item.role == "user"
            && item.authority == RenderAuthority::UntrustedMemory
            && item.provenance_uri.starts_with("hm://9/")
    }));
    let actor = hypermind.actor();
    drop(session);
    drop(hypermind);
    actor.shutdown().await.unwrap();
}
