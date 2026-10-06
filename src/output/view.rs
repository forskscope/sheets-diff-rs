//! Framework-neutral GUI view adapters over `WorkbookDiff` (RFC-029).
//!
//! These types borrow from `WorkbookDiff` and allocate display strings only
//! on demand.  No GUI framework dependency is introduced.

use crate::address::CellAddress;
use crate::model::{CellChangeKind, CellDiff, RowPlacement, Severity, SheetChange, WorkbookDiff};

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
// Change-row sort position (not an identifier -- see `ChangeAnchor` below)
// ---------------------------------------------------------------------------

/// The `(sheet_index, row, col)` sort position of a change row. **It is not an identifier.**
/// Under `RowKey` or `RowSignature`, two change rows can carry the same anchor, because a row
/// number is in whichever row space its change was numbered in (see [`CellDiff`]).
/// Under `Positional` it is unique. [`DiffView::next_after`] and [`DiffView::previous_before`]
/// find their argument by anchor equality, so with a colliding anchor they can fail to advance.
///
/// **`Ord` orders change rows into canonical sequence. It is not an identity, and must not be used
/// as a map key.** A `BTreeMap` keyed by anchor compiles and silently merges rows that share one.
/// To identify a change, use [`ChangeKey`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ChangeAnchor {
    pub sheet_index: usize,
    pub row: u32,
    pub col: u32,
}

/// Identifies one change within one [`WorkbookDiff`]: the sheet, the row placement, and the column.
///
/// It is unique within its diff, because each [`CellDiff`] is one coordinate in one row space. It is
/// **not** a cross-diff identity. Following "the same change" from one comparison to the next is an
/// alignment question, not a key question, and no key can answer it; the index of a sheet and the row
/// numbers both move when a file changes. Its fields are private, so it is produced by [`DiffView`]
/// and cannot be constructed by hand.
///
/// Derives `Eq` and `Hash`, so it keys a `HashMap` directly. It has no `Ord`: a consumer has no meaning
/// to attach to an ordering of changes beyond the order [`DiffView::position_of`] reports.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ChangeKey {
    sheet_index: usize,
    row_placement: RowPlacement,
    col: u32,
}

impl ChangeKey {
    /// The 0-based index of the sheet the change is in, within its workbook.
    pub fn sheet_index(&self) -> usize {
        self.sheet_index
    }

    /// Which row of each sheet the change belongs to. See [`RowPlacement`].
    pub fn row_placement(&self) -> &RowPlacement {
        &self.row_placement
    }

    /// The 1-based column of the change.
    pub fn col(&self) -> u32 {
        self.col
    }

    fn of(sheet_index: usize, cd: &CellDiff) -> Self {
        Self {
            sheet_index,
            row_placement: cd.row_placement.clone(),
            col: cd.address.col,
        }
    }
}

// ---------------------------------------------------------------------------
// Flat change row (one entry per visible cell change)
// ---------------------------------------------------------------------------

/// A single row in the flat change list presented to a GUI table.
pub struct CellChangeRow<'a> {
    /// The change row's sort position. Not unique under `RowKey` or `RowSignature`; see [`ChangeAnchor`].
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
    /// Old formula text, if a formula change is present (Q2: borrowed from the
    /// underlying `CellDiff`, so GUI consumers need not reach into the raw model).
    pub old_formula: Option<&'a str>,
    /// New formula text, if a formula change is present.
    pub new_formula: Option<&'a str>,
    /// Highest diagnostic severity attached to this cell.
    pub max_severity: Option<Severity>,
}

impl<'a> CellChangeRow<'a> {
    /// Convert this borrowed row into a fully owned [`OwnedCellChangeRow`]
    /// (Q3: convenience for consumers whose model outlives the `WorkbookDiff`).
    pub fn to_owned_row(&self) -> OwnedCellChangeRow {
        OwnedCellChangeRow {
            anchor: self.anchor.clone(),
            sheet_name: self.sheet_name.to_owned(),
            address: self.address.clone(),
            change_kind: self.change_kind,
            old_display: self.old_display.clone(),
            new_display: self.new_display.clone(),
            formula_changed: self.formula_changed,
            old_formula: self.old_formula.map(|s| s.to_owned()),
            new_formula: self.new_formula.map(|s| s.to_owned()),
            max_severity: self.max_severity,
        }
    }
}

