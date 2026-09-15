#![allow(clippy::missing_errors_doc)]

use crate::bundle::RetrievalManifest;
use hm_core::{Error, ErrorCode, LSN};

#[must_use]
pub fn manifest_id(
    query_digest: &[u8; 32],
    snapshot_epoch: u64,
    selected: &[(LSN, [u8; 32])],
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hypermind.retrieval-manifest.v1");
    hasher.update(query_digest);
    hasher.update(&snapshot_epoch.to_le_bytes());
    for (lsn, digest) in selected {
        hasher.update(&lsn.get().to_le_bytes());
        hasher.update(digest);
    }
    *hasher.finalize().as_bytes()
}

pub fn validate_reuse(
    manifest: &RetrievalManifest,
    current_snapshot_epoch: u64,
    selected: &[(LSN, [u8; 32])],
) -> Result<(), Error> {
    if manifest.snapshot_epoch != current_snapshot_epoch
        || manifest.selected.len() != selected.len()
        || !manifest
            .selected
            .iter()
            .zip(selected)
            .all(|(lsn, (selected_lsn, _))| lsn == selected_lsn)
        || manifest.manifest_id
            != manifest_id(&manifest.query_digest, manifest.snapshot_epoch, selected)
    {
        Err(Error::new(ErrorCode::ProjectionCheckpoint))
    } else {
        Ok(())
    }
}
