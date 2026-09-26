//! Optional row alignment to reduce false-positive cascades after row
//! insertions and deletions (RFC-011).
//!
//! Default mode is `Positional` (existing behaviour, unchanged).
//! `RowKey` and `RowSignature` modes are opt-in via `DiffOptions.matching.alignment`.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt::Write as _;

use crate::diff::CellMap;
use crate::error::SheetsDiffError;
use crate::model::{
    Diagnostic, DiagnosticKind, DiagnosticLocation, DiffStage, MatchConfidence, Severity, SheetRef,
};
use crate::options::{AlignmentMode, Cancellation};

// Re-exported in model.rs; defined here to keep alignment logic co-located.

// ---------------------------------------------------------------------------
// Public summary (populates the reserved SheetDiff.alignment_summary)
// ---------------------------------------------------------------------------

/// Populated when alignment mode is not `Positional`.
#[derive(Clone, Debug)]
pub struct AlignmentSummaryData {
    pub inserted_rows: usize,
    pub removed_rows: usize,
    pub matched_rows: usize,
    pub confidence: MatchConfidence,
}

// ---------------------------------------------------------------------------
// Coordinate mapping produced by alignment
// ---------------------------------------------------------------------------

/// Maps old 1-based row indices to new 1-based row indices for a sheet pair.
/// Rows absent from the map are inserted (new only) or removed (old only).
pub struct RowMapping {
    /// old_row → new_row for matched pairs.
    pub matched: BTreeMap<u32, u32>,
    /// Rows in the old sheet with no match (removed).
    pub removed: Vec<u32>,
    /// Rows in the new sheet with no match (inserted).
    pub inserted: Vec<u32>,
    pub summary: AlignmentSummaryData,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Compute a row mapping under the configured `AlignmentMode`.
///
/// Returns `None` when mode is `Positional` (caller uses identity mapping),
/// or when the `old_rows * new_rows` product would exceed
/// `max_alignment_product` — RFC-035 §5.2: alignment degrades to positional
/// in that case, it never errors. The bound is checked here, before any
/// mode-specific work, using the distinct row counts across the full cell
/// maps; the sequences a mode actually builds are always a subset of those
/// rows, so this is a conservative (never-too-low) estimate of the LCS
/// matrix a mode would allocate.
/// `cancellation` is polled once per row of the LCS table's fill — the only phase that is quadratic in
/// rows — and returns `Err(Cancelled)` from there. The phases around it are linear in data already read
/// and are not polled (M-measured; see `docs/src/maintainers/performance.md`).
/// `sheet` is the sheet being aligned, and is what every diagnostic raised here names in its
/// location: the new workbook's side of the pair, or the old one's when the sheet exists only
/// there — the label the renderer uses. Alignment warnings are about *the sheet*, and this is the
/// only place that knows which one.
pub fn compute_row_mapping(
    old_cells: &CellMap,
    new_cells: &CellMap,
    mode: &AlignmentMode,
    max_alignment_product: Option<u64>,
    cancellation: Option<&dyn Cancellation>,
    sheet: &SheetRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Option<RowMapping>, SheetsDiffError> {
    if matches!(mode, AlignmentMode::Positional) {
        return Ok(None);
    }

    if let Some(limit) = max_alignment_product {
        let old_rows = distinct_row_count(old_cells);
        let new_rows = distinct_row_count(new_cells);
        let product = old_rows.saturating_mul(new_rows);
        if product > limit {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                kind: DiagnosticKind::AlignmentBoundExceeded {
                    limit,
                    observed: product,
                },
                location: sheet_location(sheet),
                message: format!(
                    "alignment row product ({old_rows} old x {new_rows} new = {product}) \
                     exceeds max_alignment_product ({limit}); sheet compared positionally instead"
                ),
            });
            return Ok(None);
        }
    }

    match mode {
        AlignmentMode::Positional => Ok(None),

        AlignmentMode::RowKey { columns } => Ok(Some(row_key_alignment(
            old_cells,
            new_cells,
            columns,
            cancellation,
            sheet,
            diagnostics,
        )?)),

        AlignmentMode::RowSignature { sample_columns } => Ok(Some(row_signature_alignment(
            old_cells,
            new_cells,
            sample_columns.as_deref(),
            cancellation,
            diagnostics,
        )?)),
    }
}

