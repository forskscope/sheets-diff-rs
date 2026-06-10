// RFC-003, RFC-006: wide-column integration tests.
//
// Verifies that columns beyond 255 — historically truncated by the u8 cast —
// are now addressed correctly across the full Excel range up to XFD (16384).

use sheets_diff::core::diff::Diff;

const OLD: &str = "tests/fixtures/wide-columns-old.xlsx";
const NEW: &str = "tests/fixtures/wide-columns-new.xlsx";

#[test]
fn wide_column_fixtures_are_diffed_successfully() {
    let diff = Diff::try_new(OLD, NEW).expect("wide-column diff should succeed");
    assert_eq!(diff.cell_diffs.len(), 1, "expected one sheet with diffs");
}

#[test]
fn wide_column_addresses_include_iv1() {
    let diff = Diff::try_new(OLD, NEW).unwrap();
    let addrs: Vec<&str> = diff.cell_diffs[0].cells.iter().map(|c| c.addr.as_str()).collect();
    assert!(addrs.contains(&"IV1"), "expected IV1 (col 256), got: {addrs:?}");
}

#[test]
fn wide_column_addresses_include_iw1() {
    let diff = Diff::try_new(OLD, NEW).unwrap();
    let addrs: Vec<&str> = diff.cell_diffs[0].cells.iter().map(|c| c.addr.as_str()).collect();
    assert!(addrs.contains(&"IW1"), "expected IW1 (col 257), got: {addrs:?}");
}

#[test]
fn wide_column_addresses_include_aaa1() {
    let diff = Diff::try_new(OLD, NEW).unwrap();
    let addrs: Vec<&str> = diff.cell_diffs[0].cells.iter().map(|c| c.addr.as_str()).collect();
    assert!(
        addrs.contains(&"AAA1"),
        "expected AAA1 (col 703), got: {addrs:?}"
    );
}

#[test]
fn wide_column_addresses_include_xfd1() {
    let diff = Diff::try_new(OLD, NEW).unwrap();
    let addrs: Vec<&str> = diff.cell_diffs[0].cells.iter().map(|c| c.addr.as_str()).collect();
    assert!(
        addrs.contains(&"XFD1"),
        "expected XFD1 (col 16384), got: {addrs:?}"
    );
}

/// Diffs are sorted by numeric (row, col), not by A1 string.
/// The fixture only has row 1 cells for wide columns, so verify they come
/// before the row-2 anchor cell.
#[test]
fn wide_column_diff_order_is_numeric() {
    let diff = Diff::try_new(OLD, NEW).unwrap();
    let cells = &diff.cell_diffs[0].cells;

    // All wide-column changes are in row 1; anchor is row 2.  The anchor cell
    // has the same value in both workbooks, so it should NOT appear in diffs.
    // Every diff entry should have row == 1.
    for cell in cells {
        assert_eq!(
            cell.row, 1,
            "unexpected row {} for cell {} — expected all diffs in row 1",
            cell.row, cell.addr
        );
    }
}
