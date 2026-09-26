# RFC-011: Row/Column Alignment Optional Quality Mode

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.

**Corrected M10 unit 06 (3.0.0):** the Status covers the **row half only**, and the Status word stays.
§3's goal *"design column alignment based on header names or column signatures"* was **never
implemented**: there is no column alignment anywhere in this crate (`RowMapping` is the only mapping type).
§5's `HeaderColumn` variant *looked* like it was — `header_column_alignment` delegated to
`row_key_alignment` with `columns = [1]` and **never read a header**, i.e. it was exactly
`RowKey { columns: vec![1] }` — and §5's `RowAndColumn` variant never existed at all. `HeaderColumn` was removed
in 3.0.0 (migration: `RowKey { columns: vec![1] }`, proved equal by `tests/rowkey_replaces_header_column.rs`).
Why the Status word is unchanged: the unit-01 distinction applies. Column alignment is a §3 *design goal*;
§9's acceptance criteria — the contract — are all about row alignment (insert a row at the top; duplicate
keys detected; positional unchanged; cancellable) and pass. An RFC whose acceptance criteria are met is
implemented; a goal never built is a stale paragraph, not a broken promise. Column alignment remains open
work, and adding it later is an added variant on a `#[non_exhaustive]` enum.
**Corrected f130 (3.1.0): `RowKey` requires a key, and now says what it does when a row has none.** §8 says
"missing keys … should produce warnings and fall back to positional comparison". The implementation did neither:
`extract_row_keys` put a row in its map only if the row had a cell in a key column, so a row with none was in none of
`matched`, `removed` or `inserted` — **never compared, a change in it reported as no change, and the sheet's
`alignment_summary` said `confidence: Exact`.** That is not a missing feature: it is a wrong answer, in every release
from 2.1.0 (which introduced `RowKey`) through 3.0.0. **Now:** a row with no cell in any key column is *unmatched, not
absent*: rows with the same values in the same columns on both sides are paired with one another, in row order, and
compared like any matched pair (an unchanged subtotal or spacer line is silent); the rest are reported as removed (old
side) or inserted (new side), so a changed keyless row reaches the comparison as a whole-row change; a `Warning` `missing_alignment_key` (`DiagnosticKind::MissingAlignmentKey { old_count, new_count }`) names how many
rows each side had, on the sheet; and `confidence` is at most `Medium`, since `Exact` and `High` claim a reliability the
alignment does not have. A key column that no row populates (or an empty `columns` list) is the same rule applied to every
row. **Not done, and deliberate:** the "fall back to positional" half of §8 — comparing keyless rows against their
neighbours, which would be better than removed + inserted — needs a design (which neighbour; what when the counts differ
between sides) and is a separate unit. Until then a keyless row that *changed* is reported as a removal plus an
insertion. (A first version of this fix reported *every* keyless row that way and measured 2,200 cell diffs for one
changed cell on a 2,000-row sheet with a blank key every twentieth row; identical-row pairing brings it to 22, against
`Positional`'s 1, and a workbook compared with itself to 0.) Rows with *some* but not all of several
key columns are keyed by the values they have; that is unchanged and is not addressed here.
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

Duplicate keys, missing keys, and unstable signatures should produce warnings and fall back to positional comparison for ambiguous sections. *(Annotated f130, 3.1.0: duplicate keys warn and do **not** fall back — the LCS runs on the full sequences; missing keys warn and are reported removed / inserted, with no positional fallback. See the correction at the top.)*

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
