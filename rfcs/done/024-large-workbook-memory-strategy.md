# RFC-024 — Large Workbook Memory Strategy and Streaming-Oriented Internals

**Status.** Partially implemented (2.0.0–2.2.3) — verified 2026-08-16. Deferred: cancellation is polled once per sheet pair, not between row chunks or cell batches as the acceptance criteria specify — on a single very large sheet, cancellation is not observed mid-sheet (documented as a known limitation on `Cancellation`'s doc comment, but this RFC's own criterion overclaims the granularity).
**Target:** v2.0 guardrail, v2.x optimization  
**Related:** RFC-004, RFC-006, RFC-012, RFC-016, RFC-027

## 1. Summary

Define memory and data-flow rules for large workbooks. v2 does not need to be a
fully streaming spreadsheet diff engine on day one, but it must avoid designing
public APIs that require loading excessive duplicated data or prevent future
streaming improvements.

## 2. Motivation

A GUI diff application may compare workbooks with many sheets, tens of thousands
of rows, and thousands of columns. The original implementation performs a
blocking pass and stores stringified cell diffs. v2 introduces richer data, so
memory discipline becomes more important.

## 3. Goals

- Avoid unnecessary duplication of cell data.
- Process sheets independently where possible.
- Support resource limits from `DiffOptions`.
- Make future streaming or chunked processing possible.
- Keep output deterministic even if internals become parallel or chunked.

## 4. Non-goals

- True constant-memory comparison for every `.xlsx` file in v2.0.
- Streaming output API in v2.0.
- Supporting arbitrary gigantic workbooks without bounds.

## 5. Internal lifecycle

Recommended internal stages:

```text
InputSource
  -> OpenedWorkbook
  -> WorkbookManifest
  -> SheetPairPlan[]
  -> SheetSnapshot / RowChunk
  -> SheetDiff
  -> WorkbookDiff aggregation
```

`WorkbookManifest` contains sheet names, order, dimensions, and cheap metadata.
The engine should plan sheet matching before reading every cell.

## 6. Sheet processing policy

Process one matched sheet pair at a time by default. Do not build a complete
`Vec<NormalizedCell>` for every sheet in both workbooks unless necessary.

For a sheet pair:

1. determine used ranges;
2. enforce max-cell bounds early;
3. load or iterate normalized cells;
4. compare;
5. release temporary per-sheet snapshots;
6. append final `SheetDiff` to result.

## 7. Data structures

For sparse sheets, prefer maps keyed by coordinate:

```rust
type CellMap = BTreeMap<Coord, NormalizedCell>;
```

For dense sheets, row-oriented vectors can be faster. The internal engine can
choose based on density:

```rust
enum SheetCells {
    Sparse(BTreeMap<Coord, NormalizedCell>),
    Dense(Vec<RowCells>),
}
```

Public API should not expose this choice.

## 8. Resource limits

`DiffOptions` should include:

```rust
pub struct ResourceLimits {
    pub max_cells_read: Option<u64>,
    pub max_cells_compared: Option<u64>,
    pub max_sheets: Option<u32>,
    pub max_diffs_returned: Option<u64>,
}
```

When a limit is reached, the engine returns either a structured error or a
partial result according to a separately accepted partial-result policy. v2.0
should prefer returning an error rather than partial results unless partial
semantics are carefully designed.

## 9. Progress integration

Large-workbook processing must emit progress at sheet boundaries and at row or
cell-count intervals. Progress should be approximate, not a guarantee of exact
percentage.

## 10. Acceptance criteria

- A multi-sheet workbook is processed sheet-by-sheet internally.
- Limits are checked before expensive work where possible.
- The engine can cancel between row chunks or cell batches.
- Benchmarks include at least one wide sheet and one tall sheet.
- Public API does not expose internal storage choices.
