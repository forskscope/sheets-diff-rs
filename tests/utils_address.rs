// RFC-003, RFC-006: A1 addressing unit tests.
//
// Exercises `col_to_label` and `cell_pos_to_address` across the full Excel
// column range, including the historical u8 overflow boundary (column 256).

use sheets_diff::core::utils::{cell_pos_to_address, col_to_label};

/// Full column-label table from RFC-003.
#[test]
fn col_labels_cover_excel_width() {
    let cases: &[(usize, &str)] = &[
        (1, "A"),
        (2, "B"),
        (26, "Z"),
        (27, "AA"),
        (52, "AZ"),
        (53, "BA"),
        (255, "IU"),
        (256, "IV"),
        (257, "IW"),
        (702, "ZZ"),
        (703, "AAA"),
        (704, "AAB"),
        (16384, "XFD"),
    ];

    for &(col, expected) in cases {
        assert_eq!(col_to_label(col), expected, "col_to_label({col})");
    }
}

/// Direct address tests including maximum Excel address.
#[test]
fn cell_addresses_are_correct() {
    assert_eq!(cell_pos_to_address(1, 1), "A1");
    assert_eq!(cell_pos_to_address(25, 27), "AA25");
    assert_eq!(cell_pos_to_address(1048576, 16384), "XFD1048576");
}

/// Regression guard against the old `col as u8` truncation / underflow path.
#[test]
fn column_256_and_257_are_correct() {
    assert_eq!(cell_pos_to_address(1, 256), "IV1");
    assert_eq!(cell_pos_to_address(1, 257), "IW1");
}

/// Sorting by numeric (row, col) must put A1 < A2 < A10, not A1 < A10 < A2.
#[test]
fn cell_sort_order_is_numeric() {
    use sheets_diff::core::diff::{CellDiff, CellDiffKind};

    let mut cells: Vec<CellDiff> = vec![
        CellDiff {
            row: 10,
            col: 1,
            addr: "A10".into(),
            kind: CellDiffKind::Value,
            old: Some("x".into()),
            new: None,
        },
        CellDiff {
            row: 2,
            col: 1,
            addr: "A2".into(),
            kind: CellDiffKind::Value,
            old: Some("y".into()),
            new: None,
        },
        CellDiff {
            row: 1,
            col: 1,
            addr: "A1".into(),
            kind: CellDiffKind::Value,
            old: Some("z".into()),
            new: None,
        },
    ];

    cells.sort_by(|a, b| {
        a.row
            .cmp(&b.row)
            .then_with(|| a.col.cmp(&b.col))
            .then_with(|| a.kind.cmp(&b.kind))
    });

    let rows: Vec<usize> = cells.iter().map(|c| c.row).collect();
    assert_eq!(rows, vec![1, 2, 10]);
}
