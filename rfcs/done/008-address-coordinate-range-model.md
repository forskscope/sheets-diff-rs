# RFC-008: Address, Coordinate, and Range Model

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Coordinates  

## 1. Summary

Define correct Excel coordinates, A1 labels, explicit ranges, and deterministic ordering.

## 2. Motivation

Wide-sheet correctness is a known bug class. v2 should use a coordinate model that makes invalid states harder to represent, supports Excel's full `.xlsx` row/column bounds, and avoids sentinel tricks for empty ranges.

## 3. Goals

- Support Excel `.xlsx` columns through `XFD` and rows through 1,048,576.
- Represent coordinates as numeric row/column plus A1 conversion.
- Represent empty ranges explicitly as `None` or `RangeState::Empty`.
- Sort by numeric coordinates, not address strings.
- Reject or diagnose invalid coordinates at boundaries.

## 4. Non-goals

- Do not support infinite worksheet dimensions.
- Do not model R1C1 notation in v2.0 unless trivial.
- Do not depend on string A1 parsing for internal ordering.

## 5. External design

Proposed types:

```rust
pub struct CellAddress {
    pub row: RowIndex,
    pub col: ColIndex,
    pub a1: String,
}

pub struct RowIndex(pub u32); // 1-based
pub struct ColIndex(pub u32); // 1-based

pub struct CellRange {
    pub start: CellCoord,
    pub end: CellCoord,
}

pub enum ComparedRange {
    Empty,
    NonEmpty(CellRange),
}
```

A1 generation must use full integer math:

```rust
fn col_to_label(mut col: u32) -> String {
    let mut bytes = Vec::new();
    while col > 0 {
        let rem = ((col - 1) % 26) as u8;
        bytes.push(b'A' + rem);
        col = (col - 1) / 26;
    }
    bytes.reverse();
    String::from_utf8(bytes).expect("ASCII column label")
}
```

## 6. Internal design

Internal code should use `CellCoord { row: u32, col: u32 }` as map keys. `BTreeMap<CellCoord, NormalizedCell>` gives deterministic ordering.

Do not sort by `a1` strings because lexical ordering puts `A10` before `A2`.

## 7. Data lifecycle

1. Used ranges are read from both sheets.
2. Empty ranges remain explicit.
3. One-sided ranges are handled as named branches.
4. Union ranges are computed only for non-empty cases.
5. Cell changes are sorted by `(row, col, change kind)`.
6. A1 strings are generated at output construction time.

## 8. Error, diagnostic, and edge-case behavior

Invalid coordinates should produce internal invariant errors if caused by the library, or diagnostics if caused by unexpected workbook metadata.

Empty sheets must not rely on `u32::MAX`/`u32::MIN` sentinels.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Column labels round-trip for 1, 26, 27, 52, 53, 255, 256, 257, 702, 703, and 16,384.
- Empty vs empty, empty vs non-empty, and sparse one-sided fixtures pass.
- `A2` sorts before `A10`.
- Address generation has property tests across 1..=16,384.

## 10. Migration and compatibility

v1 address strings remain available through `CellAddress::a1`, but consumers should prefer numeric coordinates for sorting and navigation.

## 11. Open questions

- Should coordinate constructors return `Result` or use unchecked internal constructors plus public checked constructors?
- Should row/column be one-based or zero-based in public fields?
