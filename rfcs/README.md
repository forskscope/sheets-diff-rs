# `sheets-diff` RFCs

Design records for significant decisions in this project. Lifecycle, folder
semantics, numbering, and status conventions are defined by
[RFC 000](./done/000-rfc-lifecycle-policy.md).

This project uses the **5-folder variant**:

| Folder | State | Meaning |
|---|---|---|
| [`proposed/`](./proposed/) | Proposed | Open for review. Do not start implementation. |
| [`accepted/`](./accepted/) | Accepted | Design settled; implementation may start. |
| [`done/`](./done/) | Implemented | Shipped. Kept permanently as a historical record. |
| [`archive/`](./archive/) | Withdrawn / Superseded | Not pursued, or replaced by a later RFC. Kept permanently. |

The folder is the source of truth for an RFC's state; the `Status` field inside
each file mirrors it and is updated in the same commit that moves the file.
This index is updated in that commit too.

## Numbering: two series

This project has **two RFC series**, both numbered from 001, because the v1.2
stabilization line and the v2 redesign were planned independently. RFC 000
forbids renumbering an RFC that has already been referenced, and both series
have been — so they are separated by directory instead:

- A **bare number** (`RFC-014`) means the **v2 series**, in `done/` or
  `accepted/`. This is what `src/` comments and `CHANGELOG.md` refer to.
- The **v1.2 series** lives in [`done/1.2/`](./done/1.2/) and is referenced as
  `RFC 1.2/003`. Commit messages predating 2.0.0 that say "RFC 003" mean this
  series.

Numbers are assigned at creation, are never reused, and never change when a
file moves. The v2 series continues from 034; 033 is reserved for the missing
RFC described below.

Implementation companion documents live under
[`handoffs/NNN-slug/`](./handoffs/). They have no lifecycle state of their own —
it is inherited from the matching RFC — and they must not redefine it. The
active roadmap is [`ROADMAP.md`](../ROADMAP.md).

## Proposed

_None._

## Accepted

Design settled; implementation may start.

| ID | Title | Note | Handoffs |
|----|-------|------|----------|
| 022 | [Styles and Formatting Diff Policy](./accepted/022-styles-and-formatting-diff-policy.md) | `FormatCompareMode` rejects every non-`Ignore` mode; no style diff layer exists. Blocked upstream — calamine keeps `mod formats` private through 0.36. | — |
| 025 | [Deterministic Parallel Execution](./accepted/025-deterministic-parallel-execution.md) | Implementation removed 2026-08-15 (it never compiled, and parallelised the wrong phase); design retained and amended with a re-introduction gate. | [yes](./handoffs/025-deterministic-parallel-execution/) — done |

## Implemented

### Cross-cutting

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | 1.2.0 |

### v2 series

