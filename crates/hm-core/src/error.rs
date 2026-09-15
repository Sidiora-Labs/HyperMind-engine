use crate::LSN;
use core::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ErrorCode {
    InvalidArgument = 0,
    InvalidLength = 1,
    InvalidKind = 2,
    SequenceViolation = 3,
    OpenFailed = 4,
    ReadFailed = 5,
    WriteFailed = 6,
    SyncFailed = 7,
    CloseFailed = 8,
    Truncated = 9,
    ChecksumMismatch = 10,
    InteriorCorruption = 11,
    ManifestCorrupt = 12,
    BackendUnavailable = 13,
    SegmentFull = 14,
    ProofInvalid = 15,
    SignatureInvalid = 16,
    CheckpointMismatch = 17,
    AlreadyExists = 18,
    WriterViolation = 19,
    ProcessKilled = 20,
    DurabilityFailure = 21,
    ProjectionCheckpoint = 22,
    MapFull = 23,
    CryptoAuthentication = 24,
    KeyDestroyed = 25,
    LegacyPlaintext = 26,
    SchemaInvalid = 27,
    SchemaVersion = 28,
    ForbiddenKind = 29,
    OrderingViolation = 30,
    BeliefWriteGate = 31,
    BeliefIdConflict = 32,
    BeliefNotFound = 33,
    BeliefCorrupt = 34,
    NegativeExistenceUncorroborated = 35,
    EntityIndexCorrupt = 36,
    VectorIndexCorrupt = 37,
    LexicalIndexCorrupt = 38,
    TemporalLadderCorrupt = 39,
    IntentFrameCorrupt = 40,
    LoopNotFound = 41,
    WorkLedgerCorrupt = 42,
    WorkItemNotFound = 43,
    InvariantViolation = 44,
    ProtocolInvalid = 45,
    ProtocolVersion = 46,
    CapabilityDenied = 47,
    CapacityExceeded = 48,
    OperationUnavailable = 49,
    IdempotencyConflict = 50,
    ProtectedTypeWrite = 51,
    CitationInvalid = 52,
    DeadlineMissed = 53,
    Tripwire = 54,
}

impl ErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "kInvalidArgument",
            Self::InvalidLength => "kInvalidLength",
            Self::InvalidKind => "kInvalidKind",
            Self::SequenceViolation => "kSequenceViolation",
            Self::OpenFailed => "kOpenFailed",
            Self::ReadFailed => "kReadFailed",
            Self::WriteFailed => "kWriteFailed",
            Self::SyncFailed => "kSyncFailed",
            Self::CloseFailed => "kCloseFailed",
            Self::Truncated => "kTruncated",
            Self::ChecksumMismatch => "kChecksumMismatch",
            Self::InteriorCorruption => "kInteriorCorruption",
            Self::ManifestCorrupt => "kManifestCorrupt",
            Self::BackendUnavailable => "kBackendUnavailable",
            Self::SegmentFull => "kSegmentFull",
            Self::ProofInvalid => "kProofInvalid",
            Self::SignatureInvalid => "kSignatureInvalid",
            Self::CheckpointMismatch => "kCheckpointMismatch",
            Self::AlreadyExists => "kAlreadyExists",
            Self::WriterViolation => "kWriterViolation",
            Self::ProcessKilled => "kProcessKilled",
            Self::DurabilityFailure => "kDurabilityFailure",
            Self::ProjectionCheckpoint => "kProjectionCheckpoint",
            Self::MapFull => "kMapFull",
            Self::CryptoAuthentication => "kCryptoAuthentication",
            Self::KeyDestroyed => "kKeyDestroyed",
            Self::LegacyPlaintext => "kLegacyPlaintext",
            Self::SchemaInvalid => "kSchemaInvalid",
            Self::SchemaVersion => "kSchemaVersion",
            Self::ForbiddenKind => "kForbiddenKind",
            Self::OrderingViolation => "kOrderingViolation",
            Self::BeliefWriteGate => "kBeliefWriteGate",
            Self::BeliefIdConflict => "kBeliefIdConflict",
            Self::BeliefNotFound => "kBeliefNotFound",
            Self::BeliefCorrupt => "kBeliefCorrupt",
            Self::NegativeExistenceUncorroborated => "kNegativeExistenceUncorroborated",
            Self::EntityIndexCorrupt => "kEntityIndexCorrupt",
            Self::VectorIndexCorrupt => "kVectorIndexCorrupt",
            Self::LexicalIndexCorrupt => "kLexicalIndexCorrupt",
            Self::TemporalLadderCorrupt => "kTemporalLadderCorrupt",
            Self::IntentFrameCorrupt => "kIntentFrameCorrupt",
            Self::LoopNotFound => "kLoopNotFound",
            Self::WorkLedgerCorrupt => "kWorkLedgerCorrupt",
            Self::WorkItemNotFound => "kWorkItemNotFound",
            Self::InvariantViolation => "kInvariantViolation",
            Self::ProtocolInvalid => "kProtocolInvalid",
            Self::ProtocolVersion => "kProtocolVersion",
            Self::CapabilityDenied => "kCapabilityDenied",
            Self::CapacityExceeded => "kCapacityExceeded",
            Self::OperationUnavailable => "kOperationUnavailable",
            Self::IdempotencyConflict => "kIdempotencyConflict",
            Self::ProtectedTypeWrite => "kProtectedTypeWrite",
            Self::CitationInvalid => "kCitationInvalid",
            Self::DeadlineMissed => "kDeadlineMissed",
            Self::Tripwire => "kTripwire",
        }
    }
}

