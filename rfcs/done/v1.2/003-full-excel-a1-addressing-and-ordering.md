# RFC 003 — Full Excel A1 Addressing and Stable Cell Ordering

**Status.** Implemented (v1.2.0)
**Tracks.** User-visible correctness.
**Touches.** `src/core/utils.rs`, `src/core/diff.rs`, `tests/utils_address.rs`,
`tests/wide_columns.rs`.

## Summary

Replaced the `u8`-based `cell_pos_to_address` with a full-width Excel A1
column label generator covering the complete `.xlsx` range (`1..=16384`,
`A` through `XFD`). Changed cell diff sorting from lexical A1 string order
to numeric `(row, col, kind)` order.

## Problem

The old code used `col as u8`, truncating columns above 255 and potentially
underflowing at column 256 in debug builds. Excel supports up to `XFD`
(column 16,384). String-sorted A1 addresses put `A10` before `A2`.

## Delivered API

```rust
// src/core/utils.rs (public)
pub fn col_to_label(col: usize) -> String;
pub fn cell_pos_to_address(row: usize, col: usize) -> String;
```

Both functions use base-26 alphabetic conversion with 1-based Excel
semantics. `col_to_label` asserts `col > 0` in debug builds.

## Sorting change

```rust
// Before (v1.1.4) — lexical address string order
x.cells.sort_by(|a, b| a.addr.cmp(&b.addr).then_with(|| a.kind.cmp(&b.kind)));

// After (v1.2.0) — numeric grid order
x.cells.sort_by(|a, b| {
    a.row.cmp(&b.row)
        .then_with(|| a.col.cmp(&b.col))
        .then_with(|| a.kind.cmp(&b.kind))
});
```

## Correctness table

| col | v1.1.4 | v1.2.0 |
|----:|--------|--------|
| 255 | IU     | IU ✓   |
| 256 | ?      | IV ✓   |
| 257 | ?      | IW ✓   |
| 703 | ?      | AAA ✓  |
| 16384 | ?    | XFD ✓  |

## Compatibility impact

Pure correctness fix. Output for workbooks with columns > 255, and output
ordering for any workbook, may change from v1.1.4. Both changes are
intentional and noted in the release notes.

## Open questions

None.
