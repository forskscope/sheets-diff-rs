//! Fuzz target: `DiffOptionsBuilder::build` must not panic.
//!
//! Through 2.6.0 it could also return `Err(InvalidOptions)` for a combination
//! `validate()` rejected. As of 3.0.0 no combination is invalid — the settings
//! that could only fail were removed (RFC-037 §3.3) — so `build()` returns
//! `Ok` for every input this target can construct. It still must not panic,
//! which is what is being fuzzed, and `validate()` remains the seam a future
//! unusable option would use.
#![no_main]
use libfuzzer_sys::fuzz_target;
use sheets_diff::{DiffOptions, FormulaCompareMode, SheetMatchingMode};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let formula_mode = match data[0] % 2 {
        0 => FormulaCompareMode::RawText,
        _ => FormulaCompareMode::Ignore,
    };
    let sheet_mode = match data.get(1).copied().unwrap_or(0) % 3 {
        0 => SheetMatchingMode::ExactNameOnly,
        1 => SheetMatchingMode::ExactNameThenConservativeRename,
        _ => SheetMatchingMode::ExactNameThenIndex,
    };
    let max_sheets = if data.get(2).copied().unwrap_or(0) > 128 {
        Some(data.get(2).copied().unwrap_or(10) as u32)
    } else {
        None
    };
    // Must not panic regardless of combination
    let _ = DiffOptions::builder()
        .formula_compare(formula_mode)
        .sheet_matching(sheet_mode)
        .max_sheets(max_sheets.unwrap_or(u32::MAX))
        .build();
});
