//! Core comparison pipeline and internal entry-point implementations.
//!
//! Public entry points live in `lib.rs`; this module owns the pipeline logic.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek};

use calamine::Reader;

use crate::address::{CellAddress, ComparedRange};
use crate::align::compute_row_mapping;
use crate::compare::{FormulaContext, compare_formulas, compare_values};
use crate::error::{LimitKind, SheetsDiffError};
use crate::formula_refs::{RowMap, normalise_defined_names};
use crate::matcher::{MatchedPair, match_sheets};
use crate::meta::compare_workbook_metadata;
use crate::model::{
    AlignmentSummary, CellDiff, Diagnostic, DiagnosticKind, DiagnosticLocation, DiffMetrics,
    DiffStage, RowPlacement, Severity, SheetChange, SheetDiff, SheetRef, SheetSummary, Side,
    WorkbookDiff, WorkbookSideInfo,
};
use crate::normalize::normalize_cell_value;
use crate::objects::report_object_coverage;
use crate::open::{OpenedWorkbook, open_bytes, open_path, open_reader};
use crate::options::{AlignmentMode, DiffEvent, DiffOptions};

// ---------------------------------------------------------------------------
// Internal normalised cell
// ---------------------------------------------------------------------------

pub(crate) struct NormalizedCell {
    pub(crate) value: crate::model::CellValue,
    formula: Option<String>,
}

#[cfg(test)]
impl NormalizedCell {
    /// Test-only constructor — lets `align`'s tests build a `CellMap` fixture
    /// without exposing `formula` (unused by alignment) crate-wide.
    pub(crate) fn for_test(value: crate::model::CellValue) -> Self {
        Self {
            value,
            formula: None,
        }
    }
}

pub(crate) type CellMap = BTreeMap<(u32, u32), NormalizedCell>;

/// A sheet's normalised cells plus its used-range bounds (1-based, inclusive).
type SheetReadResult = (CellMap, Option<(u32, u32)>, Option<(u32, u32)>);

/// Mid-sheet cancellation polling interval (M7 Handoff 03): the number of
/// **streamed cell records** between polls in the read phase (blank records
/// count, and a sheet's values pass and formula pass share one counter), and of
/// **coordinates compared** between polls in the compare phase.
///
/// Provenance, not a guarantee. It was derived from a stated target latency:
/// unit 01 measured ~1.9 microseconds/cell for a full sheet-pair pass (300,000
/// cells in ~567 ms — `docs/src/maintainers/performance.md`, Q4). Targeting
/// 100 ms between a cancellation request and the next checkpoint (the threshold
/// at which a UI action reads as instantaneous) gives 100,000 / 1.9 ≈ 52,631;
/// rounded down to a plain number, 50,000 is ≈ 95 ms at that rate.
///
/// **That rate was measured on the dense read that the streaming read replaced,
/// and has not been re-measured since**, so neither the ≈ 95 ms nor the 100 ms
/// is a figure for the current reader.
const CANCEL_POLL_INTERVAL: u64 = 50_000;

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

    // Every defined name in either workbook, read once (the same call `meta.rs` makes). Formula annotation
    // refuses any word that is one. The union over-refuses, which is the safe direction, and the set is never
    // defaulted: an empty set means neither workbook has any.
    let defined_names = normalise_defined_names(
        old_wb
            .reader
            .defined_names()
            .iter()
            .chain(new_wb.reader.defined_names().iter())
            .map(|(name, _)| name.as_str()),
    );

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
            &SheetInputs {
                opts: &opts,
                defined_names: &defined_names,
            },
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

    // Diagnostic collection filter (`DiagnosticOptions::min_severity`). Applied
    // **once, here**: after every diagnostic vector is assembled — the workbook's
    // and each sheet's — and *before* anything counts them, so
    // `metrics.diagnostics_emitted` and `DiffSummary::diagnostics` (derived below)
    // follow what was kept and cannot disagree with the vectors. It is a
    // collection filter, not a display control: a caller who never renders
    // anything observes it. The eleven push sites are deliberately untouched, so
    // there is one place to read and one place to be wrong.
    if let Some(min) = opts.diagnostics.min_severity {
        let keep = |d: &Diagnostic| d.severity >= min;
        workbook_diagnostics.retain(keep);
        for sheet in &mut sheet_diffs {
            sheet.diagnostics.retain(keep);
            for cell in &mut sheet.cell_diffs {
                cell.diagnostics.retain(keep);
            }
        }
    }

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

