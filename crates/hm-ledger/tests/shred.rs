#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::checkpoint::signing_key_pair_from_seed;
use hm_ledger::frame::{EventKind, FrameHeader};
use hm_ledger::keyring::{EntropySource, KeyHierarchy};
use hm_ledger::rotate::rotate_keys;
use hm_ledger::segment::{AppendRequest, SegmentLog, SegmentLogOptions};
use hm_ledger::shred::{crypto_shred, load_deletion_receipt, verify_deletion_receipt};
use hm_ledger::tripwire::TripwireSet;

struct DeterministicEntropy(u64);

impl EntropySource for DeterministicEntropy {
    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Error> {
        for byte in destination {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            *byte = self.0.to_le_bytes()[0];
        }
        Ok(())
    }
}

#[test]
fn rotation_rewraps_keys_without_rewriting_the_log() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorId::new(7);
    let user = [0x11; 16];
    let old_kek = [0x22; 32];
    let new_kek = [0x33; 32];
    let mut entropy = DeterministicEntropy(41);
    let hierarchy =
        KeyHierarchy::open_or_create(temporary.path(), actor, user, &old_kek, &mut entropy, true)
            .unwrap();
    let header = header(actor, 1);
    let sealed = hierarchy
        .seal(&header, b"rotation preserves this payload", &mut entropy)
        .unwrap();
    let mut log = SegmentLog::open(temporary.path(), actor, SegmentLogOptions::default()).unwrap();
    log.append_batch(&[AppendRequest {
        kind: header.kind,
        wall_timestamp_ns: header.wall_timestamp_ns,
        conversation: header.conversation,
        sealed_payload: sealed.clone(),
    }])
    .unwrap();
    let log_before = directory_bytes(&temporary.path().join("log"));
    drop(hierarchy);

    rotate_keys(
        temporary.path(),
        actor,
        user,
        &old_kek,
        &new_kek,
        &mut entropy,
    )
    .unwrap();
    assert_eq!(directory_bytes(&temporary.path().join("log")), log_before);
    assert_eq!(
        KeyHierarchy::open_or_create(temporary.path(), actor, user, &old_kek, &mut entropy, false,)
            .err()
            .unwrap()
            .code,
        ErrorCode::CryptoAuthentication
    );
    let reopened =
        KeyHierarchy::open_or_create(temporary.path(), actor, user, &new_kek, &mut entropy, false)
            .unwrap();
    assert_eq!(
        reopened.unseal(&header, &sealed).unwrap(),
        b"rotation preserves this payload"
    );
}

#[test]
fn crypto_shred_persists_a_signed_receipt_and_prevents_recreation() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorId::new(7);
    let user = [0x11; 16];
    let kek = [0x22; 32];
    let mut entropy = DeterministicEntropy(73);
    let hierarchy =
        KeyHierarchy::open_or_create(temporary.path(), actor, user, &kek, &mut entropy, true)
            .unwrap();
    let signing = signing_key_pair_from_seed([0x42; 32]);
    let root = [0x91; 32];
    let receipt = crypto_shred(hierarchy, LSN::new(44), root, &signing).unwrap();
    assert_eq!(receipt.actor, actor);
    assert_eq!(receipt.user, user);
    assert_eq!(receipt.deleted_at_lsn, LSN::new(44));
    assert_eq!(receipt.checkpoint_root, root);
    assert_ne!(receipt.key_fingerprint, [0; 32]);
    verify_deletion_receipt(&receipt, &signing.public_key).unwrap();
    assert_eq!(
        load_deletion_receipt(temporary.path(), &signing.public_key).unwrap(),
        receipt
    );
    let mut forged = receipt;
    forged.signature[0] ^= 1;
    assert_eq!(
        verify_deletion_receipt(&forged, &signing.public_key)
            .unwrap_err()
            .code,
        ErrorCode::SignatureInvalid
    );
    assert!(!temporary.path().join("keys/KEYRING").exists());
    assert_eq!(
        KeyHierarchy::open_or_create(temporary.path(), actor, user, &kek, &mut entropy, true,)
            .err()
            .unwrap()
            .code,
        ErrorCode::KeyDestroyed
    );
}

#[test]
fn tripwires_exist_only_in_the_kernel_set_and_reject_touches() {
    let tripwires = TripwireSet::seeded([LSN::new(7), LSN::new(19)]).unwrap();
    assert_eq!(tripwires.len(), 2);
    tripwires.guard([LSN::new(6), LSN::new(8)]).unwrap();
    let error = tripwires.guard([LSN::new(19)]).unwrap_err();
    assert_eq!(error.code, ErrorCode::Tripwire);
    assert_eq!(error.lsn, LSN::new(19));
}

fn header(actor: ActorId, lsn: u64) -> FrameHeader {
    FrameHeader {
        lsn: LSN::new(lsn),
        kind: EventKind::UserMsg,
        wall_timestamp_ns: UtcNanos::new(1_000),
        actor,
        conversation: ConversationId::new([0x71; 16]),
    }
}

fn directory_bytes(directory: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(path).unwrap(),
            )
        })
        .collect();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}
