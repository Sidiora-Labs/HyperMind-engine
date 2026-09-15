#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use hm_schema::protocol::MAXIMUM_PROTOCOL_PAYLOAD_BYTES;

pub const PROTOCOL_HEADER_BYTES: usize = 8;
const READ_SLOP_BYTES: usize = 64 * 1024;

pub fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, Error> {
    if payload.is_empty() || payload.len() > MAXIMUM_PROTOCOL_PAYLOAD_BYTES {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    let length = u32::try_from(payload.len()).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    let mut frame = Vec::with_capacity(PROTOCOL_HEADER_BYTES + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&crc32c::crc32c(payload).to_le_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub struct FrameParser {
    pending: Vec<u8>,
    maximum_payload_bytes: usize,
}

impl Default for FrameParser {
    fn default() -> Self {
        Self::new(MAXIMUM_PROTOCOL_PAYLOAD_BYTES)
    }
}

impl FrameParser {
    #[must_use]
    pub fn new(maximum_payload_bytes: usize) -> Self {
        Self {
            pending: Vec::with_capacity(maximum_payload_bytes.min(READ_SLOP_BYTES)),
            maximum_payload_bytes,
        }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        let maximum_pending = self
            .maximum_payload_bytes
            .checked_add(PROTOCOL_HEADER_BYTES + READ_SLOP_BYTES)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        if bytes.len() > maximum_pending.saturating_sub(self.pending.len()) {
            return Err(Error::new(ErrorCode::CapacityExceeded));
        }
        self.pending.extend_from_slice(bytes);
        let mut output = Vec::new();
        let mut consumed = 0;
        while self.pending.len() - consumed >= PROTOCOL_HEADER_BYTES {
            let header = &self.pending[consumed..consumed + PROTOCOL_HEADER_BYTES];
            let length = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
            if length == 0 || length > self.maximum_payload_bytes {
                return Err(Error::new(ErrorCode::InvalidLength));
            }
            let framed = PROTOCOL_HEADER_BYTES + length;
            if self.pending.len() - consumed < framed {
                break;
            }
            let payload = &self.pending[consumed + PROTOCOL_HEADER_BYTES..consumed + framed];
            let expected = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
            if crc32c::crc32c(payload) != expected {
                return Err(Error::new(ErrorCode::ChecksumMismatch));
            }
            output.push(payload.to_vec());
            consumed += framed;
        }
        if consumed != 0 {
            self.pending.drain(..consumed);
        }
        Ok(output)
    }
}
