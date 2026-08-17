//! Core comparison pipeline and internal entry-point implementations.
//!
//! Public entry points live in `lib.rs`; this module owns the pipeline logic.

use std::collections::BTreeMap;
use std::io::{Read, Seek};

use calamine::Reader;

use crate::address::{CellAddress, ComparedRange};
use crate::align::{AlignCellMap, compute_row_mapping};
use crate::compare::{compare_formulas, compare_values};
use crate::error::{LimitKind, SheetsDiffError};
use crate::matcher::{MatchedPair, match_sheets};
use crate::meta::compare_workbook_metadata;
use crate::model::{
    AlignmentSummary, CellDiff, Diagnostic, DiagnosticKind, DiagnosticLocation, DiffMetrics,
    DiffStage, Severity, SheetChange, SheetDiff, SheetRef, SheetSummary, Side, WorkbookDiff,
    WorkbookSideInfo,
};
use crate::normalize::normalize_cell_value;
use crate::objects::report_object_coverage;
use crate::open::{OpenedWorkbook, open_bytes, open_path, open_reader};
use crate::options::{AlignmentMode, DiffEvent, DiffOptions};

// ---------------------------------------------------------------------------
// Internal normalised cell
// ---------------------------------------------------------------------------

struct NormalizedCell {
    value: crate::model::CellValue,
    formula: Option<String>,
}

type CellMap = BTreeMap<(u32, u32), NormalizedCell>;

/// A sheet's normalised cells plus its used-range bounds (1-based, inclusive).
type SheetReadResult = (CellMap, Option<(u32, u32)>, Option<(u32, u32)>);

/// Mid-sheet cancellation polling interval, in cells (M7 Handoff 03).
///
/// Derived from a stated target latency, not chosen arbitrarily: unit 01
/// measured ~1.9 microseconds/cell for a full sheet-pair pass (300,000
/// cells in ~567 ms — `docs/src/maintainers/performance.md`, Q4). Targeting
/// 100 ms worst-case latency between a cancellation request and the next
/// checkpoint (the threshold at which a UI action reads as instantaneous)
/// gives 100,000 / 1.9 ≈ 52,631 cells; rounded down to a plain number
/// comfortably under that budget — 50,000 cells is ≈ 95 ms worst case at
/// the measured per-cell rate.
const CANCEL_POLL_INTERVAL: u64 = 50_000;

/// Build a value-only map for alignment (avoids cloning formulas).
fn cell_map_to_align(cells: &CellMap) -> AlignCellMap {
    cells.iter().map(|(k, v)| (*k, v.value.clone())).collect()
}

// ---------------------------------------------------------------------------
// Coordinate-set key (D-03: keeps old-row and new-row numbering distinct)
// ---------------------------------------------------------------------------

/// A coordinate to compare, tagged with which row-numbering space it came
/// from so a numeric coincidence between an old-row and a new-row number can
/// never merge two distinct logical cells (see `build_sheet_diff`).
///
/// Ordering is by `(row, col)` only — the variant is a tie-breaker, never
/// the primary sort key — so output stays sorted by row then column
/// regardless of which space a coordinate came from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CoordKey {
    /// Matched or removed — `row`/`col` are in the OLD sheet's row space.
    Old(u32, u32),
    /// Inserted — `row`/`col` are in the NEW sheet's row space; no old-side
    /// counterpart exists.
    InsertedNew(u32, u32),
    /// No alignment mapping: old and new share the same row-number space.
    Positional(u32, u32),
}

impl CoordKey {
    fn addr(&self) -> (u32, u32) {
        match *self {
            CoordKey::Old(r, c) | CoordKey::InsertedNew(r, c) | CoordKey::Positional(r, c) => {
                (r, c)
            }
        }
    }

    fn variant_rank(&self) -> u8 {
        match self {
            CoordKey::Old(..) => 0,
            CoordKey::InsertedNew(..) => 1,
            CoordKey::Positional(..) => 2,
        }
    }
}

impl PartialOrd for CoordKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CoordKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.addr()
            .cmp(&other.addr())
            .then_with(|| self.variant_rank().cmp(&other.variant_rank()))
    }
}

// ---------------------------------------------------------------------------
// Public pipeline entry points (called from lib.rs)
// ---------------------------------------------------------------------------

