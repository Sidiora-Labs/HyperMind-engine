#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut parser = hm_serve::protocol::FrameParser::default();
    if let Ok(frames) = parser.push(data) {
        for frame in frames {
            let _ = hm_schema::protocol::verify_wire_envelope(&frame);
        }
    }
});
