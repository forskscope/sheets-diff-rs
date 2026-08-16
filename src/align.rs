//! Optional row alignment to reduce false-positive cascades after row
//! insertions and deletions (RFC-011).
//!
//! Default mode is `Positional` (existing behaviour, unchanged).
//! `RowKey` and `RowSignature` modes are opt-in via `DiffOptions.matching.alignment`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::model::{
    CellValue, Diagnostic, DiagnosticKind, DiagnosticLocation, DiffStage, MatchConfidence, Severity,
};
use crate::options::AlignmentMode;

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
// CellMap type alias (mirrors diff.rs internal)
// ---------------------------------------------------------------------------

pub type AlignCellMap = BTreeMap<(u32, u32), CellValue>;

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
pub fn compute_row_mapping(
    old_cells: &AlignCellMap,
    new_cells: &AlignCellMap,
    mode: &AlignmentMode,
    max_alignment_product: Option<u64>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<RowMapping> {
    if matches!(mode, AlignmentMode::Positional) {
        return None;
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
                location: DiagnosticLocation {
                    stage: DiffStage::Compare,
                    sheet_order: None,
                    sheet_name: None,
                    address: None,
                },
                message: format!(
                    "alignment row product ({old_rows} old x {new_rows} new = {product}) \
                     exceeds max_alignment_product ({limit}); sheet compared positionally instead"
                ),
            });
            return None;
        }
    }

    match mode {
        AlignmentMode::Positional => None,

        AlignmentMode::RowKey { columns } => Some(row_key_alignment(
            old_cells,
            new_cells,
            columns,
            diagnostics,
        )),

        AlignmentMode::RowSignature { sample_columns } => Some(row_signature_alignment(
            old_cells,
            new_cells,
            sample_columns.as_deref(),
            diagnostics,
        )),

        AlignmentMode::HeaderColumn => {
            Some(header_column_alignment(old_cells, new_cells, diagnostics))
        }
    }
}

/// Number of distinct 1-based row indices with at least one cell present.
fn distinct_row_count(cells: &AlignCellMap) -> u64 {
    cells.keys().map(|(r, _)| *r).collect::<BTreeSet<_>>().len() as u64
}

// ---------------------------------------------------------------------------
// Row-key alignment
// ---------------------------------------------------------------------------

fn row_key_alignment(
    old_cells: &AlignCellMap,
    new_cells: &AlignCellMap,
    key_cols: &[u32],
    diagnostics: &mut Vec<Diagnostic>,
) -> RowMapping {
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
            location: DiagnosticLocation {
                stage: DiffStage::Compare,
                sheet_order: None,
                sheet_name: None,
                address: None,
            },
            message: format!(
                "duplicate alignment keys detected ({} distinct key(s) repeated in old, \
                 {} in new); LCS matching may pair rows ambiguously among duplicates",
                old_dups.len(),
                new_dups.len()
            ),
        });
    }

    lcs_match(old_keys, new_keys)
}

// ---------------------------------------------------------------------------
// Row-signature alignment
// ---------------------------------------------------------------------------

fn row_signature_alignment(
    old_cells: &AlignCellMap,
    new_cells: &AlignCellMap,
    sample_cols: Option<&[u32]>,
    _diagnostics: &mut Vec<Diagnostic>,
) -> RowMapping {
    let old_sigs = compute_row_signatures(old_cells, sample_cols);
    let new_sigs = compute_row_signatures(new_cells, sample_cols);
    lcs_match(old_sigs, new_sigs)
}

// ---------------------------------------------------------------------------
// Header-column alignment
// ---------------------------------------------------------------------------

fn header_column_alignment(
    old_cells: &AlignCellMap,
    new_cells: &AlignCellMap,
    diagnostics: &mut Vec<Diagnostic>,
) -> RowMapping {
    // Treat row 1 as the header; use the header values as column identity.
    // Fall back to RowSignature for data rows.
    let key_col: Vec<u32> = vec![1]; // row-1 = header row; match data by that col
    row_key_alignment(old_cells, new_cells, &key_col, diagnostics)
}

// ---------------------------------------------------------------------------
// LCS-based row matching
// ---------------------------------------------------------------------------

/// Match rows using patience-LCS on their key/signature sequences.
/// Returns a `RowMapping` with the matched, inserted, and removed rows.
fn lcs_match(old_seq: BTreeMap<u32, RowKey>, new_seq: BTreeMap<u32, RowKey>) -> RowMapping {
    let old_rows: Vec<(u32, RowKey)> = old_seq.into_iter().collect();
    let new_rows: Vec<(u32, RowKey)> = new_seq.into_iter().collect();

    // Build LCS table.
    let m = old_rows.len();
    let n = new_rows.len();
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
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
    let confidence = if removed.is_empty() && inserted.is_empty() {
        MatchConfidence::Exact
    } else if n_matched > removed.len() + inserted.len() {
        MatchConfidence::High
    } else {
        MatchConfidence::Medium
    };

    let n_removed = removed.len();
    let n_inserted = inserted.len();
    RowMapping {
        matched,
        removed,
        inserted,
        summary: AlignmentSummaryData {
            inserted_rows: n_inserted,
            removed_rows: n_removed,
            matched_rows: n_matched,
            confidence,
        },
    }
}

// ---------------------------------------------------------------------------
// Key / signature extraction helpers
// ---------------------------------------------------------------------------

type RowKey = Vec<String>;

fn extract_row_keys(cells: &AlignCellMap, key_cols: &[u32]) -> BTreeMap<u32, RowKey> {
    let mut rows: BTreeMap<u32, RowKey> = BTreeMap::new();
    for col in key_cols {
        // Collect all rows that have a value in this key column.
        for ((r, c), val) in cells {
            if c == col {
                let entry = rows.entry(*r).or_default();
                entry.push(val.display_string());
            }
        }
    }
    rows
}

fn compute_row_signatures(
    cells: &AlignCellMap,
    sample_cols: Option<&[u32]>,
) -> BTreeMap<u32, RowKey> {
    let mut rows: BTreeMap<u32, RowKey> = BTreeMap::new();
    for ((r, c), val) in cells {
        if let Some(cols) = sample_cols
            && !cols.contains(c)
        {
            continue;
        }
        rows.entry(*r)
            .or_default()
            .push(format!("{c}:{}", val.display_string()));
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

    fn make_cells(data: &[(u32, u32, &str)]) -> AlignCellMap {
        data.iter()
            .map(|(r, c, v)| ((*r, *c), CellValue::Text(v.to_string())))
            .collect()
    }

    #[test]
    fn positional_mode_returns_none() {
        let cells = make_cells(&[(1, 1, "a")]);
        let mut diag = vec![];
        let result =
            compute_row_mapping(&cells, &cells, &AlignmentMode::Positional, None, &mut diag);
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
            &mut diag,
        )
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
            &mut diag,
        )
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
            &mut diag,
        )
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
            &mut diag,
        );
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
            &mut diag,
        );
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
            &mut diag,
        );
        assert!(result.is_some());
        assert!(
            !diag
                .iter()
                .any(|d| matches!(d.kind, DiagnosticKind::AlignmentBoundExceeded { .. })),
            "bound was not exceeded, should not have fired"
        );
    }
}
