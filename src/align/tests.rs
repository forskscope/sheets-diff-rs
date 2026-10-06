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

// ---------------------------------------------------------------------------
// The alignment invariant (every-row-in-exactly-one-bucket/01)
// ---------------------------------------------------------------------------

/// A deterministic generator, so a failing case reproduces from its seed.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A random sheet over rows `1..=rows` and columns `1..=cols`. Values come from a three-word alphabet,
/// so identical rows, duplicate keys and identical signatures all occur. Roughly 60% of positions hold a
/// cell, so rows with no cell in a sampled column are common.
fn random_sheet(rng: &mut Lcg, rows: u64, cols: u64) -> CellMap {
    let mut cells = CellMap::new();
    for r in 1..=rows {
        for c in 1..=cols {
            if rng.below(100) < 60 {
                let v = ["a", "b", "c"][rng.below(3) as usize];
                cells.insert(
                    (r as u32, c as u32),
                    NormalizedCell::for_test(CellValue::Text(v.to_string())),
                );
            }
        }
    }
    cells
}

/// The values of a sheet, for a failure message: the cell type itself has no `Debug`.
fn values(cells: &CellMap) -> Vec<(u32, u32, CellValue)> {
    cells
        .iter()
        .map(|((r, c), cell)| (*r, *c, cell.value.clone()))
        .collect()
}

/// `old` with a few cells changed, a few rows blanked, and a few cells added: pairings are then common,
/// which is the case the clauses are about.
fn perturb(rng: &mut Lcg, old: &CellMap) -> CellMap {
    let mut new: CellMap = old
        .iter()
        .map(|(k, c)| (*k, NormalizedCell::for_test(c.value.clone())))
        .collect();
    for cell in new.values_mut() {
        if rng.below(100) < 15 {
            cell.value = CellValue::Text(["a", "b", "c"][rng.below(3) as usize].to_string());
        }
    }
    let rows: BTreeSet<u32> = old.keys().map(|(r, _)| *r).collect();
    for r in rows {
        if rng.below(100) < 10 {
            new.retain(|(row, _), _| *row != r);
        }
    }
    for r in 1..=6u32 {
        for c in 1..=3u32 {
            if rng.below(100) < 8 {
                new.insert(
                    (r, c),
                    NormalizedCell::for_test(CellValue::Text("a".to_string())),
                );
            }
        }
    }
    new
}

/// Clause 1 is checked for every mode and every generated sheet pair. The debug assertion inside
/// `compute_row_mapping` panics on a violation, and the explicit check here keeps the test from
/// depending on that alone.
#[test]
fn the_invariant_holds_for_generated_sheets_under_every_mode() {
    let modes = [
        AlignmentMode::RowKey { columns: vec![1] },
        AlignmentMode::RowKey { columns: vec![2] },
        AlignmentMode::RowKey { columns: vec![9] },
        AlignmentMode::RowKey { columns: vec![] },
        AlignmentMode::RowSignature {
            sample_columns: Some(vec![1]),
        },
        AlignmentMode::RowSignature {
            sample_columns: Some(vec![2, 3]),
        },
        AlignmentMode::RowSignature {
            sample_columns: Some(vec![9]),
        },
        AlignmentMode::RowSignature {
            sample_columns: None,
        },
    ];
    let mut rng = Lcg(0x5eed_0bad_cafe_f00d);
    let mut checked = 0usize;
    for case in 0..1500 {
        let old_rows = rng.below(7);
        let old = random_sheet(&mut rng, old_rows, 3);
        let new = if rng.below(2) == 0 {
            let new_rows = rng.below(7);
            random_sheet(&mut rng, new_rows, 3)
        } else {
            perturb(&mut rng, &old)
        };
        for mode in &modes {
            let mut diag = vec![];
            let mapping = compute_row_mapping(&old, &new, mode, None, None, &sheet(), &mut diag)
                .unwrap_or_else(|e| panic!("case {case}, {mode:?}: {e:?}"));
            if let Some(m) = mapping {
                checked += 1;
                // The reasons promise: no reason appears twice, and `Exact` carries none.
                for (i, reason) in m.summary.reasons.iter().enumerate() {
                    assert!(
                        !m.summary.reasons[..i].contains(reason),
                        "case {case}, mode {mode:?}: {reason:?} repeated in {:?}",
                        m.summary.reasons
                    );
                }
                if m.summary.confidence == MatchConfidence::Exact {
                    assert!(m.summary.reasons.is_empty(), "case {case}, mode {mode:?}");
                }
                if m.summary.confidence == MatchConfidence::Medium {
                    assert!(
                        !m.summary.reasons.is_empty(),
                        "case {case}, mode {mode:?}: Medium with no reason"
                    );
                }
                assert_eq!(
                    row_mapping_violation(&old, &new, &m),
                    None,
                    "case {case}, mode {mode:?}\nold: {:?}\nnew: {:?}",
                    values(&old),
                    values(&new)
                );
            }
        }
    }
    assert!(checked > 5_000, "only {checked} mappings were produced");
}