pub fn run_compare_paths(
    old: impl AsRef<std::path::Path>,
    new: impl AsRef<std::path::Path>,
    opts: DiffOptions,
) -> Result<WorkbookDiff, SheetsDiffError> {
    opts.validate()?;
    let old_wb = open_path(old, Side::Old, opts.limits.max_input_bytes)?;
    let new_wb = open_path(new, Side::New, opts.limits.max_input_bytes)?;
    run_pipeline(old_wb, new_wb, opts)
}

pub fn run_compare_bytes(
    old: impl AsRef<[u8]>,
    new: impl AsRef<[u8]>,
    opts: DiffOptions,
) -> Result<WorkbookDiff, SheetsDiffError> {
    opts.validate()?;
    let old_wb = open_bytes(old, Side::Old, None, opts.limits.max_input_bytes)?;
    let new_wb = open_bytes(new, Side::New, None, opts.limits.max_input_bytes)?;
    run_pipeline(old_wb, new_wb, opts)
}

pub fn run_compare_readers<R1, R2>(
    old: R1,
    new: R2,
    opts: DiffOptions,
) -> Result<WorkbookDiff, SheetsDiffError>
where
    R1: Read + Seek,
    R2: Read + Seek,
{
    opts.validate()?;
    let old_wb = open_reader(old, Side::Old, None, opts.limits.max_input_bytes)?;
    let new_wb = open_reader(new, Side::New, None, opts.limits.max_input_bytes)?;
    run_pipeline(old_wb, new_wb, opts)
}

// ---------------------------------------------------------------------------
// Core pipeline
// ---------------------------------------------------------------------------

fn run_pipeline(
    mut old_wb: OpenedWorkbook,
    mut new_wb: OpenedWorkbook,
    mut opts: DiffOptions,
) -> Result<WorkbookDiff, SheetsDiffError> {
    let mut workbook_diagnostics: Vec<Diagnostic> = Vec::new();

    emit(&mut opts, DiffEvent::Started);
    emit(&mut opts, DiffEvent::OpeningWorkbook { side: Side::Old });
    emit(
        &mut opts,
        DiffEvent::WorkbookOpened {
            side: Side::Old,
            sheet_count: old_wb.sheets.len(),
        },
    );
    emit(&mut opts, DiffEvent::OpeningWorkbook { side: Side::New });
    emit(
        &mut opts,
        DiffEvent::WorkbookOpened {
            side: Side::New,
            sheet_count: new_wb.sheets.len(),
        },
    );

    // Sheet limit check (RFC-012 / RFC-033 §10)
    if let Some(max) = opts.limits.max_sheets {
        // Count logical pairs, not raw sheet totals: limit applies to each side.
        let old_count = old_wb.sheets.len() as u64;
        let new_count = new_wb.sheets.len() as u64;
        let observed = old_count.max(new_count);
        if observed > max as u64 {
            return Err(SheetsDiffError::LimitExceeded {
                limit: LimitKind::Sheets,
                observed,
            });
        }
    }

    // Side metadata
    let old_info = WorkbookSideInfo {
        source: old_wb.source.clone(),
        // calamine 0.36 exposes no workbook-level name in the public API.
        workbook_name: None,
        sheet_count: old_wb.sheets.len(),
    };
    let new_info = WorkbookSideInfo {
        source: new_wb.source.clone(),
        workbook_name: None,
        sheet_count: new_wb.sheets.len(),
    };

    // Sheet matching
    emit(&mut opts, DiffEvent::MatchingSheets);
    let matched = match_sheets(
        &old_wb.sheets.clone(),
        &new_wb.sheets.clone(),
        opts.matching.sheet_matching,
        &mut workbook_diagnostics,
    );

    // Object/unsupported feature coverage reporting (RFC-023)
    report_object_coverage(
        &mut old_wb,
        &mut new_wb,
        opts.output.objects,
        &mut workbook_diagnostics,
    );

    // Workbook metadata comparison (RFC-021)
    let meta_changes =
        compare_workbook_metadata(&mut old_wb, &mut new_wb, &opts, &mut workbook_diagnostics);

    // Process each sheet pair
    let total_sheets = matched.len();
    let mut sheet_diffs: Vec<SheetDiff> = Vec::with_capacity(total_sheets);
    let mut total_diffs: u64 = 0;
    let mut total_cells_read: u64 = 0;
    let mut total_cells_compared: u64 = 0;
    let mut metrics = DiffMetrics::default();

    for (idx, pair) in matched.into_iter().enumerate() {
        check_cancel(&opts)?;

        let sheet_name = pair
            .new_sheet
            .as_ref()
            .or(pair.old_sheet.as_ref())
            .map(|s| s.name.clone())
            .unwrap_or_default();

        emit(
            &mut opts,
            DiffEvent::SheetStarted {
                index: idx,
                total: total_sheets,
                name: sheet_name,
            },
        );

        let sheet_diff = process_sheet_pair(
            &pair,
            &mut old_wb,
            &mut new_wb,
            &opts,
            &mut total_diffs,
            &mut total_cells_read,
            &mut total_cells_compared,
        )?;

        let changed = sheet_diff.cell_diffs.len();
        metrics.sheets_read += 1;
        // cells_read and cells_compared are accumulated in read_sheet_cells /
        // build_sheet_diff via total_cells_read / total_cells_compared.
        metrics.diffs_emitted += changed as u64;
        emit(
            &mut opts,
            DiffEvent::SheetFinished {
                index: idx,
                changed_cells: changed,
            },
        );

        sheet_diffs.push(sheet_diff);
    }

    // Sort sheets: old-workbook order first, then new-only (Added) sheets.
    sheet_diffs.sort_by_key(|sd| {
        sd.old_sheet
            .as_ref()
            .map(|s| (0usize, s.index))
            .unwrap_or_else(|| (1, sd.new_sheet.as_ref().map(|s| s.index).unwrap_or(0)))
    });

    metrics.cells_read = total_cells_read;
    metrics.cells_compared = total_cells_compared;
    metrics.diagnostics_emitted = workbook_diagnostics.len() as u64
        + sheet_diffs
            .iter()
            .map(|s| s.diagnostics.len() as u64)
            .sum::<u64>();
    let summary = WorkbookDiff::derive_summary(&sheet_diffs, &workbook_diagnostics);

    emit(&mut opts, DiffEvent::Finished);

    Ok(WorkbookDiff {
        old: old_info,
        new: new_info,
        sheets: sheet_diffs,
        workbook_changes: meta_changes,
        object_changes: Vec::new(),
        diagnostics: workbook_diagnostics,
        summary,
        metrics,
    })
}

