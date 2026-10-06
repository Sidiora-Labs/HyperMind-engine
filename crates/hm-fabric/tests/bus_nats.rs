use hm_context::types::digest_bytes;
use hm_context::ContextError;
use hm_fabric::bus::Disposition;
use hm_fabric::bus_contract::{CensusGuard, StreamFamily};
use hm_fabric::bus_nats::{NatsBinding, NatsBus, NatsCredentials};
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[derive(Deserialize)]
struct Fixture {
    url: String,
    credentials: NatsCredentials,
    bindings: Vec<NatsBinding>,
}
fn fixture(variable: &str) -> Fixture {
    let path = std::env::var_os(variable).expect("real NATS fixture environment is required");
    serde_json::from_slice(&std::fs::read(path).expect("real NATS fixture must be readable"))
        .expect("real NATS fixture must be valid")
}
async fn connect(f: &Fixture, index: usize) -> NatsBus {
    NatsBus::connect(&f.url, f.bindings[index].clone(), f.credentials.clone())
        .await
        .expect("actual preprovisioned NATS binding must connect")
}
async fn control(action: &str, argument: Option<u64>) {
    let path =
        std::env::var_os("HM_NATS_TEST_CONTROL").expect("owned broker controller is required");
    let mut command = tokio::process::Command::new("python3");
    command.arg(path).arg(action);
    if let Some(argument) = argument {
        command.arg(argument.to_string());
    }
    let output = command
        .output()
        .await
        .expect("owned broker controller must execute");
    assert!(output.status.success(), "owned broker controller failed");
}
fn digest(value: &str) -> Vec<u8> {
    digest_bytes(value.as_bytes()).into_bytes()
}
#[tokio::test]
async fn actual_preprovisioned_jetstream_delivery_registers_grants_and_restart() {
    let participant = fixture("HM_NATS_TEST_PARTICIPANT");
    let server = fixture("HM_NATS_TEST_SERVER");
    assert_eq!(participant.bindings.len(), 6);
    assert_eq!(server.bindings.len(), 6);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();
    for (index, family) in StreamFamily::ALL.into_iter().enumerate() {
        assert_eq!(server.bindings[index].family, family);
        let producer = connect(&participant, index).await;
        let consumer = connect(&server, index).await;
        let id = format!("{nonce}-family-{index}");
        let reference = digest(&id);
        let event = producer.append_once(&id, &reference).await.unwrap();
        assert_eq!(producer.append_once(&id, &reference).await.unwrap(), event);
        assert!(matches!(
            producer.append_once(&id, &digest("changed")).await,
            Err(ContextError::Conflict)
        ));
        let replay = consumer.replay(event.sequence - 1, 16).await.unwrap();
        assert!(replay.gaps.is_empty());
        assert!(replay.events.contains(&event));
        loop {
            let delivered = consumer.next().await.unwrap().unwrap();
            consumer
                .disposition(&delivered, Disposition::Ack)
                .await
                .unwrap();
            if delivered.event == event {
                break;
            }
        }
    }
    let producer = connect(&participant, 5).await;
    let consumer = connect(&server, 5).await;
    assert!(consumer.capabilities().durable);
    assert!(!consumer.capability_matrix().broker_leaf_qualified);
    assert!(!consumer.capability_matrix().broker_system_account_qualified);
    assert!(producer
        .append_once("invalid", b"content bytes")
        .await
        .is_err());
    let peer = connect(&server, 5).await;
    let first_queue = producer
        .append_once(&format!("{nonce}-queue-a"), &digest("queue-a"))
        .await
        .unwrap();
    let second_queue = producer
        .append_once(&format!("{nonce}-queue-b"), &digest("queue-b"))
        .await
        .unwrap();
    let (first_claim, second_claim) = tokio::join!(consumer.next(), peer.next());
    let first_claim = first_claim.unwrap().unwrap();
    let second_claim = second_claim.unwrap().unwrap();
    assert_ne!(first_claim.event.sequence, second_claim.event.sequence);
    assert!([first_queue.sequence, second_queue.sequence].contains(&first_claim.event.sequence));
    assert!([first_queue.sequence, second_queue.sequence].contains(&second_claim.event.sequence));
    consumer
        .disposition(&first_claim, Disposition::Ack)
        .await
        .unwrap();
    peer.disposition(&second_claim, Disposition::Ack)
        .await
        .unwrap();
    let reference = digest("progress-and-retry");
    let event = producer
        .append_once(&format!("{nonce}-retry"), &reference)
        .await
        .unwrap();
    let first = consumer.next().await.unwrap().unwrap();
    assert_eq!(first.event, event);
    consumer
        .disposition(&first, Disposition::Progress { lease_ms: 500 })
        .await
        .unwrap();
    consumer
        .disposition(&first, Disposition::Nak)
        .await
        .unwrap();
    assert!(matches!(
        consumer.disposition(&first, Disposition::Ack).await,
        Err(ContextError::Stale)
    ));
    let retry = consumer.next().await.unwrap().unwrap();
    assert_eq!(retry.event, event);
    assert_eq!(retry.attempt, 2);
    consumer
        .disposition(&retry, Disposition::Ack)
        .await
        .unwrap();
    assert!(matches!(
        consumer.disposition(&retry, Disposition::Ack).await,
        Err(ContextError::Stale)
    ));
    let term_event = producer
        .append_once(&format!("{nonce}-term"), &digest("terminal"))
        .await
        .unwrap();
    let delivery = consumer.next().await.unwrap().unwrap();
    assert_eq!(delivery.event, term_event);
    consumer
        .disposition(&delivery, Disposition::Term)
        .await
        .unwrap();
    assert!(consumer
        .dead_letters(0, 1024)
        .await
        .unwrap()
        .iter()
        .any(|d| d.delivery.event == term_event));
    let exhausted = producer
        .append_once(&format!("{nonce}-exhausted"), &digest("bounded-attempts"))
        .await
        .unwrap();
    for attempt in 1..=3 {
        let delivery = consumer.next().await.unwrap().unwrap();
        assert_eq!(delivery.event, exhausted);
        assert_eq!(delivery.attempt, attempt);
        consumer
            .disposition(&delivery, Disposition::Nak)
            .await
            .unwrap();
    }
    assert!(consumer
        .dead_letters(0, 1024)
        .await
        .unwrap()
        .iter()
        .any(|d| d.delivery.event == exhausted));
    let key = format!("agent-{nonce}");
    let a = digest("first-presence");
    let b = digest("second-presence");
    let (left, right) = tokio::join!(
        producer.register_cas(&key, 0, &a),
        consumer.register_cas(&key, 0, &b)
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert!(matches!(
        (&left, &right),
        (Err(ContextError::Conflict), _) | (_, Err(ContextError::Conflict))
    ));
    let entry = consumer.register_get(&key).await.unwrap().unwrap();
    let snapshot = consumer.register_snapshot().await.unwrap();
    let mut guard = CensusGuard::default();
    guard.observe_snapshot(&snapshot, &key);
    assert!(!guard.proven_absent());
    let deleted = consumer
        .register_delete(&key, entry.revision)
        .await
        .unwrap();
    assert!(consumer.register_get(&key).await.unwrap().is_none());
    let updates = consumer
        .register_watch_after(entry.revision, 1024)
        .await
        .unwrap();
    assert!(updates.gaps.is_empty());
    for update in updates.updates {
        guard.observe(&update, &key);
    }
    assert!(guard.proven_absent());
    guard.disconnect();
    assert!(!guard.proven_absent());
    assert!(deleted > entry.revision);
    consumer
        .register_cas(&key, 0, &digest("restored-presence"))
        .await
        .unwrap();
    let unacked = producer
        .append_once(&format!("{nonce}-restart"), &digest("restart-redelivery"))
        .await
        .unwrap();
    let before_restart = consumer.next().await.unwrap().unwrap();
    assert_eq!(before_restart.event, unacked);
    control("restart", None).await;
    let mut reopened = None;
    for _ in 0..30 {
        match NatsBus::connect(
            &server.url,
            server.bindings[5].clone(),
            server.credentials.clone(),
        )
        .await
        {
            Ok(bus) => {
                reopened = Some(bus);
                break;
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    let reopened = reopened.expect("actual broker must recover readiness");
    let page = reopened.replay(unacked.sequence - 1, 16).await.unwrap();
    assert!(page.events.contains(&unacked));
    let after_restart = reopened.next().await.unwrap().unwrap();
    assert_eq!(after_restart.event, unacked);
    assert_eq!(after_restart.attempt, 2);
    reopened
        .disposition(&after_restart, Disposition::Ack)
        .await
        .unwrap();
    assert!(matches!(
        consumer
            .disposition(&before_restart, Disposition::Ack)
            .await,
        Err(ContextError::Stale)
    ));
    assert!(consumer
        .replay(unacked.sequence - 1, 16)
        .await
        .unwrap()
        .events
        .contains(&unacked));
    let gap = producer
        .append_once(
            &format!("{nonce}-gap"),
            &digest("removed-by-administration"),
        )
        .await
        .unwrap();
    control("delete-event", Some(gap.sequence)).await;
    let page = reopened.replay(gap.sequence - 1, 16).await.unwrap();
    assert!(page.gaps.contains(&gap.sequence));
    let errors = Arc::new(AtomicUsize::new(0));
    let observed = errors.clone();
    let raw = async_nats::ConnectOptions::new()
        .user_and_password(
            participant.credentials.username.clone(),
            participant.credentials.password.clone(),
        )
        .custom_inbox_prefix(participant.bindings[5].inbox_prefix.clone())
        .request_timeout(Some(Duration::from_millis(500)))
        .event_callback(move |event| {
            let observed = observed.clone();
            async move {
                if matches!(event, async_nats::Event::ServerError(_)) {
                    observed.fetch_add(1, Ordering::SeqCst);
                }
            }
        })
        .connect(&participant.url)
        .await
        .unwrap();
    assert!(raw
        .request(
            format!("$JS.API.CONSUMER.CREATE.{}", participant.bindings[5].stream),
            "{}".into()
        )
        .await
        .is_err());
    let foreign = raw
        .subscribe(format!("{}.>", server.bindings[5].inbox_prefix))
        .await
        .unwrap();
    raw.flush().await.unwrap();
    for _ in 0..20 {
        if errors.load(Ordering::SeqCst) >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(errors.load(Ordering::SeqCst) >= 2);
    drop(foreign);
    let denied = connect(&participant, 0).await;
    let original = denied.next().await.unwrap().unwrap();
    assert!(denied
        .disposition(&original, Disposition::Term)
        .await
        .is_err());
    let redelivery = denied.next().await.unwrap().unwrap();
    assert_eq!(redelivery.event, original.event);
    assert_eq!(redelivery.attempt, original.attempt + 1);
    denied
        .disposition(&redelivery, Disposition::Ack)
        .await
        .unwrap();
    control("revoke-server", None).await;
    let revoked = reopened.register_get(&key).await.is_err();
    let reconnect_denied = NatsBus::connect(
        &server.url,
        server.bindings[5].clone(),
        server.credentials.clone(),
    )
    .await
    .is_err();
    control("restore-server", None).await;
    assert!(revoked);
    assert!(reconnect_denied);
    let restored = connect(&server, 5).await;
    assert!(restored.register_get(&key).await.unwrap().is_some());
    let mut wrong = server.bindings[5].clone();
    wrong.scope.project_id.push_str("-wrong");
    assert!(matches!(
        NatsBus::connect(&server.url, wrong, server.credentials.clone()).await,
        Err(ContextError::ScopeMismatch)
    ));
}

#[tokio::test]
#[ignore]
async fn receipt_process_child() {
    let server = fixture("HM_NATS_TEST_SERVER");
    let bus = connect(&server, 5).await;
    let delivery = bus.next().await.unwrap().unwrap();
    let path = std::env::var("HM_NATS_RECEIPT_IPC").unwrap();
    std::fs::write(&path, serde_json::to_vec(&delivery).unwrap()).unwrap();
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert!(matches!(
        bus.disposition(&delivery, Disposition::Ack).await,
        Err(ContextError::Stale)
    ));
    let receipt = bus.settlement_receipt(&delivery).await.unwrap().unwrap();
    std::fs::write(format!("{path}.receipt"), receipt.revision.to_string()).unwrap();
}

#[tokio::test]
async fn actual_cross_process_receipt_and_rekey() {
    let server = fixture("HM_NATS_TEST_SERVER");
    let bus = connect(&server, 5).await;
    for _ in 0..32 {
        match bus.next().await.unwrap() {
            Some(delivery) => bus.disposition(&delivery, Disposition::Ack).await.unwrap(),
            None => break,
        }
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string();
    bus.append_once(&format!("receipt-{nonce}"), &digest(&nonce))
        .await
        .unwrap();
    let path = std::env::temp_dir().join(format!("hm-receipt-{nonce}"));
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "receipt_process_child"])
        .env("HM_NATS_RECEIPT_IPC", &path)
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let original: hm_fabric::bus::Delivery =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    tokio::time::sleep(Duration::from_millis(550)).await;
    let redelivery = bus.next().await.unwrap().unwrap();
    assert_eq!(original.event, redelivery.event);
    assert!(redelivery.attempt > original.attempt);
    let revision = bus
        .settlement_receipt(&redelivery)
        .await
        .unwrap()
        .unwrap()
        .revision;
    assert!(child.wait().await.unwrap().success());
    let observed: u64 = std::fs::read_to_string(format!("{}.receipt", path.display()))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(observed, revision);
    bus.disposition(&redelivery, Disposition::Ack)
        .await
        .unwrap();
    assert!(bus.capability_matrix().shared_settlement_receipt_guard);
    assert!(!bus.capability_matrix().cross_process_late_ack_fence);
    let durable = bus.binding().consumer.clone();
    control("rekey-server", None).await;
    assert!(
        NatsBus::connect(&server.url, server.bindings[5].clone(), server.credentials)
            .await
            .is_err()
    );
    let rotated = fixture("HM_NATS_TEST_SERVER");
    let fresh = connect(&rotated, 5).await;
    assert_eq!(fresh.binding().consumer, durable);
    let event = fresh
        .append_once(&format!("rekey-{nonce}"), &digest("rekey"))
        .await
        .unwrap();
    let delivery = fresh.next().await.unwrap().unwrap();
    assert_eq!(delivery.event, event);
    fresh
        .disposition(&delivery, Disposition::Ack)
        .await
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(format!("{}.receipt", path.display())).unwrap();
}
