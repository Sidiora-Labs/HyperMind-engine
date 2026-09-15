use anyhow::{Result, anyhow};
use hm_core::{ActorId, ErrorCode};
use hm_ledger::checkpoint::{PublicKey, load_checkpoint, verify_checkpoint_signature};
use hm_ledger::mmr_store::MmrStore;
use serde_json::{Value, json};
use std::path::Path;

pub fn run(
    actor_directory: &Path,
    actor: u16,
    checkpoint_path: &Path,
    public_key_hex: &str,
) -> Result<Value> {
    if actor == 0 {
        return Err(anyhow!(ErrorCode::InvalidArgument));
    }
    let public_key = decode_public_key(public_key_hex)?;
    let checkpoint = load_checkpoint(checkpoint_path)?;
    verify_checkpoint_signature(&checkpoint, &public_key)?;
    if checkpoint.actor != ActorId::new(actor) {
        return Err(anyhow!(ErrorCode::CheckpointMismatch));
    }
    let store = MmrStore::open(actor_directory, ActorId::new(actor), public_key)?;
    let root = store.root_at(checkpoint.leaf_count)?;
    if root != checkpoint.root {
        return Err(anyhow!(ErrorCode::CheckpointMismatch));
    }
    let root = hex(&root);
    Ok(json!({
        "ok": true,
        "actor": actor,
        "lsn": checkpoint.lsn.get(),
        "leaf_count": checkpoint.leaf_count,
        "root": root,
        "verified": true,
        "display": format!("MEMORY VERIFIED root={root} lsn={}", checkpoint.lsn.get()),
    }))
}

fn decode_public_key(value: &str) -> Result<PublicKey> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(anyhow!(ErrorCode::InvalidArgument));
    }
    let mut key = [0_u8; 32];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| anyhow!(ErrorCode::InvalidArgument))?;
    }
    Ok(key)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    )
}