// ---------------------------------------------------------------------------
// Per-sheet processing
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn process_sheet_pair(
    pair: &MatchedPair,
    old_wb: &mut OpenedWorkbook,
    new_wb: &mut OpenedWorkbook,
    opts: &DiffOptions,
    total_diffs: &mut u64,
    total_cells_read: &mut u64,
    total_cells_compared: &mut u64,
) -> Result<SheetDiff, SheetsDiffError> {
    let mut sheet_diag: Vec<Diagnostic> = Vec::new();

    let (old_map, old_start, old_end) = match &pair.old_sheet {
        Some(s) => read_sheet_cells(
            old_wb,
            s,
            Side::Old,
            opts,
            total_cells_read,
            &mut sheet_diag,
        )?,
        None => (CellMap::new(), None, None),
    };
    let (new_map, new_start, new_end) = match &pair.new_sheet {
        Some(s) => read_sheet_cells(
            new_wb,
            s,
            Side::New,
            opts,
            total_cells_read,
            &mut sheet_diag,
        )?,
        None => (CellMap::new(), None, None),
    };
    build_sheet_diff(
        pair,
        old_map,
        old_start,
        old_end,
        new_map,
        new_start,
        new_end,
        opts,
        total_diffs,
        total_cells_compared,
        &mut sheet_diag,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_sheet_diff(
    pair: &MatchedPair,
    old_map: CellMap,
    old_start: Option<(u32, u32)>,
    old_end: Option<(u32, u32)>,
    new_map: CellMap,
    new_start: Option<(u32, u32)>,
    new_end: Option<(u32, u32)>,
    opts: &DiffOptions,
    total_diffs: &mut u64,
    total_cells_compared: &mut u64,
    sheet_diag: &mut Vec<Diagnostic>,
) -> Result<SheetDiff, SheetsDiffError> {
    let compared_range = ComparedRange::union(old_start, old_end, new_start, new_end);

    // Alignment (RFC-011): compute row mapping if mode is not Positional.
    let align_mapping = if !matches!(opts.matching.alignment, AlignmentMode::Positional) {
        let old_align = cell_map_to_align(&old_map);
        let new_align = cell_map_to_align(&new_map);
        compute_row_mapping(
            &old_align,
            &new_align,
            &opts.matching.alignment,
            opts.limits.max_alignment_product,
            sheet_diag,
        )
    } else {
        None
    };

    // Build the coordinate set, remapping new-side rows when aligned.
    //
    // D-03: matched/removed rows are numbered in the OLD sheet's row space;
    // inserted rows have no old-side counterpart and are numbered in the NEW
    // sheet's row space. These are two independent numbering sequences — an
    // inserted new-row number can (and in practice often does) coincide with
    // an unrelated matched/removed old-row number. Collapsing both into one
    // `(row, col)` `BTreeSet` let the set silently dedupe two distinct
    // logical cells into one, and then had the lookup resolve the wrong
    // side's row for whichever survived (or leaked an unrelated new-side
    // cell into a removed row's comparison, and vice versa). `CoordKey`
    // keeps every source unambiguous; only the row/col *address* — not the
    // source — determines sort order, so the existing row-then-col output
    // ordering is unchanged.
    let mut coords: std::collections::BTreeSet<CoordKey> = std::collections::BTreeSet::new();
    match &align_mapping {
        Some(mapping) => {
            // Matched pairs — canonical address is the OLD row; union of
            // both sides' columns (a matched row's new side may have
            // columns absent from the old side, or vice versa).
            for (old_row, new_row) in &mapping.matched {
                let old_cols: Vec<u32> = old_map
                    .keys()
                    .filter(|(r, _)| r == old_row)
                    .map(|(_, c)| *c)
                    .collect();
                let new_cols: Vec<u32> = new_map
                    .keys()
                    .filter(|(r, _)| r == new_row)
                    .map(|(_, c)| *c)
                    .collect();
                for c in old_cols.iter().chain(new_cols.iter()) {
                    coords.insert(CoordKey::Old(*old_row, *c));
                }
            }
            // Removed rows — old side only; no new-side counterpart exists.
            for r in &mapping.removed {
                for (_, c) in old_map.keys().filter(|(row, _)| row == r) {
                    coords.insert(CoordKey::Old(*r, *c));
                }
            }
            // Inserted rows — new side only; no old-side counterpart
            // exists. Keyed separately so a numeric coincidence with an
            // old-row number above is never merged with it.
            for r in &mapping.inserted {
                for (_, c) in new_map.keys().filter(|(row, _)| row == r) {
                    coords.insert(CoordKey::InsertedNew(*r, *c));
                }
            }
        }
        None => {
            // No alignment: old and new share the same row-number space
            // directly (positional comparison).
            coords.extend(old_map.keys().map(|&(r, c)| CoordKey::Positional(r, c)));
            coords.extend(new_map.keys().map(|&(r, c)| CoordKey::Positional(r, c)));
        }
    }

    // Every coordinate in `coords` is compared below, whether or not it
    // produces a diff — this is where "compared" happens, not something
    // reconstructed afterwards from which cells ended up in `cell_diffs`.
    // The limit is checked here too: cumulatively across the whole
    // comparison (matching `max_diffs_returned`'s use of `*total_diffs`
    // below, not a per-sheet-local count), and before the coordinate loop
    // begins, so a comparison that would exceed the bound is refused rather
    // than aborted partway through work already started.
    let sheet_cells_compared = coords.len() as u64;
    if let Some(max) = opts.limits.max_cells_compared {
        let observed = *total_cells_compared + sheet_cells_compared;
        if observed > max {
            return Err(SheetsDiffError::LimitExceeded {
                limit: LimitKind::CellsCompared,
                observed,
            });
        }
    }
    *total_cells_compared += sheet_cells_compared;

    let mut cell_diffs: Vec<CellDiff> = Vec::new();
    let mut summary = SheetSummary::default();

    // Reusable empty sentinel — avoids repeated heap allocation.
    let empty_cell = NormalizedCell {
        value: crate::model::CellValue::Empty,
        formula: None,
    };

    for (compare_idx, key) in coords.iter().enumerate() {
        // Mid-sheet cancellation checkpoint (M7 Handoff 03). Polled on an
        // interval, not every iteration — `is_cancelled()` is a dynamic
        // trait call, and per-cell polling on a 300,000-cell sheet is
        // 300,000 virtual calls. `check_cancel` itself short-circuits to a
        // cheap `Option` check when no `Cancellation` is configured.
        if (compare_idx as u64 + 1).is_multiple_of(CANCEL_POLL_INTERVAL) {
            check_cancel(opts)?;
        }
        // `old_lookup`/`new_lookup` are `None` when that side genuinely has
        // no counterpart for this coordinate (never a numeric fallback that
        // could accidentally hit an unrelated row — the D-03 defect).
        let (row, col, old_lookup, new_lookup): (u32, u32, Option<u32>, Option<u32>) = match *key {
            CoordKey::Old(r, c) => {
                let new_lookup = align_mapping
                    .as_ref()
                    .and_then(|m| m.matched.get(&r))
                    .copied();
                (r, c, Some(r), new_lookup)
            }
            CoordKey::InsertedNew(r, c) => (r, c, None, Some(r)),
            CoordKey::Positional(r, c) => (r, c, Some(r), Some(r)),
        };

        let old_cell = old_lookup
            .and_then(|r| old_map.get(&(r, col)))
            .unwrap_or(&empty_cell);
        let new_cell = new_lookup
            .and_then(|r| new_map.get(&(r, col)))
            .unwrap_or(&empty_cell);

        let value_change = compare_values(&old_cell.value, &new_cell.value, &opts.comparison.value);

        let formula_change = compare_formulas(
            old_cell.formula.as_deref(),
            new_cell.formula.as_deref(),
            opts.comparison.formula,
        );

        if value_change.is_none() && formula_change.is_none() {
            continue;
        }

        // diffs-returned limit
        if let Some(max) = opts.limits.max_diffs_returned
            && *total_diffs >= max
        {
            return Err(SheetsDiffError::LimitExceeded {
                limit: LimitKind::DiffsReturned,
                observed: *total_diffs + 1,
            });
        }

        if value_change.is_some() {
            summary.values_changed += 1;
        }
        if formula_change.is_some() {
            summary.formulas_changed += 1;
        }
        summary.cells_changed += 1;
        *total_diffs += 1;

        let address = CellAddress::new_unchecked(row, col);
        cell_diffs.push(CellDiff {
            address,
            value: value_change,
            formula: formula_change,
            format: None,
            diagnostics: Vec::new(),
        });
    }

    // Upgrade Unchanged → Modified when there are cell diffs.
    let change = match &pair.change {
        SheetChange::Unchanged if !cell_diffs.is_empty() => SheetChange::Modified,
        other => other.clone(),
    };

    Ok(SheetDiff {
        old_sheet: pair.old_sheet.clone(),
        new_sheet: pair.new_sheet.clone(),
        change,
        cell_diffs,
        compared_range,
        alignment_summary: align_mapping.map(|m| AlignmentSummary {
            inserted_rows: m.summary.inserted_rows,
            removed_rows: m.summary.removed_rows,
            matched_rows: m.summary.matched_rows,
            confidence: m.summary.confidence,
        }),
        diagnostics: std::mem::take(sheet_diag),
        summary,
    })
}

// ---------------------------------------------------------------------------
// Sheet cell reading (M2 / M3)
// ---------------------------------------------------------------------------

/// Read all non-empty cells from a sheet into a `BTreeMap<(row1, col1), NormalizedCell>`.
///
/// - Row and column are **1-based**.
/// - Empty cells are omitted; the map is sparse.
/// - Formulas are read best-effort; a diagnostic is attached when formula text
///   is unavailable for a cell that has a cached formula-like value.
fn read_sheet_cells(
    wb: &mut OpenedWorkbook,
    sheet: &SheetRef,
    side: Side,
    opts: &DiffOptions,
    total_cells_read: &mut u64,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<SheetReadResult, SheetsDiffError> {
    let range = wb
        .reader
        .worksheet_range(&sheet.name)
        .map_err(|e| SheetsDiffError::read_sheet(side, sheet.clone(), e))?;

    // Formula range is best-effort; worksheet_formula returns an error for sheets
    // with no formulas, which is fine — we just won't emit formula changes for them.
    let formula_range = wb.reader.worksheet_formula(&sheet.name).ok();
    let has_formulas = formula_range.is_some();

    let mut cells: CellMap = BTreeMap::new();
    let mut range_start: Option<(u32, u32)> = None;
    let mut range_end: Option<(u32, u32)> = None;

    // calamine's Range::rows() gives relative (0-based) iteration; the absolute
    // top-left corner of the used range is range.start().
    let origin = range.start().unwrap_or((0, 0));

    // Local to this call (this side of this sheet), not the cumulative
    // `total_cells_read` accumulator below — a mid-sheet cancellation
    // checkpoint every `CANCEL_POLL_INTERVAL` cells (M7 Handoff 03).
    let mut read_poll_count: u64 = 0;

    for (row_idx, row) in range.rows().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            read_poll_count += 1;
            if read_poll_count.is_multiple_of(CANCEL_POLL_INTERVAL) {
                check_cancel(opts)?;
            }

            // max_cells_read limit
            *total_cells_read += 1;
            if let Some(max) = opts.limits.max_cells_read
                && *total_cells_read > max
            {
                return Err(SheetsDiffError::LimitExceeded {
                    limit: LimitKind::CellsRead,
                    observed: *total_cells_read,
                });
            }

            let value = normalize_cell_value(cell, wb.is_1904);
            if matches!(value, crate::model::CellValue::Empty) {
                continue;
            }

            // Convert to 1-based absolute coordinates.
            let row1 = origin.0 + row_idx as u32 + 1;
            let col1 = origin.1 + col_idx as u32 + 1;

            // Look up formula text via absolute coordinates. `row_idx`/
            // `col_idx` are relative to the VALUE range's origin, which need
            // not coincide with the FORMULA range's own origin (D-04):
            // `worksheet_formula`'s range is built only from cells that
            // actually carry formula text, so its top-left corner is the
            // first *formula* cell, not the first populated cell — applying
            // value-range-relative indices to it directly (as `Range::get`
            // does) silently offsets or drops formula text whenever the two
            // origins differ. `Range::get_value` translates through the
            // formula range's own `start()`, so this is correct whether or
            // not the origins coincide.
            let formula = formula_range
                .as_ref()
                .and_then(|fr| fr.get_value((origin.0 + row_idx as u32, origin.1 + col_idx as u32)))
                .filter(|s| !s.is_empty())
                .cloned();

            // Diagnostic: formula text unavailable for a cell that looks like it
            // might have a formula (numeric cached value, formula range present but
            // no text at this position).
            if has_formulas
                && formula.is_none()
                && opts.comparison.include_formula_cached_values
                && matches!(
                    value,
                    crate::model::CellValue::Integer(_) | crate::model::CellValue::Number(_)
                )
            {
                // Not every numeric cell is a formula; this is expected and not
                // worth a diagnostic unless the sheet does have formulas.
                // Emit Info-level only — don't spam warnings on plain data sheets.
                diagnostics.push(Diagnostic {
                    severity: Severity::Info,
                    kind: DiagnosticKind::FormulaUnavailable,
                    location: DiagnosticLocation {
                        stage: DiffStage::Read,
                        sheet_order: Some(sheet.index),
                        sheet_name: Some(sheet.name.clone()),
                        address: Some(CellAddress::new_unchecked(row1, col1)),
                    },
                    message: format!(
                        "formula text unavailable for numeric cell at {}{}",
                        crate::address::col_to_label(col1),
                        row1
                    ),
                });
            }

            update_bounds(&mut range_start, &mut range_end, row1, col1);
            cells.insert((row1, col1), NormalizedCell { value, formula });
        }
    }

    Ok((cells, range_start, range_end))
}

fn update_bounds(start: &mut Option<(u32, u32)>, end: &mut Option<(u32, u32)>, row: u32, col: u32) {
    *start = Some(match *start {
        None => (row, col),
        Some((r, c)) => (r.min(row), c.min(col)),
    });
    *end = Some(match *end {
        None => (row, col),
        Some((r, c)) => (r.max(row), c.max(col)),
    });
}

// ---------------------------------------------------------------------------
// Progress / cancellation helpers (M5)
// ---------------------------------------------------------------------------

/// Emit a progress event to the sink in DiffOptions, if one is configured.
///
/// Takes `opts` as `&mut` so we can call `&mut self` on the boxed trait object.
fn emit(opts: &mut DiffOptions, event: DiffEvent) {
    if let Some(sink) = opts.execution.progress.as_mut() {
        sink.on_event(event);
    }
}

/// Check the cancellation predicate and return `Err(Cancelled)` if fired.
fn check_cancel(opts: &DiffOptions) -> Result<(), SheetsDiffError> {
    if let Some(ref cancel) = opts.execution.cancellation
        && cancel.is_cancelled()
    {
        return Err(SheetsDiffError::Cancelled);
    }
    Ok(())
}
