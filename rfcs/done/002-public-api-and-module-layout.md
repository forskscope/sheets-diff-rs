# RFC-002: Public API and Module Layout

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** Public API  

## 1. Summary

Define the stable public module structure and prevent internal comparison machinery from becoming accidental public API.

## 2. Motivation

v1 exposes implementation-flavored modules and structs. v2 should present app developers with a small vocabulary: compare functions, options, model types, events, diagnostics, and output helpers. Internal parsing and comparison details should remain private so they can evolve.

## 3. Goals

- Expose a small, documented module hierarchy.
- Separate model, options, diagnostics, events, and output helpers.
- Avoid exposing calamine-specific implementation types in the primary public model.
- Provide ergonomic top-level compare functions for common use.
- Make advanced APIs discoverable without forcing them on simple users.

## 4. Non-goals

- Do not expose every internal module.
- Do not require consumers to understand calamine internals.
- Do not design a plugin framework in v2.0.

## 5. External design

Proposed public surface:

```rust
pub mod diff;
pub mod model;
pub mod options;
pub mod diagnostics;
pub mod events;
pub mod output;

pub use diff::{compare_paths, compare_bytes, compare_readers, compare_with_options};
pub use model::{WorkbookDiff, SheetDiff, CellDiff, CellValue, CellAddress};
pub use options::{DiffOptions, DiffOptionsBuilder};
pub use diagnostics::{SheetsDiffError, Diagnostic, Warning};
```

Simple callers should be able to write:

```rust
let diff = sheets_diff::compare_paths("old.xlsx", "new.xlsx")?;
```

Advanced callers should be able to write:

```rust
let opts = DiffOptions::builder()
    .compare_formulas(true)
    .sheet_matching(SheetMatchingMode::ConservativeRename)
    .max_cells(2_000_000)
    .build();

let diff = sheets_diff::compare_with_options(old_source, new_source, opts)?;
```

## 6. Internal design

Private implementation modules should be organized by pipeline stage:

```text
src/
  lib.rs
  diff.rs
  model.rs
  options.rs
  diagnostics.rs
  events.rs
  output/
  internal/
    source.rs
    open.rs
    sheet_meta.rs
    sheet_match.rs
    range.rs
    normalize.rs
    compare_cells.rs
    progress.rs
```

`internal::*` must not be publicly re-exported. If tests need internal access,
prefer crate-private modules plus integration tests through public APIs.

## 7. Data lifecycle

The public call enters `diff::*`, which converts user input into internal sources, opens workbooks, builds metadata, performs comparison, and returns `model::WorkbookDiff`. Only the final structured model crosses the public boundary.

## 8. Error, diagnostic, and edge-case behavior

Public APIs must return `Result<_, SheetsDiffError>` for fallible operations. Convenience methods may exist only if clearly marked and should not be the recommended path.

Deprecation and feature errors should be structured, not documented as panics.

## 9. Testing and acceptance criteria

Acceptance criteria:

- `cargo doc` shows a clear public module tree.
- Top-level examples compile.
- Internal modules are not visible as public API.
- The CLI imports only public APIs.
- No public function requires a lossy `&str` path.

## 10. Migration and compatibility

v1 users should migrate from `sheets_diff::core::diff::Diff::new` to top-level `compare_paths` or option-based APIs. Any legacy compatibility module should be temporary and documented.

## 11. Open questions

- Should the crate expose a prelude module?
- Should old v1 names be kept behind a `legacy` feature for one major version?
