use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::apply::{ApplyLoop, ApplyRequest, Storage, StorageCommit};
use hm_ledger::frame::{EventKind, FRAME_HEADER_SIZE, Frame};
use hm_ledger::keyring::KeyHierarchy;
use hm_ledger::segment::{AppendRequest, SegmentLog, SegmentLogOptions, WriteBackend};
use hm_sim::env::{SimClock, SimEntropy, SimulatedStorage};
use hm_sim::fault::{FaultPlan, next_random};
use std::sync::Mutex;

const ACTOR: ActorId = ActorId::new(17);

fn workload(count: usize) -> Vec<Vec<u8>> {
    let mut payloads = Vec::with_capacity(count);
    let mut random = 0x45dc_f13a_998e_f21b;
    for _ in 0..count {
        let length = usize::try_from(next_random(&mut random) % 193 + 1).expect("payload length");
        let mut payload = vec![0; length];
        for byte in &mut payload {
            *byte = next_random(&mut random).to_le_bytes()[0];
        }
        payloads.push(payload);
    }
    payloads
}

fn conversation(index: usize) -> ConversationId {
    ConversationId::new(std::array::from_fn(|byte| {
        (index * 37 + byte * 11).to_le_bytes()[0]
    }))
}

fn requests(payloads: &[Vec<u8>], start: usize, count: usize) -> Vec<ApplyRequest> {
    (start..start + count)
        .map(|index| ApplyRequest {
            kind: EventKind::try_from(u8::try_from(index % 21 + 1).expect("event kind"))
                .expect("valid event kind"),
            conversation: conversation(index % 29),
            plaintext_payload: payloads[index].clone(),
        })
        .collect()
}

fn baseline(payloads: &[Vec<u8>]) -> (Vec<u8>, Vec<u8>) {
    let mut storage = SimulatedStorage::new();
    let state = {
        let mut clock = SimClock::default();
        let mut apply = ApplyLoop::boot(&mut clock, &mut storage, ACTOR, payloads.len())
            .expect("baseline boot");
        apply
            .apply_batch(&requests(payloads, 0, payloads.len()))
            .expect("baseline apply");
        apply.state().canonical_bytes()
    };
    (state, storage.durable_bytes().to_vec())
}

#[test]
fn donor_2048_fault_schedules_converge_to_identical_canonical_bytes() {
    const WORKLOAD_SIZE: usize = 32;
    const SCHEDULES: usize = 2_048;
    let payloads = workload(WORKLOAD_SIZE);
    let (baseline_state, baseline_log) = baseline(&payloads);

    for schedule in 0..SCHEDULES {
        let mut storage = SimulatedStorage::new();
        let mut random = 0x6a09_e667_f3bc_c909 ^ schedule as u64;
        let mut iterations = 0;
        loop {
            storage.set_fault_plan(FaultPlan::default());
            let durable_count = storage.recover().expect("scheduled recovery").len();
            if durable_count == payloads.len() {
                break;
            }
            iterations += 1;
            assert!(
                iterations <= WORKLOAD_SIZE * 12,
                "schedule made no progress"
            );
            let batch_size = usize::try_from(next_random(&mut random) % 8 + 1)
                .expect("batch size")
                .min(payloads.len() - durable_count);
            let mut fault = FaultPlan {
                maximum_write_bytes: usize::try_from(next_random(&mut random) % 31 + 1)
                    .expect("write size"),
                maximum_read_bytes: usize::try_from(next_random(&mut random) % 17 + 1)
                    .expect("read size"),
                reverse_write_completion: next_random(&mut random) & 1 != 0,
                ..FaultPlan::default()
            };
            match iterations % 7 {
                1 => {
                    fault.torn_after_bytes =
                        usize::try_from(next_random(&mut random) % 41 + 1).expect("tear offset");
                }
                2 => fault.fsync_lies = true,
                3 => {
                    fault.kill_after_durable_lsn = u64::try_from(durable_count + 1)
                        .expect("durable count")
                        + next_random(&mut random) % u64::try_from(batch_size).expect("batch size");
                }
                _ => {}
            }
            storage.set_fault_plan(fault);
            let mut clock = SimClock::new(
                1_000_000 + i64::try_from(durable_count).expect("clock offset") * 1_000,
            );
            let result = {
                let mut apply =
                    ApplyLoop::boot(&mut clock, &mut storage, ACTOR, 8).expect("scheduled boot");
                apply.apply_batch(&requests(&payloads, durable_count, batch_size))
            };
            assert!(
                result.is_ok()
                    || result.as_ref().expect_err("fault error").code == ErrorCode::ProcessKilled,
                "unexpected apply error: {result:?}"
            );
            storage.crash();
        }

        storage.set_fault_plan(FaultPlan::default());
        let final_state = {
            let mut clock = SimClock::default();
            ApplyLoop::boot(&mut clock, &mut storage, ACTOR, 8)
                .expect("final boot")
                .state()
                .canonical_bytes()
        };
        assert_eq!(
            final_state, baseline_state,
            "state diverged at schedule {schedule}"
        );
        assert_eq!(
            storage.durable_bytes(),
            baseline_log,
            "log diverged at schedule {schedule}"
        );
    }
}

