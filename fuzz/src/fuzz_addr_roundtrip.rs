//! Fuzz target: `col_to_label` must not panic for any in-bounds column value,
//! and must produce labels consistent with `CellAddress`.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let col = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if col == 0 || col > 16_384 {
        return; // out-of-Excel-bounds; documented as invalid
    }
    // Must not panic and must produce a non-empty label
    let label = sheets_diff::address::col_to_label(col);
    assert!(!label.is_empty());
    assert!(label.chars().all(|c| c.is_ascii_uppercase()));

    // CellAddress::new with the same col must agree
    if let Some(addr) = sheets_diff::CellAddress::new(1, col) {
        assert!(addr.a1.ends_with('1'));
        assert!(addr.a1.starts_with(&label));
    }
});
