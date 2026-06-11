//! Fuzz target: `DiffOptionsBuilder::build` must not panic — it may return
//! `Err(InvalidOptions)` for bad combinations, but must never panic.
#![no_main]
use libfuzzer_sys::fuzz_target;
use sheets_diff::{DiffOptions, FormulaCompareMode, SheetMatchingMode};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    let formula_mode = match data[0] % 4 {
        0 => FormulaCompareMode::RawText,
        1 => FormulaCompareMode::NormalizedText,
        2 => FormulaCompareMode::RawAndNormalized,
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
