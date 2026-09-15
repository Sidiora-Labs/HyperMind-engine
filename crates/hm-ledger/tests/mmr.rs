#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::checkpoint::{
    checkpoint_path, signing_key_pair_from_seed, verify_checkpoint_signature,
};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_ledger::mmr::{Hash, Mmr, hash_frame_sealed};
use hm_ledger::mmr_store::{MmrStore, NODE_RECORD_BYTES, REPAIR_BATCH_FRAMES};

#[test]
fn sealed_leaf_vectors_and_range_proofs_are_frozen() {
    let mut random = 0xd1b5_4a32_d192_ed03_u64;
    let mut mmr = Mmr::default();
    let mut replay = Mmr::default();
    let mut leaves = Vec::with_capacity(1_024);
    for index in 0..1_024 {
        let leaf = random_hash(&mut random);
        leaves.push(leaf);
        let appended = mmr.append(leaf);
        assert_eq!(replay.append(leaf).root, appended.root);
        assert_eq!(mmr.root_at(index + 1).unwrap(), appended.root);
    }
    for _ in 0..2_048 {
        let start = next_random(&mut random) % leaves.len() as u64;
        let count = 1 + next_random(&mut random) % (leaves.len() as u64 - start);
        let proof = mmr.prove_range(start, count).unwrap();
        assert!(proof.boundary_nodes.len() <= 128);
        let range =
            &leaves[usize::try_from(start).unwrap()..usize::try_from(start + count).unwrap()];
        assert_eq!(Mmr::verify_range(range, &proof).unwrap(), mmr.root());
        let mut forged = range.to_vec();
        forged[0][0] ^= 0x80;
        assert_eq!(
            Mmr::verify_range(&forged, &proof).unwrap_err().code,
            ErrorCode::ProofInvalid
        );
        if forged.len() > 1 {
            forged.copy_from_slice(range);
            let last = forged.len() - 1;
            forged.swap(0, last);
            assert_eq!(
                Mmr::verify_range(&forged, &proof).unwrap_err().code,
                ErrorCode::ProofInvalid
            );
        }
    }

    let mut vectors = Mmr::default();
    for lsn in 1..=5 {
        let _ = vectors.append(hash_frame_sealed(
            &header(lsn),
            &[
                b'N',
                b'C',
                b'S',
                b'E',
                b'A',
                b'L',
                1,
                u8::try_from(lsn).unwrap(),
            ],
        ));
    }
    assert_eq!(
        hex(&vectors.root()),
        "e877f6f86c966d6e5c5ec8fbf15aabb365625adcd9bd6a9037c3c3fe2a8152be"
    );
}

#[test]
fn persistent_store_repairs_in_bounded_batches_and_checks_checkpoints() {
    let temporary = tempfile::tempdir().unwrap();
    let actor_directory = temporary.path().join("actor-7");
    let signing =
        signing_key_pair_from_seed(std::array::from_fn(|index| u8::try_from(index).unwrap()));
    let frames: Vec<_> = (1..=1_300).map(frame).collect();
    let mut store = MmrStore::open(&actor_directory, ActorId::new(7), signing.public_key).unwrap();
    let first = store.verify_and_repair_bounded(&frames).unwrap();
    assert_eq!(first.repaired_leaves, REPAIR_BATCH_FRAMES);
    assert!(!first.complete);
    let second = store.verify_and_repair_bounded(&frames).unwrap();
    assert_eq!(second.repaired_leaves, 1_300 - REPAIR_BATCH_FRAMES);
    assert!(second.complete);
    let checkpoint = store.create_checkpoint(&signing).unwrap();
    verify_checkpoint_signature(&checkpoint, &signing.public_key).unwrap();
    let status = store.verification_status();
    assert!(status.verified);
    assert_eq!(status.leaf_count, 1_300);
    assert_eq!(status.last_checkpoint_lsn, LSN::new(1_300));
    drop(store);

    let reopened = MmrStore::open(&actor_directory, ActorId::new(7), signing.public_key).unwrap();
    assert_eq!(reopened.verification_status(), status);
    assert_eq!(reopened.root_at(1_300).unwrap(), checkpoint.root);
    assert_eq!(
        checkpoint_path(&actor_directory.join("mmr/checkpoints"), LSN::new(1_300))
            .file_name()
            .unwrap(),
        "00000000000000001300.ckpt"
    );

    let peaks = actor_directory.join("mmr/peaks");
    let mut encoded = std::fs::read(&peaks).unwrap();
    let (first, rest) = encoded.split_at_mut(NODE_RECORD_BYTES);
    first.swap_with_slice(&mut rest[..NODE_RECORD_BYTES]);
    std::fs::write(&peaks, encoded).unwrap();
    assert_eq!(
        MmrStore::open(&actor_directory, ActorId::new(7), signing.public_key)
            .err()
            .unwrap()
            .code,
        ErrorCode::CheckpointMismatch
    );
}

#[test]
fn leaf_commits_to_header_and_sealed_payload() {
    let original = hash_frame_sealed(&header(1), b"NCSEAL01-first-ciphertext");
    assert_ne!(
        original,
        hash_frame_sealed(&header(1), b"NCSEAL01-second-ciphertext")
    );
    assert_ne!(
        original,
        hash_frame_sealed(&header(2), b"NCSEAL01-first-ciphertext")
    );
}

fn frame(lsn: u64) -> Frame {
    Frame {
        header: header(lsn),
        sealed_payload: [b"NCSEAL01".as_slice(), &lsn.to_le_bytes()].concat(),
    }
}

fn header(lsn: u64) -> FrameHeader {
    FrameHeader {
        lsn: LSN::new(lsn),
        kind: EventKind::UserMsg,
        wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn * 1_000).unwrap()),
        actor: ActorId::new(7),
        conversation: ConversationId::new([lsn.to_le_bytes()[0]; 16]),
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn random_hash(state: &mut u64) -> Hash {
    std::array::from_fn(|_| next_random(state).to_le_bytes()[0])
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