/// What every sheet pair is processed against, bundled so the per-sheet functions stay under clippy's argument limit.
struct SheetInputs<'a> {
    opts: &'a DiffOptions,
    /// Every defined name in either workbook, normalised. See `run_pipeline`.
    defined_names: &'a BTreeSet<String>,
}

fn process_sheet_pair(
    pair: &MatchedPair,
    old_wb: &mut OpenedWorkbook,
    new_wb: &mut OpenedWorkbook,
    inputs: &SheetInputs<'_>,
    total_diffs: &mut u64,
    total_cells_read: &mut u64,
    total_cells_compared: &mut u64,
) -> Result<SheetDiff, SheetsDiffError> {
    let opts = inputs.opts;
    let mut sheet_diag: Vec<Diagnostic> = Vec::new();

    let old: SheetReadResult = match &pair.old_sheet {
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
    let new: SheetReadResult = match &pair.new_sheet {
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
        old,
        new,
        inputs,
        total_diffs,
        total_cells_compared,
        &mut sheet_diag,
    )
}

fn build_sheet_diff(
    pair: &MatchedPair,
    (old_map, old_start, old_end): SheetReadResult,
    (new_map, new_start, new_end): SheetReadResult,
    inputs: &SheetInputs<'_>,
    total_diffs: &mut u64,
    total_cells_compared: &mut u64,
    sheet_diag: &mut Vec<Diagnostic>,
) -> Result<SheetDiff, SheetsDiffError> {
    let (opts, defined_names) = (inputs.opts, inputs.defined_names);
    let compared_range = ComparedRange::union(old_start, old_end, new_start, new_end);

    // Alignment (RFC-011): compute row mapping if mode is not Positional.
    let align_mapping = if !matches!(opts.matching.alignment, AlignmentMode::Positional) {
        // The sheet the alignment warnings are about: the new side, else the old one — the label the
        // text renderer uses. A matched pair always has at least one side.
        let sheet = pair
            .new_sheet
            .as_ref()
            .or(pair.old_sheet.as_ref())
            .expect("a matched pair has at least one side");
        compute_row_mapping(
            &old_map,
            &new_map,
            &opts.matching.alignment,
            opts.limits.max_alignment_product,
            opts.execution.cancellation.as_deref(),
            sheet,
            sheet_diag,
        )?
    } else {
        None
    };

    // The row mapping as formula annotation needs it, with its monotonicity computed once for the sheet pair.
    let row_map = align_mapping.as_ref().map(|m| RowMap::new(&m.matched));

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
            // Each loop reads one row's cells with a range query on the ordered map — O(log n + the
            // row's cells) — and not a scan of the whole map per row, which made building this set
            // O(rows x cells) (f131: 37 s for one row against 40,000; nothing in `Limits` bounded it).
            // `(r, 0)..=(r, u32::MAX)` is every possible column of row `r`, and both ends are inclusive,
            // so column 0 and the largest column are covered.
            //
            // Each loop still polls for cancellation once per row. That is no longer "once per full scan of
            // a map"; it is once per row, and the set is O(rows) to build, so a large sheet still takes
            // time and the token is still worth looking at.

            // Matched pairs — canonical address is the OLD row; union of
            // both sides' columns (a matched row's new side may have
            // columns absent from the old side, or vice versa).
            for (old_row, new_row) in &mapping.matched {
                check_cancel(opts)?;
                for (_, c) in old_map
                    .range((*old_row, 0)..=(*old_row, u32::MAX))
                    .map(|(k, _)| k)
                {
                    coords.insert(CoordKey::Old(*old_row, *c));
                }
                for (_, c) in new_map
                    .range((*new_row, 0)..=(*new_row, u32::MAX))
                    .map(|(k, _)| k)
                {
                    coords.insert(CoordKey::Old(*old_row, *c));
                }
            }
            // Removed rows — old side only; no new-side counterpart exists.
            for r in &mapping.removed {
                check_cancel(opts)?;
                for (_, c) in old_map.range((*r, 0)..=(*r, u32::MAX)).map(|(k, _)| k) {
                    coords.insert(CoordKey::Old(*r, *c));
                }
            }
            // Inserted rows — new side only; no old-side counterpart
            // exists. Keyed separately so a numeric coincidence with an
            // old-row number above is never merged with it.
            for r in &mapping.inserted {
                check_cancel(opts)?;
                for (_, c) in new_map.range((*r, 0)..=(*r, u32::MAX)).map(|(k, _)| k) {
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
        let (row, col, old_lookup, new_lookup, row_placement): (
            u32,
            u32,
            Option<u32>,
            Option<u32>,
            RowPlacement,
        ) = match *key {
            CoordKey::Old(r, c) => {
                let new_lookup = align_mapping
                    .as_ref()
                    .and_then(|m| m.matched.get(&r))
                    .copied();
                let placement = match new_lookup {
                    Some(n) => RowPlacement::PairedByAlignment {
                        old_row: r,
                        new_row: n,
                    },
                    None => RowPlacement::UnpairedInOldSheet { old_row: r },
                };
                (r, c, Some(r), new_lookup, placement)
            }
            CoordKey::InsertedNew(r, c) => (
                r,
                c,
                None,
                Some(r),
                RowPlacement::UnpairedInNewSheet { new_row: r },
            ),
            CoordKey::Positional(r, c) => (
                r,
                c,
                Some(r),
                Some(r),
                RowPlacement::ComparedPositionally { row: r },
            ),
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
            &FormulaContext {
                placement: &row_placement,
                rows: row_map.as_ref(),
                names: defined_names,
            },
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
            row_placement,
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
            reasons: m.summary.reasons,
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
///
/// The sheet is **streamed** (`Xlsx::worksheet_cells_reader`), not read through
/// `worksheet_range`. `worksheet_range` returns a *dense* `Range` whose
/// allocation is rows × columns of the bounding box of the populated cells, so
/// one stray cell far from the data made a few-kilobyte workbook allocate
/// gigabytes — before `max_cells_read` or the cancellation poll could run,
/// because both lived in a loop over the finished range. Streaming makes memory
/// proportional to the populated cells and puts both checks inside the loop that
/// spends the resource.
///
/// `cells_read` and `max_cells_read` count **populated cells**: each sheet contributes the
/// cells this function retains in the returned map, cumulatively across sheets and sides.
/// (Through 2.6.0 they counted the *area of the bounding box* of those cells — the memory a
/// dense range allocated, which streaming stopped spending.) The bound is evaluated on that
/// running count before each cell is retained, so it fires before the memory it bounds is
/// spent. A blank record — a styled cell with no value — is not counted, exactly as it is
/// not retained. The cancellation poll counts every cell record streamed, blank or not: a
/// different quantity, and deliberately so, since blank records cost time.
fn read_sheet_cells(
    wb: &mut OpenedWorkbook,
    sheet: &SheetRef,
    side: Side,
    opts: &DiffOptions,
    total_cells_read: &mut u64,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<SheetReadResult, SheetsDiffError> {
    let is_1904 = wb.is_1904;
    let mut cells: CellMap = BTreeMap::new();
    let mut range_start: Option<(u32, u32)> = None;
    let mut range_end: Option<(u32, u32)> = None;

    // Local to this call (this side of this sheet), not the cumulative
    // `total_cells_read` accumulator — a mid-sheet cancellation checkpoint
    // every `CANCEL_POLL_INTERVAL` cell records (M7 Handoff 03).
    let mut poll_count: u64 = 0;

    // Pass 1: values. A chart sheet is not a worksheet; `worksheet_range` gave it
    // an empty range, so it has no cells here either.
    {
        let mut reader = match wb.reader.worksheet_cells_reader(&sheet.name) {
            Ok(r) => r,
            Err(calamine::XlsxError::NotAWorksheet(_)) => return Ok((cells, None, None)),
            Err(e) => return Err(SheetsDiffError::read_sheet(side, sheet.clone(), e)),
        };
        while let Some(cell) = reader
            .next_cell()
            .map_err(|e| SheetsDiffError::read_sheet(side, sheet.clone(), e))?
        {
            poll_count += 1;
            if poll_count.is_multiple_of(CANCEL_POLL_INTERVAL) {
                check_cancel(opts)?;
            }
            if matches!(cell.get_value(), calamine::DataRef::Empty) {
                continue;
            }

            let data: calamine::Data = cell.get_value().clone().into();
            let value = normalize_cell_value(&data, is_1904);
            if matches!(value, crate::model::CellValue::Empty) {
                continue;
            }

            // Convert the absolute 0-based position to 1-based coordinates.
            let (row0, col0) = cell.get_position();
            let (row1, col1) = (row0 + 1, col0 + 1);

            // max_cells_read: one more retained cell than the bound allows. Checked *before*
            // the insert, so it fires before the memory it bounds is spent. A repeated
            // address replaces the cell it repeats and retains nothing more, so it is not
            // counted: the figure is the size of the map, not the number of records streamed.
            if !cells.contains_key(&(row1, col1)) {
                if let Some(max) = opts.limits.max_cells_read
                    && *total_cells_read >= max
                {
                    return Err(SheetsDiffError::LimitExceeded {
                        limit: LimitKind::CellsRead,
                        observed: *total_cells_read + 1,
                    });
                }
                *total_cells_read += 1;
            }

            update_bounds(&mut range_start, &mut range_end, row1, col1);
            cells.insert(
                (row1, col1),
                NormalizedCell {
                    value,
                    formula: None,
                },
            );
        }
    }

    // Pass 2: formula text, attached to cells that have a value. Best-effort as
    // before: a read error (other than cancellation) drops every formula for the
    // sheet rather than failing the read. The formulas are collected first and
    // attached only on success, so a failure part-way never leaves a sheet with
    // some of its formulas.
    let mut formulas: Vec<((u32, u32), String)> = Vec::new();
    if let Ok(mut reader) = wb.reader.worksheet_cells_reader(&sheet.name) {
        loop {
            match reader.next_formula() {
                Ok(Some(cell)) => {
                    poll_count += 1;
                    if poll_count.is_multiple_of(CANCEL_POLL_INTERVAL) {
                        check_cancel(opts)?;
                    }
                    if !cell.get_value().is_empty() {
                        let (row0, col0) = cell.get_position();
                        let key = (row0 + 1, col0 + 1);
                        if cells.contains_key(&key) {
                            formulas.push((key, cell.get_value().clone()));
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    formulas.clear();
                    break;
                }
            }
        }
    }
    // f135: whether this sheet genuinely has at least one formula, captured here
    // -- before `formulas` is drained below -- from whether the formula pass
    // actually found one attached to a retained cell. The old `has_formulas`
    // was the formula PASS's own success flag (true on `Ok(None)`, which is
    // ordinary end-of-stream reached by every readable sheet), not whether the
    // sheet has formulas; it was true for essentially every sheet, including
    // ones with none at all. That is the whole reason the per-cell diagnostic
    // below used to fire on plain numeric data: a 20,000x10 sheet of plain
    // numbers, no formula anywhere, measured 400,000 of them and a 158.9 MiB
    // result (f135). A read error still clears `formulas`, so it is empty and
    // this is `false` in that case too, exactly as `has_formulas` was.
    let sheet_has_formulas = !formulas.is_empty();
    // Applied in stream order, so a duplicate address keeps its last formula as
    // `Range::from_sparse` did.
    for (key, text) in formulas {
        if let Some(cell) = cells.get_mut(&key) {
            cell.formula = Some(text);
        }
    }

    // Diagnostic: this sheet has formulas, but not every numeric cell is one --
    // expected, and worth at most Info. f135: previously pushed once per such
    // cell (unbounded by anything), which is excessive even on a sheet that
    // genuinely has formulas -- measured, a realistic 1,000-row mixed sheet (8
    // plain-numeric columns, 2 formula columns) would emit 8,000 of these per
    // side under the old per-cell shape, 6.6 MB of JSON for one sheet. One Info
    // per sheet, carrying the count, is what a caller wants; `address: None`
    // since it is not about one cell. Kept out of `DiagnosticKind`'s payload
    // (a plain-text count, not a field) so this stays within the existing
    // public API -- adding a field to a currently-unit variant is a breaking
    // change under `#[non_exhaustive]`, and this defect fix is not the place
    // for that.
    if sheet_has_formulas && opts.comparison.include_formula_cached_values {
        let count = cells
            .values()
            .filter(|cell| {
                cell.formula.is_none()
                    && matches!(
                        cell.value,
                        crate::model::CellValue::Integer(_) | crate::model::CellValue::Number(_)
                    )
            })
            .count();
        if count > 0 {
            diagnostics.push(Diagnostic {
                severity: Severity::Info,
                kind: DiagnosticKind::FormulaUnavailable,
                location: DiagnosticLocation {
                    stage: DiffStage::Read,
                    sheet_order: Some(sheet.index),
                    sheet_name: Some(sheet.name.clone()),
                    address: None,
                },
                message: format!(
                    "formula text unavailable for {count} numeric cell{} on this sheet",
                    if count == 1 { "" } else { "s" }
                ),
            });
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
