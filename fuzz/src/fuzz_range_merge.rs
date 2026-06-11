//! Fuzz target: `ComparedRange::union` must not panic for any combination of
//! optional `(u32, u32)` start/end pairs.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }
    fn read_opt(b: &[u8]) -> Option<(u32, u32)> {
        if b[0] == 0 { return None; }
        let r = u32::from_le_bytes([b[1], b[2], b[3], b[4]]);
        let c = u32::from_le_bytes([b[5], b[6], b[7], b[8]]);
        Some((r.max(1), c.max(1)))
    }
    let old_start = read_opt(&data[0..]);
    let old_end   = read_opt(&data[8..]);
    let new_start = if data.len() >= 24 { read_opt(&data[16..]) } else { None };
    let new_end   = if data.len() >= 32 { read_opt(&data[24..]) } else { None };
    // Must not panic
    let _ = sheets_diff::ComparedRange::union(old_start, old_end, new_start, new_end);
});
