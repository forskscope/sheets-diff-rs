//! Framework-neutral GUI view adapters over `WorkbookDiff` (RFC-029).
//!
//! These types borrow from `WorkbookDiff` and allocate display strings only
//! on demand.  No GUI framework dependency is introduced.

use crate::address::CellAddress;
use crate::model::{
    CellChangeKind, CellDiff, SheetChange, Severity, WorkbookDiff,
};

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

/// Controls which change categories are visible in a `DiffView`.
#[derive(Clone, Debug)]
pub struct ViewFilter {
    pub include_values: bool,
    pub include_formulas: bool,
    /// Formatting diffs (always false until RFC-022 is implemented).
    pub include_formatting: bool,
    pub include_info_diagnostics: bool,
    /// If `Some`, only include changes from the listed sheet indices (0-based).
    pub sheets: Option<Vec<usize>>,
}

impl Default for ViewFilter {
    fn default() -> Self {
        Self {
            include_values: true,
            include_formulas: true,
            include_formatting: false,
            include_info_diagnostics: false,
            sheets: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Stable change anchor (for virtualized tables / navigation)
// ---------------------------------------------------------------------------

/// A stable, deterministic identifier for a single change row.
///
/// Ordering matches the canonical `(sheet_index, row, col)` sort.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ChangeAnchor {
    pub sheet_index: usize,
    pub row: u32,
    pub col: u32,
}

// ---------------------------------------------------------------------------
// Flat change row (one entry per visible cell change)
// ---------------------------------------------------------------------------

/// A single row in the flat change list presented to a GUI table.
pub struct CellChangeRow<'a> {
    /// Stable anchor for navigation and virtualized table positioning.
    pub anchor: ChangeAnchor,
    pub sheet_name: &'a str,
    pub address: &'a CellAddress,
    /// Combined change kind derived from sub-fields.
    pub change_kind: CellChangeKind,
    /// Display string for the old value (empty if Added).
    pub old_display: String,
    /// Display string for the new value (empty if Removed).
    pub new_display: String,
    /// Whether a formula also changed on this cell.
    pub formula_changed: bool,
    /// Highest diagnostic severity attached to this cell.
    pub max_severity: Option<Severity>,
}

// ---------------------------------------------------------------------------
// Sheet summary row
// ---------------------------------------------------------------------------

/// Summary line for one sheet in the sheet-tree view.
pub struct SheetSummaryRow<'a> {
    pub sheet_index: usize,
    pub name: &'a str,
    pub change: &'a SheetChange,
    pub cells_changed: usize,
    pub has_diagnostics: bool,
}

// ---------------------------------------------------------------------------
// DiffView — main adapter
// ---------------------------------------------------------------------------

/// Borrowed view over a `WorkbookDiff`, providing filtered iteration and
/// deterministic navigation for GUI applications.
pub struct DiffView<'a> {
    pub workbook: &'a WorkbookDiff,
}

impl<'a> DiffView<'a> {
    pub fn new(workbook: &'a WorkbookDiff) -> Self {
        Self { workbook }
    }

    // ------------------------------------------------------------------
    // Sheet tree
    // ------------------------------------------------------------------

    /// Iterate sheet summary rows in workbook display order.
    pub fn sheets(&self) -> impl Iterator<Item = SheetSummaryRow<'a>> {
        self.workbook.sheets.iter().enumerate().map(|(i, sd)| {
            SheetSummaryRow {
                sheet_index: i,
                name: sd.new_sheet.as_ref().or(sd.old_sheet.as_ref())
                    .map(|s| s.name.as_str()).unwrap_or("?"),
                change: &sd.change,
                cells_changed: sd.summary.cells_changed,
                has_diagnostics: !sd.diagnostics.is_empty(),
            }
        })
    }

    // ------------------------------------------------------------------
    // Flat change list
    // ------------------------------------------------------------------

    /// Collect all visible cell-change rows into a `Vec`, respecting the filter.
    ///
    /// Order is deterministic: sheet order → (row, col).
    pub fn rows(&'a self, filter: &ViewFilter) -> Vec<CellChangeRow<'a>> {
        let mut out = Vec::new();
        for (sheet_idx, sd) in self.workbook.sheets.iter().enumerate() {
            if let Some(ref allowed) = filter.sheets {
                if !allowed.contains(&sheet_idx) { continue; }
            }
            let sheet_name = sd.new_sheet.as_ref().or(sd.old_sheet.as_ref())
                .map(|s| s.name.as_str()).unwrap_or("?");
            for cd in &sd.cell_diffs {
                if let Some(row) = cell_to_row(cd, sheet_idx, sheet_name, filter) {
                    out.push(row);
                }
            }
        }
        out
    }

    /// Total number of visible change rows (may iterate; not O(1)).
    pub fn row_count(&self, filter: &ViewFilter) -> usize {
        self.rows(filter).len()
    }

    // ------------------------------------------------------------------
    // Navigation
    // ------------------------------------------------------------------

    /// Return the first change anchor in the view, or `None` if empty.
    pub fn first(&self, filter: &ViewFilter) -> Option<ChangeAnchor> {
        self.rows(filter).into_iter().next().map(|r| r.anchor)
    }

    /// Return the anchor immediately after `current`, or `None` if at end.
    pub fn next_after(&self, current: &ChangeAnchor, filter: &ViewFilter) -> Option<ChangeAnchor> {
        let mut past = false;
        for row in self.rows(filter).into_iter() {
            if past {
                return Some(row.anchor);
            }
            if &row.anchor == current {
                past = true;
            }
        }
        None
    }

    /// Return the anchor immediately before `current`, or `None` if at start.
    pub fn previous_before(&self, current: &ChangeAnchor, filter: &ViewFilter) -> Option<ChangeAnchor> {
        let mut prev: Option<ChangeAnchor> = None;
        for row in self.rows(filter).into_iter() {
            if &row.anchor == current {
                return prev;
            }
            prev = Some(row.anchor.clone());
        }
        None
    }

    // ------------------------------------------------------------------
    // Per-sheet slice
    // ------------------------------------------------------------------

    /// All cell-change rows for one sheet (by 0-based sheet index).
    pub fn sheet_rows(
        &'a self,
        sheet_index: usize,
        filter: &ViewFilter,
    ) -> Vec<CellChangeRow<'a>> {
        let mut f = filter.clone();
        f.sheets = Some(vec![sheet_index]);
        self.rows(&f)
    }
}

// ---------------------------------------------------------------------------
// Helper: CellDiff → CellChangeRow
// ---------------------------------------------------------------------------

fn cell_to_row<'a>(
    cd: &'a CellDiff,
    sheet_index: usize,
    sheet_name: &'a str,
    filter: &ViewFilter,
) -> Option<CellChangeRow<'a>> {
    let has_value = cd.value.is_some() && filter.include_values;
    let has_formula = cd.formula.is_some() && filter.include_formulas;

    if !has_value && !has_formula {
        return None;
    }

    let old_display = cd.value.as_ref()
        .map(|vc| vc.old.display_string())
        .unwrap_or_default();
    let new_display = cd.value.as_ref()
        .map(|vc| vc.new.display_string())
        .unwrap_or_default();

    let max_severity = cd.diagnostics.iter()
        .map(|d| d.severity)
        .max();

    Some(CellChangeRow {
        anchor: ChangeAnchor {
            sheet_index,
            row: cd.address.row,
            col: cd.address.col,
        },
        sheet_name,
        address: &cd.address,
        change_kind: cd.change_kind(),
        old_display,
        new_display,
        formula_changed: cd.formula.is_some(),
        max_severity,
    })
}
