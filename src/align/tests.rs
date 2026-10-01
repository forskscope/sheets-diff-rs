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