#[test]
fn donor_kill_at_every_lsn_recovers_exactly() {
    const COUNT: usize = 64;
    let payloads = workload(COUNT);
    let (baseline_state, baseline_log) = baseline(&payloads);
    for kill in 1..=COUNT {
        let mut storage = SimulatedStorage::new();
        for index in 0..COUNT {
            storage.set_fault_plan(FaultPlan::default());
            let durable_count = storage.recover().expect("kill recovery").len();
            let mut fault = FaultPlan::default();
            if index + 1 == kill {
                fault.kill_after_durable_lsn = u64::try_from(kill).expect("kill LSN");
            }
            storage.set_fault_plan(fault);
            let mut clock = SimClock::new(
                1_000_000 + i64::try_from(durable_count).expect("clock offset") * 1_000,
            );
            let result = {
                let mut apply =
                    ApplyLoop::boot(&mut clock, &mut storage, ACTOR, 1).expect("kill boot");
                apply.apply_batch(&requests(&payloads, index, 1))
            };
            assert!(
                result.is_ok()
                    || result.as_ref().expect_err("kill error").code == ErrorCode::ProcessKilled
            );
            storage.crash();
        }
        storage.set_fault_plan(FaultPlan::default());
        let final_state = {
            let mut clock = SimClock::default();
            ApplyLoop::boot(&mut clock, &mut storage, ACTOR, 1)
                .expect("restart")
                .state()
                .canonical_bytes()
        };
        assert_eq!(final_state, baseline_state, "state diverged at LSN {kill}");
        assert_eq!(
            storage.durable_bytes(),
            baseline_log,
            "log diverged at LSN {kill}"
        );
    }
}

#[test]
fn donor_corruption_and_writer_ownership_are_rejected() {
    let payloads = workload(3);
    let mut storage = SimulatedStorage::new();
    let mut clock = SimClock::default();
    {
        let apply = Mutex::new(
            ApplyLoop::boot(&mut clock, &mut storage, ACTOR, 3).expect("ownership boot"),
        );
        let first = requests(&payloads, 0, 1);
        std::thread::scope(|scope| {
            let intruder = scope.spawn(|| {
                apply
                    .lock()
                    .expect("apply lock")
                    .apply_batch(&first)
                    .expect_err("second writer")
                    .code
            });
            assert_eq!(
                intruder.join().expect("intruder thread"),
                ErrorCode::WriterViolation
            );
        });
        apply
            .lock()
            .expect("owner lock")
            .apply_batch(&requests(&payloads, 0, 3))
            .expect("owner writer");
    }
    storage
        .corrupt_durable_byte(FRAME_HEADER_SIZE, 0x80)
        .expect("inject corruption");
    assert_eq!(
        storage.recover().expect_err("interior corruption").code,
        ErrorCode::InteriorCorruption
    );
}

struct RealStorage {
    log: SegmentLog,
    keys: KeyHierarchy,
    entropy: SimEntropy,
}

impl Storage for RealStorage {
    fn append(&mut self, frames: &[Frame]) -> Result<StorageCommit, hm_core::Error> {
        let requests: Vec<_> = frames
            .iter()
            .map(|frame| AppendRequest {
                kind: frame.header.kind,
                wall_timestamp_ns: frame.header.wall_timestamp_ns,
                conversation: frame.header.conversation,
                sealed_payload: self
                    .keys
                    .seal(&frame.header, &frame.sealed_payload, &mut self.entropy)
                    .expect("seal real-storage frame"),
            })
            .collect();
        let result = self.log.append_batch(&requests)?;
        Ok(StorageCommit {
            first_lsn: result.first_lsn,
            last_lsn: result.last_lsn,
        })
    }

    fn recover(&mut self) -> Result<Vec<Frame>, hm_core::Error> {
        self.read_from(LSN::new(1), usize::MAX)
    }

    fn read_from(
        &mut self,
        first_lsn: LSN,
        maximum_frames: usize,
    ) -> Result<Vec<Frame>, hm_core::Error> {
        self.log
            .read_from(first_lsn, maximum_frames)?
            .into_iter()
            .map(|mut frame| {
                frame.sealed_payload = self.keys.unseal(&frame.header, &frame.sealed_payload)?;
                Ok(frame)
            })
            .collect()
    }
}

fn real_options(maximum_io_bytes: usize) -> SegmentLogOptions {
    SegmentLogOptions {
        maximum_io_bytes,
        backend: WriteBackend::Pwrite,
        ..SegmentLogOptions::default()
    }
}

fn real_storage(path: &std::path::Path, maximum_io_bytes: usize) -> RealStorage {
    let mut entropy = SimEntropy::new(7);
    let keys =
        KeyHierarchy::open_or_create(path, ACTOR, [0x52; 16], &[0xa7; 32], &mut entropy, true)
            .expect("real key hierarchy");
    RealStorage {
        log: SegmentLog::open(path, ACTOR, real_options(maximum_io_bytes))
            .expect("real segment storage"),
        keys,
        entropy,
    }
}

#[test]
fn donor_real_segment_storage_restarts_to_identical_state() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let payloads = workload(37);
    let expected = {
        let mut storage = real_storage(temporary.path(), 5);
        let mut clock = SimClock::default();
        let mut apply = ApplyLoop::boot(&mut clock, &mut storage, ACTOR, 37).expect("real boot");
        apply
            .apply_batch(&requests(&payloads, 0, payloads.len()))
            .expect("real apply");
        apply.state().canonical_bytes()
    };
    let actual = {
        let mut storage = real_storage(temporary.path(), 3);
        let mut clock = SimClock::default();
        ApplyLoop::boot(&mut clock, &mut storage, ACTOR, 37)
            .expect("restart boot")
            .state()
            .canonical_bytes()
    };
    assert_eq!(actual, expected);
}