/// `Err(Cancelled)` if a cancellation token is configured and has fired.
///
/// This module has its own three-line helper rather than reaching for `diff.rs`'s `check_cancel`,
/// which takes the whole `&DiffOptions`: alignment is passed what it needs (the token) and nothing
/// more, as it is passed `max_alignment_product` rather than the options tree.
fn check_cancelled(cancellation: Option<&dyn Cancellation>) -> Result<(), SheetsDiffError> {
    match cancellation {
        Some(c) if c.is_cancelled() => Err(SheetsDiffError::Cancelled),
        _ => Ok(()),
    }
}

/// The location of a diagnostic about `sheet` itself: both `sheet_order` and `sheet_name`, never
/// one without the other, and no cell address (the warning is about the sheet, not a cell).
fn sheet_location(sheet: &SheetRef) -> DiagnosticLocation {
    DiagnosticLocation {
        stage: DiffStage::Compare,
        sheet_order: Some(sheet.index),
        sheet_name: Some(sheet.name.clone()),
        address: None,
    }
}

/// Number of distinct 1-based row indices with at least one cell present.
fn distinct_row_count(cells: &CellMap) -> u64 {
    cells.keys().map(|(r, _)| *r).collect::<BTreeSet<_>>().len() as u64
}

// ---------------------------------------------------------------------------
// Row-key alignment
// ---------------------------------------------------------------------------

fn row_key_alignment(
    old_cells: &CellMap,
    new_cells: &CellMap,
    key_cols: &[u32],
    cancellation: Option<&dyn Cancellation>,
    sheet: &SheetRef,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<RowMapping, SheetsDiffError> {
    let old_keys = extract_row_keys(old_cells, key_cols);
    let new_keys = extract_row_keys(new_cells, key_cols);

    // Detect duplicate keys — emit a warning. LCS still runs on the full
    // sequences (duplicates included); it does not fall back to positional
    // for just the affected rows, so the message must not claim it does.
    let old_dups = find_duplicate_keys(&old_keys);
    let new_dups = find_duplicate_keys(&new_keys);
    if !old_dups.is_empty() || !new_dups.is_empty() {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            kind: DiagnosticKind::DuplicateAlignmentKey {
                old_count: old_dups.len(),
                new_count: new_dups.len(),
            },
            location: sheet_location(sheet),
            message: format!(
                "duplicate alignment keys detected ({} distinct key(s) repeated in old, \
                 {} in new); LCS matching may pair rows ambiguously among duplicates",
                old_dups.len(),
                new_dups.len()
            ),
        });
    }

    // A row with no cell in any key column has no key, so it cannot be matched *by key*. What it
    // must not do is disappear: `extract_row_keys` never sees it, so without this it would be in
    // none of `matched`, `removed` or `inserted`, its cells would never be compared, and the
    // summary would call the alignment exact (f130).
    //
    // Two things happen to such rows instead. **Rows whose cells hold the same values in the same
    // columns on both sides are paired with one another, in row order** — a subtotal or spacer line
    // that did not change must not become a removal plus an insertion, or the noise would scale with
    // how many blank keys the sheet has (2,200 cell diffs for one changed cell on a 2,000-row sheet
    // with a blank key every twentieth row). Which of several identical rows pairs with which has no
    // observable effect: identical rows compare identically whichever way they are paired. **The rest
    // are unmatched** — removed on the old side, inserted on the new — so a keyless row that changed
    // reaches the comparison as a whole-row change and is not lost. Pairing a *changed* keyless row
    // with its counterpart would be better still and needs a design (which neighbour; what when the
    // counts differ between sides); it is not done here.
    let old_keyless = keyless_rows(old_cells, &old_keys);
    let new_keyless = keyless_rows(new_cells, &new_keys);

    let mut mapping = lcs_match(old_keys, new_keys, cancellation)?;

    if !old_keyless.is_empty() || !new_keyless.is_empty() {
        let (old_count, new_count) = (old_keyless.len(), new_keyless.len());
        let (paired, old_rest, new_rest) =
            pair_identical_rows(old_cells, new_cells, old_keyless, new_keyless);
        let n_paired = paired.len();

        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            kind: DiagnosticKind::MissingAlignmentKey {
                old_count,
                new_count,
            },
            location: sheet_location(sheet),
            message: format!(
                "{old_count} row(s) in old and {new_count} in new have no cell in any alignment key \
                 column and cannot be matched by key; {n_paired} pair(s) of rows with identical \
                 values were matched to each other, and the other {} old / {} new row(s) are reported \
                 as removed / inserted rather than compared with a counterpart",
                old_rest.len(),
                new_rest.len()
            ),
        });

        mapping.matched.extend(paired);
        mapping.removed.extend(old_rest);
        mapping.removed.sort_unstable();
        mapping.inserted.extend(new_rest);
        mapping.inserted.sort_unstable();

        let (n_matched, n_removed, n_inserted) = (
            mapping.matched.len(),
            mapping.removed.len(),
            mapping.inserted.len(),
        );
        mapping.summary = AlignmentSummaryData {
            inserted_rows: n_inserted,
            removed_rows: n_removed,
            matched_rows: n_matched,
            // `High` says the pairing is reliable apart from a few real insertions and removals.
            // Rows placed by their content alone, or not at all, are neither — so a sheet with any
            // keyless row is at most `Medium`, however many of them paired.
            confidence: match confidence_for(n_matched, n_removed, n_inserted) {
                MatchConfidence::Exact | MatchConfidence::High => MatchConfidence::Medium,
                lower => lower,
            },
        };
    }

    Ok(mapping)
}

