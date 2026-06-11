//! Cell addressing: A1 label encoding, 1-based coordinates, Excel bounds.
//!
//! Excel limits: rows 1–1_048_576, columns 1–16_384 (A–XFD).

use std::fmt;

#[cfg(feature = "serde")]
use serde::Serialize;

// ---------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------

/// Maximum valid 1-based row index in an Excel .xlsx workbook.
pub const MAX_ROW: u32 = 1_048_576;
/// Maximum valid 1-based column index in an Excel .xlsx workbook.
pub const MAX_COL: u32 = 16_384;
/// The A1 label of the last valid Excel column (column 16384).
pub const MAX_COL_LABEL: &str = "XFD";

// ---------------------------------------------------------------------------
// CellAddress
// ---------------------------------------------------------------------------

/// The address of a single cell, carrying both numeric coordinates and the A1
/// label.
///
/// - `row` and `col` are **1-based**.
/// - `a1` is the canonical Excel A1 string (e.g. `"XFD1048576"`).
/// - Sorting must use `(row, col)`, never lexicographic A1 order.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CellAddress {
    pub row: u32,
    pub col: u32,
    pub a1: String,
}

impl CellAddress {
    /// Construct a `CellAddress` from 1-based `(row, col)`.
    ///
    /// Returns `None` when `row` or `col` is zero or exceeds the Excel limit.
    pub fn new(row: u32, col: u32) -> Option<Self> {
        if row == 0 || row > MAX_ROW || col == 0 || col > MAX_COL {
            return None;
        }
        let a1 = format!("{}{}", col_to_label(col), row);
        Some(Self { row, col, a1 })
    }

    /// Construct without bounds checking.  Caller asserts validity.
    #[inline]
    pub(crate) fn new_unchecked(row: u32, col: u32) -> Self {
        Self {
            a1: format!("{}{}", col_to_label(col), row),
            row,
            col,
        }
    }
}

impl fmt::Display for CellAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.a1)
    }
}

// ---------------------------------------------------------------------------
// ComparedRange
// ---------------------------------------------------------------------------

/// The bounding rectangle that was compared for a sheet pair.
///
/// `None` on either side means that side was empty (no used range).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(PartialEq)]
pub struct ComparedRange {
    /// Inclusive top-left, 1-based.
    pub start: Option<(u32, u32)>,
    /// Inclusive bottom-right, 1-based.
    pub end: Option<(u32, u32)>,
}

impl ComparedRange {
    pub fn empty() -> Self {
        Self { start: None, end: None }
    }

    /// Expand to contain both sides' used ranges.
    pub(crate) fn union(
        old_start: Option<(u32, u32)>,
        old_end: Option<(u32, u32)>,
        new_start: Option<(u32, u32)>,
        new_end: Option<(u32, u32)>,
    ) -> Self {
        let start = match (old_start, new_start) {
            (None, None) => None,
            (Some(a), None) | (None, Some(a)) => Some(a),
            (Some((ar, ac)), Some((br, bc))) => Some((ar.min(br), ac.min(bc))),
        };
        let end = match (old_end, new_end) {
            (None, None) => None,
            (Some(a), None) | (None, Some(a)) => Some(a),
            (Some((ar, ac)), Some((br, bc))) => Some((ar.max(br), ac.max(bc))),
        };
        Self { start, end }
    }
}

// ---------------------------------------------------------------------------
// A1 encoding helpers
// ---------------------------------------------------------------------------

/// Convert a 1-based column index to an Excel column label (`1` → `"A"`,
/// `16384` → `"XFD"`).
///
/// Panics (debug) if `col == 0`.
pub fn col_to_label(mut col: u32) -> String {
    debug_assert!(col > 0, "col must be 1-based");
    let mut bytes = Vec::with_capacity(3);
    while col > 0 {
        let rem = (col - 1) % 26;
        bytes.push(b'A' + rem as u8);
        col = (col - 1) / 26;
    }
    bytes.reverse();
    // Safety: only ASCII uppercase letters were pushed.
    unsafe { String::from_utf8_unchecked(bytes) }
}

/// Convert a 1-based `(row, col)` pair to an Excel A1 address string.
pub fn cell_pos_to_a1(row: u32, col: u32) -> String {
    format!("{}{}", col_to_label(col), row)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn col_label_single_letters() {
        assert_eq!(col_to_label(1), "A");
        assert_eq!(col_to_label(26), "Z");
    }

    #[test]
    fn col_label_double_letters() {
        assert_eq!(col_to_label(27), "AA");
        assert_eq!(col_to_label(52), "AZ");
        assert_eq!(col_to_label(53), "BA");
        assert_eq!(col_to_label(702), "ZZ");
    }

    #[test]
    fn col_label_triple_letters() {
        assert_eq!(col_to_label(703), "AAA");
        assert_eq!(col_to_label(16_384), MAX_COL_LABEL);
    }

    #[test]
    fn cell_address_new_valid() {
        let addr = CellAddress::new(1, 1).unwrap();
        assert_eq!(addr.a1, "A1");
        assert_eq!(addr.row, 1);
        assert_eq!(addr.col, 1);

        let last = CellAddress::new(MAX_ROW, MAX_COL).unwrap();
        assert_eq!(last.a1, "XFD1048576");
    }

    #[test]
    fn cell_address_new_out_of_bounds() {
        assert!(CellAddress::new(0, 1).is_none());
        assert!(CellAddress::new(1, 0).is_none());
        assert!(CellAddress::new(MAX_ROW + 1, 1).is_none());
        assert!(CellAddress::new(1, MAX_COL + 1).is_none());
    }

    #[test]
    fn sort_order_is_row_col_not_a1_lexicographic() {
        let a2 = CellAddress::new(2, 1).unwrap();
        let a10 = CellAddress::new(10, 1).unwrap();
        assert!(a2 < a10, "A10 must sort after A2 (numeric row, not lex)");
    }
}