impl TryFrom<u8> for ErrorCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        const CODES: [ErrorCode; 55] = [
            ErrorCode::InvalidArgument,
            ErrorCode::InvalidLength,
            ErrorCode::InvalidKind,
            ErrorCode::SequenceViolation,
            ErrorCode::OpenFailed,
            ErrorCode::ReadFailed,
            ErrorCode::WriteFailed,
            ErrorCode::SyncFailed,
            ErrorCode::CloseFailed,
            ErrorCode::Truncated,
            ErrorCode::ChecksumMismatch,
            ErrorCode::InteriorCorruption,
            ErrorCode::ManifestCorrupt,
            ErrorCode::BackendUnavailable,
            ErrorCode::SegmentFull,
            ErrorCode::ProofInvalid,
            ErrorCode::SignatureInvalid,
            ErrorCode::CheckpointMismatch,
            ErrorCode::AlreadyExists,
            ErrorCode::WriterViolation,
            ErrorCode::ProcessKilled,
            ErrorCode::DurabilityFailure,
            ErrorCode::ProjectionCheckpoint,
            ErrorCode::MapFull,
            ErrorCode::CryptoAuthentication,
            ErrorCode::KeyDestroyed,
            ErrorCode::LegacyPlaintext,
            ErrorCode::SchemaInvalid,
            ErrorCode::SchemaVersion,
            ErrorCode::ForbiddenKind,
            ErrorCode::OrderingViolation,
            ErrorCode::BeliefWriteGate,
            ErrorCode::BeliefIdConflict,
            ErrorCode::BeliefNotFound,
            ErrorCode::BeliefCorrupt,
            ErrorCode::NegativeExistenceUncorroborated,
            ErrorCode::EntityIndexCorrupt,
            ErrorCode::VectorIndexCorrupt,
            ErrorCode::LexicalIndexCorrupt,
            ErrorCode::TemporalLadderCorrupt,
            ErrorCode::IntentFrameCorrupt,
            ErrorCode::LoopNotFound,
            ErrorCode::WorkLedgerCorrupt,
            ErrorCode::WorkItemNotFound,
            ErrorCode::InvariantViolation,
            ErrorCode::ProtocolInvalid,
            ErrorCode::ProtocolVersion,
            ErrorCode::CapabilityDenied,
            ErrorCode::CapacityExceeded,
            ErrorCode::OperationUnavailable,
            ErrorCode::IdempotencyConflict,
            ErrorCode::ProtectedTypeWrite,
            ErrorCode::CitationInvalid,
            ErrorCode::DeadlineMissed,
            ErrorCode::Tripwire,
        ];
        CODES.get(usize::from(value)).copied().ok_or(())
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Error {
    pub code: ErrorCode,
    pub system_error: i32,
    pub lsn: LSN,
    pub offset: u64,
}

impl Error {
    #[must_use]
    pub const fn new(code: ErrorCode) -> Self {
        Self {
            code,
            system_error: 0,
            lsn: LSN::new(0),
            offset: 0,
        }
    }

    #[must_use]
    pub const fn with_system_error(mut self, system_error: i32) -> Self {
        self.system_error = system_error;
        self
    }

    #[must_use]
    pub const fn at_lsn(mut self, lsn: LSN) -> Self {
        self.lsn = lsn;
        self
    }

    #[must_use]
    pub const fn at_offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} (system_error={}, lsn={}, offset={})",
            self.code, self.system_error, self.lsn, self.offset
        )
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_numeric_codes_are_stable_and_extensions_are_appended() {
        assert_eq!(ErrorCode::InvalidArgument as u8, 0);
        assert_eq!(ErrorCode::IdempotencyConflict as u8, 50);
        assert_eq!(ErrorCode::ProtectedTypeWrite as u8, 51);
        assert_eq!(ErrorCode::CitationInvalid as u8, 52);
        assert_eq!(ErrorCode::DeadlineMissed as u8, 53);
        assert_eq!(ErrorCode::Tripwire as u8, 54);
    }

    #[test]
    fn every_numeric_code_round_trips() {
        for raw in 0_u8..=54 {
            let code = ErrorCode::try_from(raw).expect("known error code");
            assert_eq!(code as u8, raw);
            assert!(code.as_str().starts_with('k'));
        }
        assert_eq!(ErrorCode::try_from(55), Err(()));
    }

    #[test]
    fn error_carries_operational_location() {
        let error = Error::new(ErrorCode::ReadFailed)
            .with_system_error(5)
            .at_lsn(LSN::new(9))
            .at_offset(43);
        assert_eq!(error.system_error, 5);
        assert_eq!(error.lsn, LSN::new(9));
        assert_eq!(error.offset, 43);
    }
}
