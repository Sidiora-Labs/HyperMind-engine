#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FaultPlan {
    pub maximum_write_bytes: usize,
    pub maximum_read_bytes: usize,
    pub torn_after_bytes: usize,
    pub kill_after_durable_lsn: u64,
    pub reverse_write_completion: bool,
    pub fsync_lies: bool,
}

impl Default for FaultPlan {
    fn default() -> Self {
        Self {
            maximum_write_bytes: usize::MAX,
            maximum_read_bytes: usize::MAX,
            torn_after_bytes: usize::MAX,
            kill_after_durable_lsn: 0,
            reverse_write_completion: false,
            fsync_lies: false,
        }
    }
}

#[must_use]
pub fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
