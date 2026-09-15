#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::checkpoint::{checkpoint_path, signing_key_pair_from_seed};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_ledger::mmr_store::MmrStore;
use std::process::Command;

#[test]
fn offline_verify_needs_no_decryption_key() {
    let temporary = tempfile::tempdir().unwrap();
    let actor_directory = temporary.path().join("actor-7");
    let signing = signing_key_pair_from_seed([0x51; 32]);
    let mut store = MmrStore::open(&actor_directory, ActorId::new(7), signing.public_key).unwrap();
    for lsn in 1..=9 {
        store
            .append_frame(&Frame {
                header: FrameHeader {
                    lsn: LSN::new(lsn),
                    kind: EventKind::UserMsg,
                    wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap()),
                    actor: ActorId::new(7),
                    conversation: ConversationId::new([u8::try_from(lsn).unwrap(); 16]),
                },
                sealed_payload: [b"NCSEAL01".as_slice(), &lsn.to_le_bytes()].concat(),
            })
            .unwrap();
    }
    let checkpoint = store.create_checkpoint(&signing).unwrap();
    drop(store);
    assert!(!actor_directory.join("keys/KEYRING").exists());
    let output = Command::new(env!("CARGO_BIN_EXE_hm"))
        .arg("verify")
        .arg(&actor_directory)
        .arg("7")
        .arg(checkpoint_path(
            &actor_directory.join("mmr/checkpoints"),
            checkpoint.lsn,
        ))
        .arg(hex(&signing.public_key))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("MEMORY VERIFIED root="));
    assert!(stdout.contains(" lsn=9"));
    assert!(stdout.contains(&hex(&checkpoint.root)));
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            write!(output, "{byte:02x}").unwrap();
            output
        },
    )
}
