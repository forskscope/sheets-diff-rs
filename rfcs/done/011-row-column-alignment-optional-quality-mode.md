# RFC-011: Row/Column Alignment Optional Quality Mode

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.1 candidate, optional v2.0 if ready  
**Created:** 2026-06-11  
**Category:** Diff quality  

## 1. Summary

Provide an opt-in alignment mode to reduce false-positive cascades after row or column insertion/deletion.

## 2. Motivation

Position-only comparison is simple and deterministic, but inserting a row near the top can mark every following row as changed. Premium spreadsheet diff tools reduce this by aligning rows/columns. v2 should design this as an optional mode with clear bounds, not as an always-on default.

## 3. Goals

- Support positional mode as the default.
- Design row alignment based on key columns or row signatures.
- Design column alignment based on header names or column signatures.
- Expose alignment decisions in diagnostics or metadata.
- Bound runtime and memory use.

## 4. Non-goals

- Do not guarantee perfect semantic matching.
- Do not hide true changes to reduce diff size.
- Do not enable expensive alignment by default in v2.0.

## 5. External design

Options:

```rust
pub enum AlignmentMode {
    Positional,
    RowKey { columns: Vec<ColIndex> },
    RowSignature { sample_columns: Option<Vec<ColIndex>> },
    HeaderColumn,
    RowAndColumn { row: RowAlignment, column: ColumnAlignment },
}
```

Public result metadata:

```rust
pub struct AlignmentSummary {
    pub mode: AlignmentModeSummary,
    pub inserted_rows: usize,
    pub removed_rows: usize,
    pub moved_or_matched_rows: usize,
    pub confidence: MatchConfidence,
}
```

## 6. Internal design

Suggested internal approach for row-key mode:

1. Extract row keys from configured columns.
2. Build sequences of row keys for old and new.
3. Run LCS or patience-diff style matching on keys.
4. Compare matched rows by aligned coordinate mapping.
5. Emit inserted/removed row diagnostics or metadata.

For row-signature mode, compute a stable hash/signature from selected normalized cell values. Avoid display strings except as fallback.

## 7. Data lifecycle

1. Sheet pair enters alignment stage.
2. Alignment mode decides coordinate mapping.
3. Cell comparison receives a mapping rather than raw positional coordinates.
4. Output addresses still refer to old/new actual coordinates; UI must know when rows were aligned.
5. Alignment summary is attached to sheet result.

## 8. Error, diagnostic, and edge-case behavior

Duplicate keys, missing keys, and unstable signatures should produce warnings and fall back to positional comparison for ambiguous sections.

Large sheets must respect max row/cell bounds. Alignment cancellation points are mandatory.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Inserting one row at top does not report every following row as modified under key alignment.
- Duplicate key ambiguity is detected.
- Positional mode output remains unchanged.
- Alignment can be cancelled.
- Runtime is documented and tested on large generated fixtures.

## 10. Migration and compatibility

No migration is required because the default remains positional. Apps opting into alignment must update UI to explain inserted/removed aligned rows.

## 11. Open questions

- Is this included in v2.0 or deferred to v2.1?
- Should row/column insertions be represented as structural changes separate from cell diffs?
