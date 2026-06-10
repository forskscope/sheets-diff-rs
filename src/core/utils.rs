/// Returns sheets whose names appear in both lists, preserving old workbook order.
pub fn filter_same_name_sheets(old_sheets: &[String], new_sheets: &[String]) -> Vec<String> {
    old_sheets
        .iter()
        .filter(|s| new_sheets.contains(s))
        .cloned()
        .collect()
}

/// Returns the bounding rectangle that covers both ranges.
///
/// Returns `(start_row, start_col, end_row_exclusive, end_col_exclusive)`.
pub fn diff_range(
    old_start: Option<(u32, u32)>,
    new_start: Option<(u32, u32)>,
    old_end: Option<(u32, u32)>,
    new_end: Option<(u32, u32)>,
) -> (u32, u32, u32, u32) {
    let (old_start_row, old_start_col) = old_start.unwrap_or((u32::MAX, u32::MAX));
    let (new_start_row, new_start_col) = new_start.unwrap_or((u32::MAX, u32::MAX));
    let (old_end_row, old_end_col) = old_end.unwrap_or((u32::MIN, u32::MIN));
    let (new_end_row, new_end_col) = new_end.unwrap_or((u32::MIN, u32::MIN));

    let start_row = old_start_row.min(new_start_row);
    let start_col = old_start_col.min(new_start_col);
    let end_row = old_end_row.max(new_end_row);
    let end_col = old_end_col.max(new_end_col);

    (start_row, start_col, end_row + 1, end_col + 1)
}

/// Converts a 1-based column index to an Excel column label (e.g. 1 → "A", 27 → "AA").
///
/// Excel's full column range is `1..=16384`, where column 16384 is `XFD`.
///
/// # Panics
///
/// Panics in debug builds if `col == 0`. A zero column is an internal programming
/// error; calamine returns 0-based coordinates and every call site must add 1 before
/// calling this function.
pub fn col_to_label(mut col: usize) -> String {
    assert!(col > 0, "Excel column index is 1-based; col == 0 is invalid");

    let mut bytes = Vec::new();
    while col > 0 {
        let rem = (col - 1) % 26;
        bytes.push(b'A' + rem as u8);
        col = (col - 1) / 26;
    }
    bytes.reverse();
    // Safety: only ASCII uppercase letters are pushed.
    String::from_utf8(bytes).expect("only ASCII uppercase letters are generated")
}

/// Converts a 1-based `(row, col)` pair to an Excel A1 address string.
///
/// For example: `(1, 1)` → `"A1"`, `(1, 16384)` → `"XFD1"`.
pub fn cell_pos_to_address(row: usize, col: usize) -> String {
    format!("{}{}", col_to_label(col), row)
}