// ---------------------------------------------------------------------------
// Row-signature alignment
// ---------------------------------------------------------------------------

fn row_signature_alignment(
    old_cells: &CellMap,
    new_cells: &CellMap,
    sample_cols: Option<&[u32]>,
    cancellation: Option<&dyn Cancellation>,
    _diagnostics: &mut Vec<Diagnostic>,
) -> Result<RowMapping, SheetsDiffError> {
    let old_sigs = compute_row_signatures(old_cells, sample_cols);
    let new_sigs = compute_row_signatures(new_cells, sample_cols);
    lcs_match(old_sigs, new_sigs, cancellation)
}

// ---------------------------------------------------------------------------
// LCS-based row matching
// ---------------------------------------------------------------------------

/// Match rows using patience-LCS on their key/signature sequences.
/// Returns a `RowMapping` with the matched, inserted, and removed rows.
fn lcs_match(
    old_seq: BTreeMap<u32, RowKey>,
    new_seq: BTreeMap<u32, RowKey>,
    cancellation: Option<&dyn Cancellation>,
) -> Result<RowMapping, SheetsDiffError> {
    let old_rows: Vec<(u32, RowKey)> = old_seq.into_iter().collect();
    let new_rows: Vec<(u32, RowKey)> = new_seq.into_iter().collect();

    // Build LCS table.
    let m = old_rows.len();
    let n = new_rows.len();
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
        // The poll: once per row of the table, before its `n` cells — `m` polls in all, one atomic
        // load per `n` cell operations. Not per cell: the table is quadratic and nobody needs
        // granularity finer than a row.
        check_cancelled(cancellation)?;
        for j in (0..n).rev() {
            if old_rows[i].1 == new_rows[j].1 {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    // Trace back.
    let mut matched: BTreeMap<u32, u32> = BTreeMap::new();
    let mut old_used = vec![false; m];
    let mut new_used = vec![false; n];
    let (mut i, mut j) = (0, 0);
    while i < m && j < n {
        if old_rows[i].1 == new_rows[j].1 {
            matched.insert(old_rows[i].0, new_rows[j].0);
            old_used[i] = true;
            new_used[j] = true;
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }

    let removed: Vec<u32> = old_rows
        .iter()
        .enumerate()
        .filter(|(idx, _)| !old_used[*idx])
        .map(|(_, (r, _))| *r)
        .collect();
    let inserted: Vec<u32> = new_rows
        .iter()
        .enumerate()
        .filter(|(idx, _)| !new_used[*idx])
        .map(|(_, (r, _))| *r)
        .collect();

    let n_matched = matched.len();
    let confidence = confidence_for(n_matched, removed.len(), inserted.len());

    let n_removed = removed.len();
    let n_inserted = inserted.len();
    Ok(RowMapping {
        matched,
        removed,
        inserted,
        summary: AlignmentSummaryData {
            inserted_rows: n_inserted,
            removed_rows: n_removed,
            matched_rows: n_matched,
            confidence,
        },
    })
}

/// `Exact` when every row on both sides was matched; `High` when matched rows outnumber the rest;
/// `Medium` otherwise.
fn confidence_for(n_matched: usize, n_removed: usize, n_inserted: usize) -> MatchConfidence {
    if n_removed == 0 && n_inserted == 0 {
        MatchConfidence::Exact
    } else if n_matched > n_removed + n_inserted {
        MatchConfidence::High
    } else {
        MatchConfidence::Medium
    }
}

// ---------------------------------------------------------------------------
// Key / signature extraction helpers
// ---------------------------------------------------------------------------

type RowKey = Vec<String>;

fn extract_row_keys(cells: &CellMap, key_cols: &[u32]) -> BTreeMap<u32, RowKey> {
    let mut rows: BTreeMap<u32, RowKey> = BTreeMap::new();
    for col in key_cols {
        // Collect all rows that have a value in this key column.
        for ((r, c), cell) in cells {
            if c == col {
                let entry = rows.entry(*r).or_default();
                entry.push(cell.value.display_string());
            }
        }
    }
    rows
}

/// The rows of `cells` that `extract_row_keys` did not key: those with a cell, but none in any key
/// column. In ascending row order. (A row with no cell at all is not in `cells` and is not a row
/// of this sheet as far as any comparison is concerned.)
fn keyless_rows(cells: &CellMap, keyed: &BTreeMap<u32, RowKey>) -> Vec<u32> {
    let mut rows: Vec<u32> = Vec::new();
    for (r, _) in cells.keys() {
        if !keyed.contains_key(r) && rows.last() != Some(r) {
            rows.push(*r);
        }
    }
    rows
}

/// What a row holds, for pairing keyless rows: each cell's column and value, in column order. Rows
/// with equal strings hold the same values in the same columns. (`Debug`, not `display_string()`,
/// so the number `1` and the text `"1"` are different; formulas are not part of it — a paired row is
/// still compared cell by cell, formulas included, so pairing never hides a difference.)
fn row_content(cells: &CellMap, row: u32) -> String {
    let mut s = String::new();
    for ((_, col), cell) in cells.range((row, 0)..=(row, u32::MAX)) {
        let _ = write!(s, "{col}={:?};", cell.value);
    }
    s
}

/// Pair keyless rows whose content is identical, the k-th such row on the old side with the k-th on
/// the new, in row order. Returns the pairs (old row -> new row) and the rows left over on each side.
fn pair_identical_rows(
    old_cells: &CellMap,
    new_cells: &CellMap,
    old_keyless: Vec<u32>,
    new_keyless: Vec<u32>,
) -> (BTreeMap<u32, u32>, Vec<u32>, Vec<u32>) {
    let mut available: HashMap<String, VecDeque<u32>> = HashMap::new();
    for r in new_keyless {
        available
            .entry(row_content(new_cells, r))
            .or_default()
            .push_back(r);
    }
    let mut paired = BTreeMap::new();
    let mut old_rest = Vec::new();
    for r in old_keyless {
        match available
            .get_mut(&row_content(old_cells, r))
            .and_then(VecDeque::pop_front)
        {
            Some(n) => {
                paired.insert(r, n);
            }
            None => old_rest.push(r),
        }
    }
    let mut new_rest: Vec<u32> = available.into_values().flatten().collect();
    new_rest.sort_unstable();
    (paired, old_rest, new_rest)
}

fn compute_row_signatures(cells: &CellMap, sample_cols: Option<&[u32]>) -> BTreeMap<u32, RowKey> {
    let mut rows: BTreeMap<u32, RowKey> = BTreeMap::new();
    for ((r, c), cell) in cells {
        if let Some(cols) = sample_cols
            && !cols.contains(c)
        {
            continue;
        }
        rows.entry(*r)
            .or_default()
            .push(format!("{c}:{}", cell.value.display_string()));
    }
    rows
}

fn find_duplicate_keys(keys: &BTreeMap<u32, RowKey>) -> Vec<RowKey> {
    let mut seen: HashMap<&RowKey, usize> = HashMap::new();
    for k in keys.values() {
        *seen.entry(k).or_insert(0) += 1;
    }
    seen.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(k, _)| k.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::NormalizedCell;
    use crate::model::CellValue;

    fn make_cells(data: &[(u32, u32, &str)]) -> CellMap {
        data.iter()
            .map(|(r, c, v)| {
                (
                    (*r, *c),
                    NormalizedCell::for_test(CellValue::Text(v.to_string())),
                )
            })
            .collect()
    }

    fn sheet() -> SheetRef {
        SheetRef {
            name: "Sheet1".into(),
            index: 0,
        }
    }

    #[test]
    fn positional_mode_returns_none() {
        let cells = make_cells(&[(1, 1, "a")]);
        let mut diag = vec![];
        let result = compute_row_mapping(
            &cells,
            &cells,
            &AlignmentMode::Positional,
            None,
            None,
            &sheet(),
            &mut diag,
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn row_key_identity_match() {
        let old = make_cells(&[(1, 1, "id1"), (2, 1, "id2"), (3, 1, "id3")]);
        let new = make_cells(&[(1, 1, "id1"), (2, 1, "id2"), (3, 1, "id3")]);
        let mut diag = vec![];
        let mapping = compute_row_mapping(
            &old,
            &new,
            &AlignmentMode::RowKey { columns: vec![1] },
            None,
            None,
            &sheet(),
            &mut diag,
        )
        .unwrap()
        .unwrap();
        assert_eq!(mapping.matched.len(), 3);
        assert!(mapping.removed.is_empty());
        assert!(mapping.inserted.is_empty());
    }

    #[test]
    fn row_key_detects_inserted_row() {
        // old: id1, id2, id3 — new: id1, id_new, id2, id3 (one inserted at row 2)
        let old = make_cells(&[(1, 1, "id1"), (2, 1, "id2"), (3, 1, "id3")]);
        let new = make_cells(&[
            (1, 1, "id1"),
            (2, 1, "id_new"),
            (3, 1, "id2"),
            (4, 1, "id3"),
        ]);
        let mut diag = vec![];
        let mapping = compute_row_mapping(
            &old,
            &new,
            &AlignmentMode::RowKey { columns: vec![1] },
            None,
            None,
            &sheet(),
            &mut diag,
        )
        .unwrap()
        .unwrap();
        // id1/id2/id3 all matched; id_new is inserted
        assert_eq!(mapping.matched.len(), 3);
        assert_eq!(mapping.inserted.len(), 1);
        assert!(mapping.removed.is_empty());
    }

    #[test]
    fn row_key_detects_removed_row() {
        let old = make_cells(&[(1, 1, "id1"), (2, 1, "id2"), (3, 1, "id3")]);
        let new = make_cells(&[(1, 1, "id1"), (2, 1, "id3")]);
        let mut diag = vec![];
        let mapping = compute_row_mapping(
            &old,
            &new,
            &AlignmentMode::RowKey { columns: vec![1] },
            None,
            None,
            &sheet(),
            &mut diag,
        )
        .unwrap()
        .unwrap();
        assert_eq!(mapping.matched.len(), 2); // id1, id3
        assert_eq!(mapping.removed.len(), 1); // id2
        assert!(mapping.inserted.is_empty());
    }

    #[test]
    fn duplicate_keys_produce_diagnostic() {
        let old = make_cells(&[(1, 1, "dup"), (2, 1, "dup"), (3, 1, "unique")]);
        let new = make_cells(&[(1, 1, "dup"), (2, 1, "unique")]);
        let mut diag = vec![];
        let _ = compute_row_mapping(
            &old,
            &new,
            &AlignmentMode::RowKey { columns: vec![1] },
            None,
            None,
            &sheet(),
            &mut diag,
        )
        .unwrap();
        assert!(!diag.is_empty(), "expected diagnostic for duplicate keys");
        assert!(
            diag.iter()
                .any(|d| matches!(d.kind, DiagnosticKind::DuplicateAlignmentKey { .. })),
            "expected DuplicateAlignmentKey, got {diag:?}"
        );
    }

    #[test]
    fn alignment_bound_exceeded_degrades_to_positional_not_error() {
        // 3 old rows x 3 new rows = product 9, bound of 5 is exceeded.
        let old = make_cells(&[(1, 1, "id1"), (2, 1, "id2"), (3, 1, "id3")]);
        let new = make_cells(&[(1, 1, "id1"), (2, 1, "id2"), (3, 1, "id3")]);
        let mut diag = vec![];
        let result = compute_row_mapping(
            &old,
            &new,
            &AlignmentMode::RowKey { columns: vec![1] },
            Some(5),
            None,
            &sheet(),
            &mut diag,
        )
        .unwrap();
        // Degrades to None (caller's true-positional path) — never an error.
        assert!(result.is_none());
        assert!(
            diag.iter().any(|d| matches!(
                d.kind,
                DiagnosticKind::AlignmentBoundExceeded {
                    limit: 5,
                    observed: 9
                }
            )),
            "expected AlignmentBoundExceeded {{ limit: 5, observed: 9 }}, got {diag:?}"
        );
    }

    #[test]
    fn alignment_bound_within_limit_still_aligns() {
        let old = make_cells(&[(1, 1, "id1"), (2, 1, "id2"), (3, 1, "id3")]);
        let new = make_cells(&[(1, 1, "id1"), (2, 1, "id2"), (3, 1, "id3")]);
        let mut diag = vec![];
        let result = compute_row_mapping(
            &old,
            &new,
            &AlignmentMode::RowKey { columns: vec![1] },
            Some(9), // product is exactly 9 — must not exceed
            None,
            &sheet(),
            &mut diag,
        )
        .unwrap();
        assert!(result.is_some());
        assert!(
            !diag
                .iter()
                .any(|d| matches!(d.kind, DiagnosticKind::AlignmentBoundExceeded { .. })),
            "bound was not exceeded, should not have fired"
        );
    }
}
