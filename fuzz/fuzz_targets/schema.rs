#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for raw in 1..=22 {
        if let Ok(kind) = hm_schema::event::EventKind::try_from(raw) {
            let _ = hm_schema::event::verify_event(data, kind, hm_schema::event::Boundary::Import);
        }
    }
});
