# sheets-diff

[![crates.io](https://img.shields.io/crates/v/sheets-diff.svg)](https://crates.io/crates/sheets-diff)
[![docs.rs](https://docs.rs/sheets-diff/badge.svg)](https://docs.rs/sheets-diff)
[![deps.rs](https://deps.rs/crate/sheets-diff/latest/status.svg)](https://deps.rs/crate/sheets-diff)
[![License](https://img.shields.io/github/license/nabbisen/sheets-diff-rs)](LICENSE)

**Structured diff engine for Microsoft Excel `.xlsx` workbooks.**

Compares two workbooks and returns a typed, deterministic result — not just a
text diff. Application developers get the data; they decide the presentation.

## Overview

`sheets-diff` compares `.xlsx` workbooks at the cell level and returns a
`WorkbookDiff` carrying:

- per-sheet change classification (added, removed, renamed, moved, modified);
- per-cell typed value changes (`Integer`, `Number`, `Bool`, `DateTime`, …);
- per-cell formula text changes;
- structured diagnostics and per-level summary counts;
- deterministic ordering by sheet index, then `(row, col)`.

**Current format support: `.xlsx`.** Other spreadsheet formats (`.ods`,
`.xlsm`, …) may be added in future versions. Plain-text formats such as CSV
and TSV are out of scope — diffs of text files are better served by dedicated
text-diff tools.

## Why / When

Use `sheets-diff` when you need:

- a **library** that returns structured data rather than printing a diff;
- **typed values** — `Text("100")` and `Integer(100)` are distinct;
- **GUI or batch** integration: bytes/reader inputs, progress events, cancellation;
- **no hidden side effects** — the library never writes to stdout/stderr or
  accesses the network.

It is intentionally *not* a spreadsheet editor, merge engine, or formula engine.

## Quick Start

```rust
use sheets_diff::compare_paths;

let diff = compare_paths("old.xlsx", "new.xlsx")?;
println!("{} cell(s) changed", diff.summary.cells_changed);

for sheet in &diff.sheets {
    for cell in &sheet.cell_diffs {
        if let Some(vc) = &cell.value {
            println!("{}: {} → {}", cell.address.a1,
                vc.old.display_string(), vc.new.display_string());
        }
    }
}
```

## Features

| Cargo feature | What it enables |
|---|---|
| *(none)* | Core library — no extra deps |
| `serde` | `Serialize` on all public model types; `output::json` helpers |
| `chrono` | ISO-8601 string synthesis for `DateTime` values |
| `cli` | Builds the `sheets-diff` binary (requires `clap`) |

## Design Notes

- **One `CellDiff` per address.** Value and formula changes are independent
  sub-fields; no duplicate-address entries.
- **`#[non_exhaustive]` everywhere.** Adding fields or variants in v2.x is
  additive — no forced semver bump for struct consumers.
- **Conservative by default.** Sheet rename detection only fires when exactly
  one old and one new sheet are unmatched. Ambiguous matches surface as
  `Added`/`Removed` plus a diagnostic.
- **`calamine` 0.35 pinned.** The `Data` enum variant set is the grounding
  for all `CellValue` conversions.

## More Detail

- [Full documentation](docs/src/SUMMARY.md) *(mdbook)*
- [Migration from v1](docs/src/migration/v1-to-v2.md)
- [RFCs](rfcs/) — design records for every significant decision
