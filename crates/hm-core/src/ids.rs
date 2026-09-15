use core::fmt;
use sha2::{Digest, Sha256};

const CONVERSATION_DOMAIN: &[u8] = b"neocortex-conversation-v1\0";

macro_rules! scalar_id {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name($inner);

        impl $name {
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $inner {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

scalar_id!(Lsn, u64);
scalar_id!(ActorId, u16);
scalar_id!(SchemaVersion, u16);
scalar_id!(UtcNanos, i64);

#[allow(clippy::upper_case_acronyms)]
pub type LSN = Lsn;

macro_rules! byte_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name([u8; 16]);

        impl $name {
            #[must_use]
            pub const fn new(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }

            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 16] {
                &self.0
            }

            #[must_use]
            pub const fn into_bytes(self) -> [u8; 16] {
                self.0
            }
        }

        impl From<[u8; 16]> for $name {
            fn from(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }
        }

        impl From<$name> for [u8; 16] {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }
    };
}

byte_id!(EntityId);
byte_id!(ConversationId);

impl ConversationId {
    #[must_use]
    pub fn derive(external_id: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(CONVERSATION_DOMAIN);
        hasher.update(external_id.as_bytes());
        let digest = hasher.finalize();
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        Self(bytes)
    }
}

impl From<&str> for ConversationId {
    fn from(external_id: &str) -> Self {
        Self::derive(external_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_id_matches_go_client_vectors() {
        assert_eq!(
            ConversationId::derive("conv-contract").into_bytes(),
            [
                0xb3, 0xcf, 0x38, 0xe8, 0xd2, 0x39, 0xd9, 0x60, 0x23, 0x15, 0x89, 0xfb, 0x74, 0x57,
                0x5f, 0x91,
            ]
        );
        assert_eq!(
            ConversationId::derive("").into_bytes(),
            [
                0xdd, 0xc4, 0xca, 0x0c, 0x88, 0xfa, 0xf6, 0x7d, 0xff, 0x74, 0x6f, 0x56, 0xe9, 0x2b,
                0xc8, 0xde,
            ]
        );
    }

    #[test]
    fn byte_ids_have_canonical_hex_display() {
        let id = EntityId::new([0xab; 16]);
        assert_eq!(id.to_string(), "abababababababababababababababab");
    }

    #[test]
    fn scalar_ids_preserve_values() {
        assert_eq!(LSN::new(42).get(), 42);
        assert_eq!(ActorId::new(7).get(), 7);
        assert_eq!(SchemaVersion::new(2).get(), 2);
        assert_eq!(UtcNanos::new(-1).get(), -1);
    }
}
