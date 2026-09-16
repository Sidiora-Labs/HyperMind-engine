use std::collections::BTreeSet;
use std::collections::HashMap;

pub const DOCUMENT_DOMAIN: &[u8] = b"hypermind.document.v1\0";
pub const CHUNK_CONTENT_DOMAIN: &[u8] = b"hypermind.document-chunk-content.v1\0";
pub const CHUNK_ID_DOMAIN: &[u8] = b"hypermind.document-chunk-id.v1\0";

#[must_use]
pub fn document_identity(name: &str, content: &[u8]) -> [u8; 32] {
    let length = u32::try_from(name.len()).unwrap_or(u32::MAX);
    let mut hasher = blake3::Hasher::new();
    hasher.update(DOCUMENT_DOMAIN);
    hasher.update(&length.to_be_bytes());
    hasher.update(name.as_bytes());
    hasher.update(content);
    *hasher.finalize().as_bytes()
}

#[must_use]
pub fn chunk_content_hash(text: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(CHUNK_CONTENT_DOMAIN);
    hasher.update(text.as_bytes());
    *hasher.finalize().as_bytes()
}

#[must_use]
pub fn chunk_identity(
    document_id: &[u8; 32],
    content_hash: &[u8; 32],
    occurrence: u32,
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(CHUNK_ID_DOMAIN);
    hasher.update(document_id);
    hasher.update(content_hash);
    hasher.update(&occurrence.to_be_bytes());
    *hasher.finalize().as_bytes()
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Claims {
    cursor: u32,
    reserved: BTreeSet<u32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OccurrenceCounter {
    claims: HashMap<[u8; 32], Claims>,
}

impl OccurrenceCounter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next(&mut self, content_hash: &[u8; 32]) -> u32 {
        let claims = self.claims.entry(*content_hash).or_default();
        while claims.cursor < u32::MAX && claims.reserved.contains(&claims.cursor) {
            claims.cursor += 1;
        }
        let occurrence = claims.cursor;
        claims.cursor = claims.cursor.saturating_add(1);
        occurrence
    }

    pub fn reserve(&mut self, content_hash: &[u8; 32], occurrence: u32) {
        self.claims
            .entry(*content_hash)
            .or_default()
            .reserved
            .insert(occurrence);
    }
}
