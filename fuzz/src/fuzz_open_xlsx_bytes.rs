//! Fuzz target: `compare_bytes` must not panic on arbitrary input.
//!
//! Run with:  cargo fuzz run fuzz_open_xlsx_bytes
//!
//! The oracle is simple: any input is acceptable as long as the function
//! returns `Ok(_)` or `Err(_)` — panics are failures.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Split the fuzz input in half: first half = "old", second = "new".
    let mid = data.len() / 2;
    let old = &data[..mid];
    let new = &data[mid..];
    // Result is intentionally ignored — we only care that there is no panic.
    let _ = sheets_diff::compare_bytes(old, new);
});
