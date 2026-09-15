use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, FrameHeader};
use hm_ledger::keyring::{EntropySource, KEYRING_BYTES, KEYRING_MAGIC, KeyHierarchy};
use hm_ledger::seal::SEALED_RECORD_MAGIC;
use std::fs;
use std::os::unix::fs::PermissionsExt;

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

fn user() -> [u8; 16] {
    let mut user = [0_u8; 16];
    user[0] = 0x41;
    user[15] = 0x9d;
    user
}

fn kek() -> [u8; 32] {
    std::array::from_fn(|index| u8::try_from(index * 7 + 3).expect("KEK byte"))
}

fn header(lsn: u64) -> FrameHeader {
    let mut conversation = [0_u8; 16];
    conversation[0] = lsn.to_le_bytes()[0];
    FrameHeader {
        lsn: LSN::new(lsn),
        kind: EventKind::try_from(u8::try_from((lsn - 1) % 21 + 1).expect("kind byte"))
            .expect("kind"),
        wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).expect("test LSN") * 1_000),
        actor: ActorId::new(23),
        conversation: ConversationId::new(conversation),
    }
}

#[test]
fn donor_crypto_vectors_round_trip_and_bind_aad() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let mut entropy = DeterministicEntropy(0x8128);
    let hierarchy = KeyHierarchy::open_or_create(
        temporary.path(),
        ActorId::new(23),
        user(),
        &kek(),
        &mut entropy,
        true,
    )
    .expect("create hierarchy");

    for lsn in 1..=512 {
        let payload: Vec<u8> = (0..(lsn * 37 % 2049))
            .map(|index| ((lsn * 31 + index) & 0xff) as u8)
            .collect();
        let frame_header = header(lsn);
        let sealed = hierarchy
            .seal(&frame_header, &payload, &mut entropy)
            .expect("seal");
        assert_eq!(&sealed[..8], SEALED_RECORD_MAGIC);
        assert_eq!(sealed.len(), 8 + 24 + 32 + payload.len() + 16);
        assert_eq!(
            hierarchy.unseal(&frame_header, &sealed).expect("unseal"),
            payload
        );

        let mut tampered = sealed.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x80;
        assert_eq!(
            hierarchy
                .unseal(&frame_header, &tampered)
                .expect_err("tamper")
                .code,
            ErrorCode::CryptoAuthentication
        );

        let mut wrong = frame_header;
        wrong.lsn = LSN::new(lsn + 1);
        assert_eq!(
            hierarchy
                .unseal(&wrong, &sealed)
                .expect_err("LSN substitution")
                .code,
            ErrorCode::CryptoAuthentication
        );
        wrong = frame_header;
        wrong.kind = if frame_header.kind == EventKind::UserMsg {
            EventKind::DeliveredMsg
        } else {
            EventKind::UserMsg
        };
        assert_eq!(
            hierarchy
                .unseal(&wrong, &sealed)
                .expect_err("kind substitution")
                .code,
            ErrorCode::CryptoAuthentication
        );
    }

    assert_eq!(
        hierarchy
            .unseal(&header(1), b"raw plaintext")
            .expect_err("plaintext")
            .code,
        ErrorCode::LegacyPlaintext
    );
}

#[test]
fn keyring_layout_permissions_and_wrapping_are_durable() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let mut entropy = DeterministicEntropy(41);
    let hierarchy = KeyHierarchy::open_or_create(
        temporary.path(),
        ActorId::new(23),
        user(),
        &kek(),
        &mut entropy,
        true,
    )
    .expect("create hierarchy");
    let path = hierarchy.keyring_path().to_owned();
    let bytes = fs::read(&path).expect("read keyring");
    assert_eq!(bytes.len(), KEYRING_BYTES);
    assert_eq!(&bytes[..8], KEYRING_MAGIC);
    assert_eq!(&bytes[8..12], &1_u32.to_le_bytes());
    assert_eq!(&bytes[12..14], &23_u16.to_le_bytes());
    assert_eq!(&bytes[14..16], &[0, 0]);
    assert_eq!(&bytes[16..32], &user());
    assert_eq!(
        fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
        0o600
    );
    drop(hierarchy);

    let mut reopen_entropy = DeterministicEntropy(1);
    KeyHierarchy::open_or_create(
        temporary.path(),
        ActorId::new(23),
        user(),
        &kek(),
        &mut reopen_entropy,
        false,
    )
    .expect("reopen hierarchy");
    let mut wrong_user = user();
    wrong_user[0] ^= 1;
    assert_eq!(
        KeyHierarchy::open_or_create(
            temporary.path(),
            ActorId::new(23),
            wrong_user,
            &kek(),
            &mut reopen_entropy,
            false,
        )
        .err()
        .expect("wrong user")
        .code,
        ErrorCode::CryptoAuthentication
    );
    let mut wrong_kek = kek();
    wrong_kek[4] ^= 0x20;
    assert_eq!(
        KeyHierarchy::open_or_create(
            temporary.path(),
            ActorId::new(23),
            user(),
            &wrong_kek,
            &mut reopen_entropy,
            false,
        )
        .err()
        .expect("wrong KEK")
        .code,
        ErrorCode::CryptoAuthentication
    );
}

#[test]
fn missing_keyring_refuses_legacy_plaintext_actor_directory() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let mut entropy = DeterministicEntropy(1);
    assert_eq!(
        KeyHierarchy::open_or_create(
            temporary.path(),
            ActorId::new(23),
            user(),
            &kek(),
            &mut entropy,
            false,
        )
        .err()
        .expect("missing keyring")
        .code,
        ErrorCode::LegacyPlaintext
    );
}
