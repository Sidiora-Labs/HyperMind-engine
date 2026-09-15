#![allow(clippy::missing_errors_doc)]

use crate::frame::FrameHeader;
use crate::keyring::{EntropySource, KeyHierarchy};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hm_core::{Error, ErrorCode};
use subtle::ConstantTimeEq;

pub const SEALED_RECORD_MAGIC: &[u8; 8] = b"NCSEAL01";

const NONCE_BYTES: usize = 24;
const TAG_BYTES: usize = 16;
const DIGEST_BYTES: usize = 32;
const RECORD_DOMAIN: &[u8] = b"neocortex.record.v1";

impl KeyHierarchy {
    pub fn seal(
        &self,
        header: &FrameHeader,
        plaintext: &[u8],
        entropy: &mut impl EntropySource,
    ) -> Result<Vec<u8>, Error> {
        self.validate_actor(header)?;
        let mut nonce = [0_u8; NONCE_BYTES];
        entropy.fill(&mut nonce)?;
        let digest = blake3::hash(plaintext);
        let mut authenticated_plaintext = Vec::with_capacity(DIGEST_BYTES + plaintext.len());
        authenticated_plaintext.extend_from_slice(digest.as_bytes());
        authenticated_plaintext.extend_from_slice(plaintext);

        let cipher = XChaCha20Poly1305::new(Key::from_slice(self.data_key.as_ref()));
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &authenticated_plaintext,
                    aad: &record_aad(header, &self.user),
                },
            )
            .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))?;
        let mut sealed =
            Vec::with_capacity(SEALED_RECORD_MAGIC.len() + NONCE_BYTES + ciphertext.len());
        sealed.extend_from_slice(SEALED_RECORD_MAGIC);
        sealed.extend_from_slice(&nonce);
        sealed.extend_from_slice(&ciphertext);
        Ok(sealed)
    }

    pub fn unseal(&self, header: &FrameHeader, sealed: &[u8]) -> Result<Vec<u8>, Error> {
        self.validate_actor(header)?;
        let minimum = SEALED_RECORD_MAGIC.len() + NONCE_BYTES + DIGEST_BYTES + TAG_BYTES;
        if sealed.len() < minimum || sealed.get(..8) != Some(SEALED_RECORD_MAGIC.as_slice()) {
            return Err(Error::new(ErrorCode::LegacyPlaintext));
        }
        let nonce: &[u8; NONCE_BYTES] = sealed[8..8 + NONCE_BYTES]
            .try_into()
            .map_err(|_| Error::new(ErrorCode::LegacyPlaintext))?;
        let ciphertext = &sealed[8 + NONCE_BYTES..];
        let cipher = XChaCha20Poly1305::new(Key::from_slice(self.data_key.as_ref()));
        let plaintext = cipher
            .decrypt(
                XNonce::from_slice(nonce),
                Payload {
                    msg: ciphertext,
                    aad: &record_aad(header, &self.user),
                },
            )
            .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))?;
        if plaintext.len() < DIGEST_BYTES {
            return Err(Error::new(ErrorCode::CryptoAuthentication));
        }
        let (stored_digest, payload) = plaintext.split_at(DIGEST_BYTES);
        let actual_digest = blake3::hash(payload);
        if stored_digest.ct_eq(actual_digest.as_bytes()).unwrap_u8() != 1 {
            return Err(Error::new(ErrorCode::CryptoAuthentication));
        }
        Ok(payload.to_vec())
    }

    fn validate_actor(&self, header: &FrameHeader) -> Result<(), Error> {
        if header.actor == self.actor {
            Ok(())
        } else {
            Err(Error::new(ErrorCode::CryptoAuthentication))
        }
    }
}

fn record_aad(header: &FrameHeader, user: &[u8; 16]) -> Vec<u8> {
    let mut aad = Vec::with_capacity(RECORD_DOMAIN.len() + 2 + 8 + 1 + user.len());
    aad.extend_from_slice(RECORD_DOMAIN);
    aad.extend_from_slice(&header.actor.get().to_le_bytes());
    aad.extend_from_slice(&header.lsn.get().to_le_bytes());
    aad.push(header.kind as u8);
    aad.extend_from_slice(user);
    aad
}