/// Fully owned counterpart to [`CellChangeRow`] (Q3).
///
/// All borrowed fields become owned (`String`, `CellAddress`), so the row can
/// outlive the `WorkbookDiff` it was derived from. Produced by
/// [`CellChangeRow::to_owned_row`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OwnedCellChangeRow {
    pub anchor: ChangeAnchor,
    pub sheet_name: String,
    pub address: CellAddress,
    pub change_kind: CellChangeKind,
    pub old_display: String,
    pub new_display: String,
    pub formula_changed: bool,
    pub old_formula: Option<String>,
    pub new_formula: Option<String>,
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
        self.workbook
            .sheets
            .iter()
            .enumerate()
            .map(|(i, sd)| SheetSummaryRow {
                sheet_index: i,
                name: sd
                    .new_sheet
                    .as_ref()
                    .or(sd.old_sheet.as_ref())
                    .map(|s| s.name.as_str())
                    .unwrap_or("?"),
                change: &sd.change,
                cells_changed: sd.summary.cells_changed,
                has_diagnostics: !sd.diagnostics.is_empty(),
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
            if let Some(ref allowed) = filter.sheets
                && !allowed.contains(&sheet_idx)
            {
                continue;
            }
            let sheet_name = sd
                .new_sheet
                .as_ref()
                .or(sd.old_sheet.as_ref())
                .map(|s| s.name.as_str())
                .unwrap_or("?");
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
    ///
    /// Deprecated: use [`DiffView::next_change`], which cannot fail to advance.
    ///
    /// **Can fail to advance.** `current` is found by anchor equality. When two change rows share
    /// its anchor (possible under `RowKey` or `RowSignature`; see [`ChangeAnchor`]), this matches the
    /// first of them and returns the second, so called with either one it returns the later one, and
    /// a forward loop stops at the pair instead of moving past it.
    #[deprecated(
        since = "3.5.0",
        note = "can fail to advance when two change rows share an anchor; use \
                DiffView::next_change, which cannot"
    )]
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
    ///
    /// Deprecated: use [`DiffView::previous_change`], which cannot skip a change.
    ///
    /// **Can skip a change.** `current` is found by anchor equality. When two change rows share its
    /// anchor (possible under `RowKey` or `RowSignature`; see [`ChangeAnchor`]), this matches the first
    /// of them, so asked about the later one it returns the predecessor of the earlier one, and the
    /// earlier row is skipped.
    #[deprecated(
        since = "3.5.0",
        note = "can skip a change when two change rows share an anchor; use \
                DiffView::previous_change, which cannot"
    )]
    pub fn previous_before(
        &self,
        current: &ChangeAnchor,
        filter: &ViewFilter,
    ) -> Option<ChangeAnchor> {
        let mut prev: Option<ChangeAnchor> = None;
        for row in self.rows(filter).into_iter() {
            if &row.anchor == current {
                return prev;
            }
            prev = Some(row.anchor.clone());
        }
        None
    }

    /// The change at `index` in the view, or `None` past the end.
    ///
    /// Route A: a position, for walking. Moving one change is `index ± 1`, and `row_count` gives the
    /// length. Each call re-filters the whole workbook, so walking every change this way costs O(n²);
    /// that cost is pre-existing, and a cache is a separate change.
    pub fn change_at(&'a self, index: usize, filter: &ViewFilter) -> Option<CellChangeRow<'a>> {
        self.rows(filter).into_iter().nth(index)
    }

    /// The key of the change at `index` in the view, or `None` past the end.
    ///
    /// Takes the same index as [`DiffView::change_at`], so a caller holding both cannot pair one index
    /// with another's result. To visit every change, map over `0..row_count(filter)`.
    pub fn key_at(&self, index: usize, filter: &ViewFilter) -> Option<ChangeKey> {
        self.visible_cells(filter)
            .get(index)
            .map(|(sheet_index, cd)| ChangeKey::of(*sheet_index, cd))
    }

    /// The key of the first change in the view, or `None` if there are none.
    pub fn first_change(&self, filter: &ViewFilter) -> Option<ChangeKey> {
        self.key_at(0, filter)
    }

    /// The position of `key` in the view, or `None` if this view does not contain that change.
    pub fn position_of(&self, key: &ChangeKey, filter: &ViewFilter) -> Option<usize> {
        self.visible_cells(filter)
            .iter()
            .position(|(sheet_index, cd)| ChangeKey::of(*sheet_index, cd) == *key)
    }

    /// The change after `key`, or `None` at the end or if `key` is not in this view.
    ///
    /// Cannot fail to advance: keys are unique within a diff, so `key` has one position, and the
    /// result is the entry after it.
    pub fn next_change(&self, key: &ChangeKey, filter: &ViewFilter) -> Option<ChangeKey> {
        let visible = self.visible_cells(filter);
        let at = visible
            .iter()
            .position(|(sheet_index, cd)| ChangeKey::of(*sheet_index, cd) == *key)?;
        visible
            .get(at + 1)
            .map(|(sheet_index, cd)| ChangeKey::of(*sheet_index, cd))
    }

    /// The change before `key`, or `None` at the start or if `key` is not in this view.
    ///
    /// Cannot skip a change, for the same reason as [`DiffView::next_change`].
    pub fn previous_change(&self, key: &ChangeKey, filter: &ViewFilter) -> Option<ChangeKey> {
        let visible = self.visible_cells(filter);
        let at = visible
            .iter()
            .position(|(sheet_index, cd)| ChangeKey::of(*sheet_index, cd) == *key)?;
        at.checked_sub(1)
            .and_then(|prev| visible.get(prev))
            .map(|(sheet_index, cd)| ChangeKey::of(*sheet_index, cd))
    }

    /// Each visible change as its sheet index and cell diff, in the order `rows` reports them.
    ///
    /// Uses the same filter and the same `cell_to_row` as `rows`, so the two agree on which changes
    /// are visible and in what order.
    fn visible_cells(&self, filter: &ViewFilter) -> Vec<(usize, &CellDiff)> {
        let mut out = Vec::new();
        for (sheet_idx, sd) in self.workbook.sheets.iter().enumerate() {
            if let Some(ref allowed) = filter.sheets
                && !allowed.contains(&sheet_idx)
            {
                continue;
            }
            let sheet_name = sd
                .new_sheet
                .as_ref()
                .or(sd.old_sheet.as_ref())
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            for cd in &sd.cell_diffs {
                if cell_to_row(cd, sheet_idx, sheet_name, filter).is_some() {
                    out.push((sheet_idx, cd));
                }
            }
        }
        out
    }

    // ------------------------------------------------------------------
    // Per-sheet slice
    // ------------------------------------------------------------------

    /// All cell-change rows for one sheet (by 0-based sheet index).
    pub fn sheet_rows(&'a self, sheet_index: usize, filter: &ViewFilter) -> Vec<CellChangeRow<'a>> {
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

    let old_display = cd
        .value
        .as_ref()
        .map(|vc| vc.old.display_string())
        .unwrap_or_default();
    let new_display = cd
        .value
        .as_ref()
        .map(|vc| vc.new.display_string())
        .unwrap_or_default();

    let max_severity = cd.diagnostics.iter().map(|d| d.severity).max();

    // Borrow formula text from the underlying change, if present (Q2).
    let (old_formula, new_formula) = match &cd.formula {
        Some(fc) => (
            fc.old.as_ref().map(|t| t.raw.as_str()),
            fc.new.as_ref().map(|t| t.raw.as_str()),
        ),
        None => (None, None),
    };

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
        old_formula,
        new_formula,
        max_severity,
    })
}
