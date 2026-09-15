#![no_main]

use hm_core::{ActorId, ConversationId, Error, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, FrameHeader};
use hm_ledger::keyring::{EntropySource, KeyHierarchy};
use libfuzzer_sys::fuzz_target;

struct Entropy(u8);

impl EntropySource for Entropy {
    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Error> {
        for byte in destination {
            *byte = self.0;
            self.0 = self.0.wrapping_add(1);
        }
        Ok(())
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(directory) = tempfile::tempdir() else {
        return;
    };
    let actor = ActorId::new(1);
    let mut entropy = Entropy(1);
    let Ok(keys) = KeyHierarchy::open_or_create(
        directory.path(),
        actor,
        [2; 16],
        &[3; 32],
        &mut entropy,
        true,
    ) else {
        return;
    };
    let header = FrameHeader {
        lsn: LSN::new(1),
        kind: EventKind::UserMsg,
        wall_timestamp_ns: UtcNanos::new(1),
        actor,
        conversation: ConversationId::new([4; 16]),
    };
    let _ = keys.unseal(&header, data);
});
