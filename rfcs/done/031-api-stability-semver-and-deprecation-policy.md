# RFC-031 — API Stability, SemVer, and Deprecation Policy After v2

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0 decision  
**Related:** RFC-001, RFC-002, RFC-003, RFC-014, RFC-017, RFC-026

## 1. Summary

Define the public API stability policy for `sheets-diff` after v2.0. Because v2
is a breaking redesign, it should also establish rules that prevent avoidable
v3 churn.

## 2. Motivation

Downstream applications will build adapters around `WorkbookDiff`, `CellDiff`,
`CellValue`, diagnostics, options, and serialization. If those types change
frequently, the crate becomes expensive to adopt. The v2 release should set a
clear boundary between stable API, experimental features, and internal modules.

## 3. Goals

- Identify stable public API surfaces.
- Use semver consistently.
- Provide deprecation windows.
- Mark experimental features clearly.
- Keep internal modules private.

## 4. Non-goals

- Freezing all behavior forever.
- Guaranteeing stable JSON for experimental fields.
- Avoiding all breaking changes if a v3 is truly needed.

## 5. Public API categories

### Stable in v2.x

- `diff_*` entry points;
- `DiffOptions` builder;
- `WorkbookDiff`, `SheetDiff`, `CellDiff` core fields;
- `CellValue` core variants;
- error and diagnostic top-level model;
- stable JSON schema when `serde` is enabled.

### Additive in v2.x

- new diagnostics;
- new optional object/format change variants if enums are marked
  `#[non_exhaustive]`;
- new options with safe defaults;
- new adapter/helper methods.

### Experimental

- formatting diff internals;
- object diffing;
- formula normalization;
- parallel execution;
- alignment heuristics until quality gates pass.

## 6. Rust API policy

Use `#[non_exhaustive]` on public enums likely to grow:

```rust
#[non_exhaustive]
pub enum DiagnosticKind { ... }
```

For structs, prefer constructors/builders if fields may grow. If structs are
field-public for ergonomics, adding fields is breaking for literal construction.
Choose deliberately.

Recommendation: public result structs may expose read-only fields if stable, but
options should use builders.

## 7. Deprecation policy

- Deprecate before removal when possible.
- Keep deprecated APIs for at least one minor release.
- Document replacements in rustdoc and migration guide.
- Do not remove APIs in patch releases.

## 8. JSON schema policy

If `serde` output is advertised as stable:

- adding optional fields is minor-compatible;
- removing or renaming fields is breaking;
- changing enum tag names is breaking;
- experimental sections must be under clearly named optional keys.

## 9. Release checklist

Before v2.0.0:

- run public API review;
- run `cargo public-api` or equivalent if adopted;
- verify docs examples compile;
- confirm experimental feature labels;
- update migration guide.

## 10. Acceptance criteria

- Public enums likely to grow are non-exhaustive or otherwise future-proofed.
- `DiffOptions` uses a builder.
- Experimental features are documented.
- SemVer policy is included in README before v2.0 release.
