//! Fuzz target: `compare_bytes` must not panic on arbitrary input.
//!
//! Run with:  cargo fuzz run fuzz_open_xlsx_bytes
//!
//! The oracle is simple: any input is acceptable as long as the function
//! returns `Ok(_)` or `Err(_)` — panics are failures.
//!
//! **Framing (M9 unit 01).** The input used to be split at `data.len() / 2`, which made a valid
//! seed nearly impossible to write (see `framing.rs`). It is now a length-prefixed split, shared
//! with the permanent guard and the corpus generator via `include!` so this file, `tests/` and
//! `examples/gen-fuzz-corpus.rs` can never decode a seed differently.

#![no_main]
use libfuzzer_sys::fuzz_target;

include!("framing.rs");

fuzz_target!(|data: &[u8]| {
    let (old, new) = split_old_new(data);
    // Result is intentionally ignored — we only care that there is no panic.
    let _ = sheets_diff::compare_bytes(old, new);
});
