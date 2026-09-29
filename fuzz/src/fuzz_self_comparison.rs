//! Fuzz target: a workbook compared with itself must produce zero cell diffs, in every alignment
//! mode, with no sheet change other than `Unchanged` (M9 unit 01).
//!
//! `fuzz_open_xlsx_bytes` proves the crate does not panic on arbitrary input, but "did not panic" is
//! satisfied by a wrong answer too — f123, f130 and f131 all lived past the point that target
//! reaches (it has never once opened a real workbook; every seed failed at `not an xlsx file`).
//! **This target reaches the reader on every input that is a plausible `.xlsx` candidate** (the
//! rest of the fuzz input, after a 3-byte header — see `self_seed.rs`), because there is nothing to
//! split: one seed is one workbook. When [`sheets_diff::compare_bytes_with_options`] returns `Ok`,
//! the identity oracle applies: read → normalise → match → align → compare all ran, on both sides of
//! the same bytes, and the only correct answer is "no difference".
//!
//! **This is a real oracle, not "did not panic".** With f130's original defect reintroduced —
//! keyless rows dropped before pairing, `src/align.rs` — this target's own corpus (a workbook with a
//! blank key every twentieth row, self-compared under `RowKey`) fails it; see the review request's
//! failing-first evidence. `RowSignature` does not move: it has no keyless path, so the invariant is
//! load-bearing for `RowKey` here specifically, and a target that only ever selected `RowSignature`
//! would have proved nothing about that class of defect.
//!
//! The header also drives `Limits`, bounded so no input this target can construct allocates more
//! than `Limits::hardened()` already allows on its own — see `self_seed.rs`.

#![no_main]
use libfuzzer_sys::fuzz_target;

include!("self_seed.rs");

fuzz_target!(|data: &[u8]| {
    let (header, workbook) = split_self_seed(data);
    let opts = opts_from_header(header);
    if let Ok(diff) = sheets_diff::compare_bytes_with_options(workbook, workbook, opts) {
        assert_self_comparison_is_empty(&diff);
    }
    // An `Err` here (not an xlsx file, a limit exceeded, ...) is not this target's oracle failing —
    // it means the identity property was never exercised on this input. Coverage measures how often
    // that happens; see the review request.
});
