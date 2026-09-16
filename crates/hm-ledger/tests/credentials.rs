#![forbid(unsafe_code)]

use hm_core::{ActorId, Error, ErrorCode};
use hm_ledger::credentials::{CREDENTIAL_MAGIC, CredentialVault, consent_key};
use hm_ledger::keyring::{EntropySource, KeyHierarchy};
use hm_ledger::rotate::rotate_keys;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

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

const PROVIDER: &str = "repository-host";
const CONNECTOR: [u8; 16] = [
    0x0a, 0x1b, 0x2c, 0x3d, 0x4e, 0x5f, 0x60, 0x71, 0x82, 0x93, 0xa4, 0xb5, 0xc6, 0xd7, 0xe8, 0xf9,
];
const CONNECTOR_HEX: &str = "0a1b2c3d4e5f60718293a4b5c6d7e8f9";
const OTHER_CONNECTOR: [u8; 16] = [0x77; 16];
const OTHER_CONNECTOR_HEX: &str = "77777777777777777777777777777777";
const SECRET: &[u8] = b"delivery signing secret \x00\x01\xfe\xff bytes";

fn hierarchy(directory: &Path, actor: u16, user: [u8; 16], kek: &[u8; 32]) -> KeyHierarchy {
    let mut entropy = DeterministicEntropy(41);
    KeyHierarchy::open_or_create(
        directory,
        ActorId::new(actor),
        user,
        kek,
        &mut entropy,
        true,
    )
    .expect("key hierarchy")
}

fn credential_path(root: &Path, version: u32) -> std::path::PathBuf {
    root.join("connectors")
        .join(CONNECTOR_HEX)
        .join(format!("{version:010}.cred"))
}

#[test]
fn credential_round_trips_under_the_actor_key_hierarchy() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let keys = hierarchy(temporary.path(), 7, [0x11; 16], &[0x22; 32]);
    let vault = CredentialVault::open(temporary.path()).expect("open vault");

    let connectors = temporary.path().join("connectors");
    let mode = fs::metadata(&connectors)
        .expect("connectors metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);

    let mut entropy = DeterministicEntropy(97);
    vault
        .store(&keys, PROVIDER, &CONNECTOR, 1, SECRET, &mut entropy)
        .expect("store credential");

    let recovered = vault
        .secret(&keys, PROVIDER, &CONNECTOR, 1)
        .expect("read credential");
    assert_eq!(recovered.as_slice(), SECRET);

    assert_eq!(
        vault
            .store(&keys, PROVIDER, &CONNECTOR, 0, SECRET, &mut entropy)
            .expect_err("zero version")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        vault
            .secret(&keys, PROVIDER, &CONNECTOR, 9)
            .expect_err("unknown version")
            .code,
        ErrorCode::InvalidArgument
    );
}

#[test]
fn credential_aad_binds_actor_user_provider_and_connector() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let keys = hierarchy(temporary.path(), 7, [0x11; 16], &[0x22; 32]);
    let vault = CredentialVault::open(temporary.path()).expect("open vault");
    let mut entropy = DeterministicEntropy(1337);
    vault
        .store(&keys, PROVIDER, &CONNECTOR, 1, SECRET, &mut entropy)
        .expect("store credential");

    assert_eq!(
        vault
            .secret(&keys, "workspace-host", &CONNECTOR, 1)
            .expect_err("provider mismatch")
            .code,
        ErrorCode::CryptoAuthentication
    );

    let mut misplaced = DeterministicEntropy(5);
    vault
        .store(&keys, PROVIDER, &OTHER_CONNECTOR, 1, SECRET, &mut misplaced)
        .expect("store second connector");
    let moved = fs::read(credential_path(temporary.path(), 1)).expect("read first credential");
    let other_directory = temporary
        .path()
        .join("connectors")
        .join(OTHER_CONNECTOR_HEX);
    fs::write(other_directory.join("0000000001.cred"), moved).expect("relocate credential");
    assert_eq!(
        vault
            .secret(&keys, PROVIDER, &OTHER_CONNECTOR, 1)
            .expect_err("connector mismatch")
            .code,
        ErrorCode::CryptoAuthentication
    );

    let second = tempfile::tempdir().expect("second temporary directory");
    let other_actor = hierarchy(second.path(), 9, [0x11; 16], &[0x22; 32]);
    assert_eq!(
        vault
            .secret(&other_actor, PROVIDER, &CONNECTOR, 1)
            .expect_err("actor mismatch")
            .code,
        ErrorCode::CryptoAuthentication
    );

    let third = tempfile::tempdir().expect("third temporary directory");
    let other_user = hierarchy(third.path(), 7, [0x44; 16], &[0x22; 32]);
    assert_eq!(
        vault
            .secret(&other_user, PROVIDER, &CONNECTOR, 1)
            .expect_err("user mismatch")
            .code,
        ErrorCode::CryptoAuthentication
    );

    assert_eq!(
        vault
            .secret(&keys, PROVIDER, &CONNECTOR, 1)
            .expect("original still readable")
            .as_slice(),
        SECRET
    );
}