/// The bound-exceeded path returns no mapping, so there is nothing to check and nothing may fire.
#[test]
fn bound_exceeded_returns_no_mapping_so_nothing_is_checked() {
    let old = make_cells(&[(1, 1, "a"), (2, 1, "b")]);
    let new = make_cells(&[(1, 1, "b"), (2, 1, "a")]);
    let mut diag = vec![];
    let mapping = compute_row_mapping(
        &old,
        &new,
        &AlignmentMode::RowKey { columns: vec![1] },
        Some(1),
        None,
        &sheet(),
        &mut diag,
    )
    .unwrap();
    assert!(mapping.is_none());
}

fn mapping(matched: &[(u32, u32)], removed: &[u32], inserted: &[u32]) -> RowMapping {
    let matched: BTreeMap<u32, u32> = matched.iter().copied().collect();
    RowMapping {
        summary: AlignmentSummaryData {
            inserted_rows: inserted.len(),
            removed_rows: removed.len(),
            matched_rows: matched.len(),
            confidence: MatchConfidence::Medium,
            reasons: Vec::new(),
        },
        matched,
        removed: removed.to_vec(),
        inserted: inserted.to_vec(),
    }
}

/// Two old rows and two new rows, each with a cell in column 1.
fn two_by_two() -> (CellMap, CellMap) {
    (
        make_cells(&[(1, 1, "a"), (2, 1, "b")]),
        make_cells(&[(1, 1, "a"), (2, 1, "b")]),
    )
}

fn assert_clause(violation: Option<String>, clause: &str) {
    let v = violation.unwrap_or_else(|| panic!("no violation reported; wanted {clause}"));
    assert!(v.contains(clause), "wanted {clause}, got: {v}");
}

#[test]
fn clause_1_old_side_total_catches_a_row_in_no_bucket() {
    let (old, new) = two_by_two();
    let m = mapping(&[(1, 1)], &[], &[2]);
    assert_clause(
        row_mapping_violation(&old, &new, &m),
        "clause 1 (old side, total): old row 2",
    );
}

#[test]
fn clause_1_old_side_disjoint_catches_a_row_in_two_buckets() {
    let (old, new) = two_by_two();
    let m = mapping(&[(1, 1), (2, 2)], &[2], &[]);
    assert_clause(
        row_mapping_violation(&old, &new, &m),
        "clause 1 (old side, disjoint): old row 2",
    );
}

#[test]
fn clause_2_new_side_total_catches_a_row_in_no_bucket() {
    let (old, new) = two_by_two();
    let m = mapping(&[(1, 1)], &[2], &[]);
    assert_clause(
        row_mapping_violation(&old, &new, &m),
        "clause 2 (new side, total): new row 2",
    );
}

#[test]
fn clause_2_new_side_disjoint_catches_a_row_in_two_buckets() {
    let (old, new) = two_by_two();
    let m = mapping(&[(1, 1), (2, 2)], &[], &[1]);
    assert_clause(
        row_mapping_violation(&old, &new, &m),
        "clause 2 (new side, disjoint): new row 1",
    );
}

#[test]
fn clause_3_catches_an_invented_old_row() {
    let (old, new) = two_by_two();
    let m = mapping(&[(1, 1), (2, 2)], &[9], &[]);
    assert_clause(
        row_mapping_violation(&old, &new, &m),
        "clause 3 (no row is invented): old row 9",
    );
}

#[test]
fn clause_3_catches_an_invented_new_row() {
    let (old, new) = two_by_two();
    let m = mapping(&[(1, 1), (2, 2)], &[], &[7]);
    assert_clause(
        row_mapping_violation(&old, &new, &m),
        "clause 3 (no row is invented): new row 7",
    );
}

/// Clause 4 is hand-built here. The engine cannot reach it: LCS advances both indices on every match, so
/// no new row can be matched twice. That is evidence the clause holds by construction, and this test is
/// evidence that the checker would catch it if that stopped being true.
#[test]
fn clause_4_catches_two_old_rows_matched_to_one_new_row() {
    let (old, new) = two_by_two();
    let m = mapping(&[(1, 1), (2, 1)], &[], &[2]);
    assert_clause(
        row_mapping_violation(&old, &new, &m),
        "clause 4 (matched is injective): old rows 1 and 2 are both matched to new row 1",
    );
}