| ID | Title |
|----|-------|
| 001 | [v2 Release Scope and Compatibility Line](./done/001-v2-release-scope-and-compatibility-line.md) |
| 002 | [Public API and Module Layout](./done/002-public-api-and-module-layout.md) |
| 003 | [Workbook Result Data Model](./done/003-workbook-result-data-model.md) |
| 004 | [Input Sources and Workbook Opening](./done/004-input-sources-and-workbook-opening.md) |
| 005 | [Error, Diagnostics, and Warning Model](./done/005-error-diagnostics-and-warning-model.md) |
| 006 | [Diff Options and Configuration Builder](./done/006-diff-options-and-configuration-builder.md) |
| 007 | [Typed Cell Values and Normalization](./done/007-typed-cell-values-and-normalization.md) |
| 008 | [Address, Coordinate, and Range Model](./done/008-address-coordinate-range-model.md) |
| 009 | [Sheet Matching, Renames, and Moves](./done/009-sheet-matching-renames-and-moves.md) |
| 010 | [Cell Comparison Engine and Change Policy](./done/010-cell-comparison-engine-and-change-policy.md) |
| 011 | [Row/Column Alignment Optional Quality Mode](./done/011-row-column-alignment-optional-quality-mode.md) |
| 012 | [Progress, Cancellation, and Resource Bounds](./done/012-progress-cancellation-and-resource-bounds.md) |
| 013 | [Output Formatters, CLI, and Exit Codes](./done/013-output-formatters-cli-and-exit-codes.md) |
| 014 | [Serde Feature and Stable Report Schema](./done/014-serde-feature-and-stable-report-schema.md) |
| 015 | [Test Fixtures, Regression, and Property Testing](./done/015-test-fixtures-regression-and-property-testing.md) |
| 016 | [Security, Privacy, and No-Side-Effects Policy](./done/016-security-privacy-and-no-side-effects-policy.md) |
| 017 | [v1 to v2 Migration Guide and Adapter](./done/017-v1-to-v2-migration-guide-and-adapter.md) |
| 018 | [Formula Comparison Semantics](./done/018-formula-comparison-semantics.md) |
| 019 | [Numeric, Date, and Tolerance Comparison Policies](./done/019-numeric-date-and-tolerance-comparison-policies.md) |
| 020 | [Display Formatting and Number-Format Capture](./done/020-display-formatting-and-number-format-capture.md) |
| 021 | [Workbook Metadata and Defined-Name Diffs](./done/021-workbook-metadata-and-defined-name-diffs.md) |
| 023 | [Non-Cell Workbook Objects and Unsupported Feature Reporting](./done/023-non-cell-workbook-objects-and-unsupported-features.md) |
| 024 | [Large Workbook Memory Strategy](./done/024-large-workbook-memory-strategy.md) |
| 026 | [Feature Flags and Dependency Governance](./done/026-feature-flags-and-dependency-governance.md) |
| 027 | [Benchmark and Performance Governance](./done/027-benchmark-and-performance-governance.md) |
| 028 | [Fuzzing and Hostile-Input Hardening](./done/028-fuzzing-and-hostile-input-hardening.md) |
| 029 | [GUI Integration View Adapters](./done/029-gui-integration-view-adapters.md) |
| 030 | [Extended Fixture Generators and Corpus Management](./done/030-extended-fixture-generators-and-corpus-management.md) |
| 031 | [API Stability, SemVer, and Deprecation Policy After v2](./done/031-api-stability-semver-and-deprecation-policy.md) |
| 032 | [Unsupported, Corrupt, and Encrypted Workbook Handling](./done/032-unsupported-corrupt-and-encrypted-workbook-handling.md) |
| 034 | [Build Assurance and Fixture Integrity](./done/034-build-assurance-and-fixture-integrity.md) — *implemented M1, 2026-08-15; no release* |

### v1.2 series

| ID | Title | Shipped in |
|----|-------|------------|
| 1.2/001 | [v1.2 Stabilization Scope and Compatibility Policy](./done/1.2/001-v1.2-stabilization-scope.md) | 1.2.0 |
| 1.2/002 | [Fallible Diff Construction and Error Model](./done/1.2/002-fallible-diff-construction-error-model.md) | 1.2.0 |
| 1.2/003 | [Full Excel A1 Addressing and Stable Cell Ordering](./done/1.2/003-full-excel-a1-addressing-and-ordering.md) | 1.2.0 |
| 1.2/004 | [Reader-Based and Path-Safe Input Constructors](./done/1.2/004-reader-based-and-path-safe-input-constructors.md) | 1.2.0 |
| 1.2/005 | [Library Diagnostics Without Stdout Writes](./done/1.2/005-library-diagnostics-without-stdout-writes.md) | 1.2.0 |
| 1.2/006 | [Regression Fixture and CI Hardening](./done/1.2/006-regression-fixture-and-ci-hardening.md) | 1.2.0 |

## Archive

_None._

---

## Restoration notes (2026-08-15)

This directory was rebuilt on 2026-08-15. Commit `a06fb0e` ("v2 first release")
had deleted the entire `rfcs/` tree, including RFC 000 and the six implemented
v1.2 RFCs.

**Provenance.** The v1.2 series and RFC 000 were recovered from git history at
`a06fb0e^`. The v2 series was imported from the planning package that had never
been committed to this repository; its files were renamed from `RFC-NNN-slug.md`
to the `NNN-slug.md` form RFC 000 requires, and release-tag references were
normalised to drop the `v` prefix.

**Status verification is outstanding.** The v2 series was placed in `done/`
because the v2 line shipped across 2.0.0–2.2.3, and each file's `Status` field
says so — but **no per-RFC verification against the implementation has been
performed**, and each file records that caveat. Two are known to be wrong and
have already been moved to `accepted/` (022, 025); others are known to be
*partial* — 014 ships `Serialize` but no `Deserialize`, 020's `CellNumberFormat`
is always `None`, and 021/023 surface findings only as diagnostics with their
structured `WorkbookChange` types permanently empty. A verification pass should
either confirm each `Implemented` claim or move the RFC and record what was
deferred.

**RFC-033 is missing.** `src/` cites it as normative in 11 places — it is named
as the canonical lexicon for the public model in `model.rs`, `options.rs`, and
`error.rs`. No copy exists in this repository, in git history, or in the
planning package. It has to be reconstructed from the code that references it.