#[test]
fn credential_versions_are_additive_and_rotatable() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let keys = hierarchy(temporary.path(), 7, [0x11; 16], &[0x22; 32]);
    let vault = CredentialVault::open(temporary.path()).expect("open vault");
    let mut entropy = DeterministicEntropy(11);

    assert!(
        vault
            .versions(&CONNECTOR)
            .expect("empty versions")
            .is_empty()
    );
    assert_eq!(
        vault
            .active_version(&CONNECTOR)
            .expect_err("no versions")
            .code,
        ErrorCode::InvalidArgument
    );

    vault
        .store(&keys, PROVIDER, &CONNECTOR, 1, SECRET, &mut entropy)
        .expect("store version one");
    vault
        .store(
            &keys,
            PROVIDER,
            &CONNECTOR,
            2,
            b"rotated secret",
            &mut entropy,
        )
        .expect("store version two");

    assert_eq!(vault.versions(&CONNECTOR).expect("versions"), vec![1, 2]);
    assert_eq!(vault.active_version(&CONNECTOR).expect("active version"), 2);
    assert_eq!(
        vault
            .secret(&keys, PROVIDER, &CONNECTOR, 1)
            .expect("prior version")
            .as_slice(),
        SECRET
    );
    assert_eq!(
        vault
            .secret(&keys, PROVIDER, &CONNECTOR, 2)
            .expect("current version")
            .as_slice(),
        b"rotated secret"
    );

    assert_eq!(
        vault
            .store(&keys, PROVIDER, &CONNECTOR, 2, b"overwrite", &mut entropy)
            .expect_err("repeat store")
            .code,
        ErrorCode::AlreadyExists
    );
    assert_eq!(
        vault
            .secret(&keys, PROVIDER, &CONNECTOR, 2)
            .expect("unchanged after refusal")
            .as_slice(),
        b"rotated secret"
    );
}

#[test]
fn credential_files_are_owner_only() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let keys = hierarchy(temporary.path(), 7, [0x11; 16], &[0x22; 32]);
    let vault = CredentialVault::open(temporary.path()).expect("open vault");
    let mut entropy = DeterministicEntropy(3);
    vault
        .store(&keys, PROVIDER, &CONNECTOR, 1, SECRET, &mut entropy)
        .expect("store credential");

    let path = credential_path(temporary.path(), 1);
    let metadata = fs::metadata(&path).expect("credential metadata");
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);

    let bytes = fs::read(&path).expect("credential bytes");
    assert_eq!(&bytes[..8], CREDENTIAL_MAGIC.as_slice());
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 1);
    assert_eq!(bytes.len(), 8 + 4 + 24 + SECRET.len() + 16);
    assert!(
        !bytes.windows(SECRET.len()).any(|window| window == SECRET),
        "plaintext secret must not appear in the sealed file"
    );
    assert!(
        !temporary
            .path()
            .join("connectors")
            .join(CONNECTOR_HEX)
            .join("0000000001.cred.pending")
            .exists()
    );
}

#[test]
fn key_rotation_preserves_credentials() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let actor = ActorId::new(7);
    let user = [0x11; 16];
    let old_kek = [0x22; 32];
    let new_kek = [0x33; 32];
    let keys = hierarchy(temporary.path(), 7, user, &old_kek);
    let vault = CredentialVault::open(temporary.path()).expect("open vault");
    let mut entropy = DeterministicEntropy(59);
    vault
        .store(&keys, PROVIDER, &CONNECTOR, 1, SECRET, &mut entropy)
        .expect("store version one");
    vault
        .store(
            &keys,
            PROVIDER,
            &CONNECTOR,
            2,
            b"rotated secret",
            &mut entropy,
        )
        .expect("store version two");
    drop(keys);

    rotate_keys(
        temporary.path(),
        actor,
        user,
        &old_kek,
        &new_kek,
        &mut entropy,
    )
    .expect("rotate keys");

    let rotated = hierarchy(temporary.path(), 7, user, &new_kek);
    let reopened = CredentialVault::open(temporary.path()).expect("reopen vault");
    assert_eq!(reopened.versions(&CONNECTOR).expect("versions"), vec![1, 2]);
    assert_eq!(
        reopened
            .secret(&rotated, PROVIDER, &CONNECTOR, 1)
            .expect("version one after rotation")
            .as_slice(),
        SECRET
    );
    assert_eq!(
        reopened
            .secret(&rotated, PROVIDER, &CONNECTOR, 2)
            .expect("version two after rotation")
            .as_slice(),
        b"rotated secret"
    );
}

#[test]
fn consent_key_is_stable_and_actor_bound() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let keys = hierarchy(temporary.path(), 7, [0x11; 16], &[0x22; 32]);
    let first = consent_key(&keys);
    assert_eq!(first.len(), 32);
    drop(keys);

    let reopened = hierarchy(temporary.path(), 7, [0x11; 16], &[0x22; 32]);
    assert_eq!(consent_key(&reopened).as_slice(), first.as_slice());

    let second = tempfile::tempdir().expect("second temporary directory");
    let other_actor = hierarchy(second.path(), 9, [0x11; 16], &[0x22; 32]);
    assert_ne!(consent_key(&other_actor).as_slice(), first.as_slice());

    let third = tempfile::tempdir().expect("third temporary directory");
    let other_user = hierarchy(third.path(), 7, [0x44; 16], &[0x22; 32]);
    assert_ne!(consent_key(&other_user).as_slice(), first.as_slice());
}
