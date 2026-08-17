# Changelog

## [Unreleased]

### Changed

- **CLI contract change: exit code 3 for invalid/corrupt input (M4).** RFC-013
  specified this from the start; it was never emitted. `src/main.rs` collapsed
  every non-option, non-comparison-result failure to exit code 2, including
  corrupt input — a caller could not distinguish "this file is not a workbook"
  from "you passed a bad flag." Now: **3** when something about the bytes at
  the given path make them unusable as a workbook (wrong format, corrupt
  internals, or encrypted); **2** narrows to everything else — reaching those
  bytes in the first place (missing file, permissions, a lock held elsewhere),
  caller misconfiguration, a resource limit, or an internal bug. Full mapping
  and reasoning: [`docs/src/migration/v1-to-v2.md`](docs/src/migration/v1-to-v2.md#cli-exit-codes),
  also printed by `sheets-diff --help`.

  **This is a behaviour change to the CLI contract, not a bugfix footnote.**
  A script matching exit code `2` for "something went wrong with the file"
  will now see `3` for the corrupt-input subset of that. It is correct per
  RFC-013 and it is still a compatibility event — this is why the M4 release
  is 2.4.0 rather than 2.3.1. The library is unchanged; only `src/main.rs`
  moved. Covered by five new subprocess tests
  (`tests/cli.rs`) exercising the real binary — no exit code had ever been
  verified by anything before this.

- **Compatibility event: `max_cells_compared` now bounds what it names, and
  a comparison that succeeded before can start returning `LimitExceeded`
  (M4).** The check inside `build_sheet_diff` compared `cell_diffs.len()`
  against the limit — a count that only grows when a coordinate produces a
  diff, so it measured diffs found, not coordinates visited. A workbook with
  millions of populated cells and few differences passed straight through
  regardless of the configured bound; the limit could not do what
  `options.rs`'s own doc comment says the linear limits exist for ("their
  cost scales predictably with input size"). It now counts coordinates
  compared, cumulatively across the whole comparison (matching
  `max_diffs_returned`'s existing cumulative check), before each sheet's
  comparison work begins rather than partway through it.

  **`Limits::hardened()` sets `max_cells_compared: Some(5_000_000)`. Under
  the old enforcement that bounded diffs; under the new one it bounds
  coordinates.** A caller using `hardened()` (or setting this limit
  explicitly) to compare a large workbook with few differences, which
  succeeded in 2.3.0, can now return `LimitExceeded` in 2.4.0 — this is the
  limit finally doing what it was always documented to do, and it is still
  a behaviour change a consumer can be surprised by. `Limits::default()`
  leaves this limit unset, so default-configured callers are unaffected.
  `LimitExceeded { observed, .. }` now reports the coordinate count, not a
  diff count, for this limit.

### Added

- **The fixture corpus grew from 7 to 18 scenarios**, closing every gap
  ranked 1–5 by consequence in RFC-030 Handoff 01's coverage-dimension
  report (RFC-036). Each new scenario carries a dedicated assertion, not
  only a golden — RFC-036 §5.1 defines "covered" as an assertion that would
  fail if the behaviour broke, precisely because a golden alone cannot
  detect having been *born* wrong, which is what happened to the `formula`
  fixture for over a year. New coverage: row/formula origins shifted below
  row 1 (plus the D-04 negative control where the origins coincide);
  `AlignmentMode::RowSignature` and `HeaderColumn`, previously exercised by
  no test at any level; `CellError` comparison and
  `ValueDifferenceKind::ErrorKindChanged`, also previously untested at any
  level; `SheetChange::Moved`, never before distinguished from `Unchanged`
  by any assertion; ordinary serial-based dates in the golden corpus for
  the first time, despite dates being where four M2 defects lived; non-ASCII
  sheet names and cell text; a chart sheet beside a worksheet; a
  physically-present-but-empty leading cell (confirmed not to anchor the
  range origin, matching calamine's read source); and the ISO-datetime
  reachability case promoted from a hand-built test into a durable corpus
  trip-wire. Full matrix and the standing coverage obligation this creates
  for future changes to `normalize.rs`/`compare.rs`/`align.rs`/`diff.rs`:
  [`tests/fixtures/corpus/README.md`](tests/fixtures/corpus/README.md).
- `examples/gen-fixtures.rs` gained a `patch_xlsx_xml` helper (duplicated
  from `tests/support.rs`, consistent with the generator's existing
  independence from anything under `tests/`) for the two new scenarios
  `rust_xlsxwriter`'s public API cannot produce directly.

No comparison behaviour changed. This is test-corpus and test-infrastructure
work only; `src/` is untouched.

### Documentation

- **Two false statements removed from source comments (M4).**
  `meta.rs`'s `compare_workbook_metadata` claimed metadata comparison could
  be disabled via a `Ignore` mode and had a `CompareAvailable` default;
  neither exists — `WorkbookMetadataMode` (RFC-021) was never built, the
  function's `_opts` parameter is unused, and metadata comparison always
  runs unconditionally. Comments now say so.
- **Three `CellValue` variants documented as unreachable.** `Integer`,
  `Duration` and `Unsupported` cannot be produced through any `.xlsx` input
  this crate accepts — six of the nine variants are live in practice, and
  nothing in the public documentation previously said so. Each now carries
  a doc comment stating the fact and its cause (RFC-007).
- **Nine stale version anchors dropped from comments and one public error
  message.** `model.rs` and `options.rs` stated several still-true facts as
  if scoped to a past minor version (e.g. "Always empty in v2.0",
  "not available in v2.0") — accurate today, but implying a change that
  never happened by the time a reader reaches them in 2.3.0. Reworded to
  state the fact without the version, including the wording of
  `SheetsDiffError::InvalidOptions`'s message for `FormulaCompareMode`
  variants without an implemented normaliser.

No behaviour changed; these are documentation-only corrections.

### Fixed

- **`DiffMetrics.cells_compared` now counts what it claims to (M4).** It
  previously equalled `cells_changed` exactly, always — a dead `filter(...)`
  term in its accumulation formula could never contribute, so every
  compared-but-unchanged coordinate went uncounted. It is now accumulated
  where the comparison actually happens (once per coordinate in the
  aligned/positional coordinate set built in `build_sheet_diff`, regardless
  of whether that coordinate produces a diff), so it is always
  `>= diffs_emitted` rather than always equal to it. **2.2.3's changelog
  entry claiming this was already fixed is true as of this release; it was
  not true when written** (flagged wrong in M2 unit 06's audit; this closes
  that annotation). `cells_read` and `diffs_emitted` were checked and are
  unaffected — both were already counting correctly.

  This changes `DiffMetrics.cells_compared`'s value for any comparison with
  at least one compared-but-unchanged cell — which is the normal case. The
  fixture corpus moved accordingly: 13 goldens changed, each in exactly the
  `cells_compared` field and nothing else.

- **A disk or filesystem I/O failure while reading a sheet no longer
  reports as a corrupt workbook (M4).** `classify_read_error`'s catch-all
  routed every `calamine::XlsxError` other than `WorksheetNotFound` to
  `ReadErrorKind::MalformedSheet`, including `XlsxError::Io` — an I/O
  failure part-way through a read has nothing to do with the workbook's own
  content, and as of the CLI's exit-code-3 change (above) this
  misclassification would have told a user their file was corrupt when
  nothing was wrong with it. `XlsxError::Io` now classifies as
  `ReadErrorKind::Other`; `exit_code_for` maps `ReadSheet`'s sub-kinds
  individually rather than wholesale, with `Other` conservatively exiting 2
  rather than 3 — the same default already applied to `OpenErrorKind::Other`.
  **This changes which `ReadErrorKind` variant a given input can produce**
  (a `#[non_exhaustive]` public enum) — existing `match` arms compile
  unchanged since they must already have a catch-all, but the value observed
  for this input class changes.

## [2.3.0] - 2026-08-16

**Security and integrity release.** Clears two denial-of-service advisories
reachable from untrusted workbook input, bounds the first-party paths that could
exhaust memory or abort the host process, and fixes four defects where the
engine reported "identical" for cells that differ. A threat model now records
what is defended, what is not, and where each control is checked:
[`docs/src/maintainers/threat-model.md`](docs/src/maintainers/threat-model.md).

### Security

- **MSRV raised from 1.85.0 to 1.88.0; `calamine` upgraded from 0.35 to
  0.36.** This is a real compatibility event for consumers on older
  toolchains, called out here rather than as a footnote. The driver is
  security: `calamine` 0.35 pulled in `quick-xml` 0.39.4, which carries
  `RUSTSEC-2026-0194` (quadratic runtime on duplicate-attribute checking) and
  `RUSTSEC-2026-0195` (unbounded namespace-declaration allocation), both
  denial-of-service on XML input and both fixed in `quick-xml` >= 0.41.
  `calamine` 0.36 resolves `quick-xml` to 0.41.0 and `zip` to 8.6.0; neither
  advisory is reachable from the dependency tree after this change.
  Consumers that read `.xlsx` files they did not author — this crate's
  documented threat model — were exposed to both advisories through this
  path. Verified with `cargo audit` before and after: 0.35 shows 3
  vulnerabilities (the two above plus one unrelated, dev-only advisory in
  `crossbeam-epoch` via `criterion`), 0.36 shows only the unrelated one.
  `calamine`'s public API used by this crate (`Data`, `CellErrorType`,
  `XlsxError`, `SheetType`, `SheetVisible`, the `Reader` trait) is
  byte-identical between versions, and the full fixture corpus — all seven
  `expected.json` goldens — is unchanged, confirming the migration alters no
  comparison behaviour.

- **`#![forbid(unsafe_code)]` crate-wide.** The crate's one `unsafe` block
  (`address::col_to_label`'s `String::from_utf8_unchecked`) is replaced with
  the safe `String::from_utf8().expect(..)` — the bytes pushed are always
  ASCII uppercase, so the conversion cannot fail, and nothing is given up by
  going through the safe path.

### Added

- **Resource bounds on superlinear and input-size paths** (RFC-035). Two new
  `Limits` fields, both `Some` by default:
  - `max_alignment_product` (default 25,000,000, empirically measured — see
    RFC-035 §9) bounds the `old_rows × new_rows` row-alignment LCS matrix.
    When exceeded, the affected sheet degrades to positional comparison and
    emits an `alignment_bound_exceeded` diagnostic — it never errors.
  - `max_input_bytes` (default 500 MiB) bounds the input size, checked
    *before* any read begins (`fs::metadata` before `fs::read`, a `Seek` to
    measure length before `read_to_end`, or a length check before the
    internal `to_vec()`). Exceeding it is a hard `LimitExceeded` error, since
    unbounded allocation here happens before any comparison logic can
    observe or report it.

  `Limits::hardened()` now also sets both of the above, plus a preset for
  every other `Limits` dimension, for callers comparing untrusted input.
  New `DiffOptionsBuilder` methods: `max_alignment_product`,
  `max_input_bytes`, `limits`. New diagnostic codes:
  `alignment_bound_exceeded`, `duplicate_alignment_key`. New
  `LimitKind::InputBytes`.

- **`CellDateTime::has_serial: bool`** (D-01, see Fixed below) — distinguishes
  a genuine Excel date serial from the placeholder used when only an ISO
  string is available.

### Changed

- **Comparison output changes for four correctness fixes (D-01 through
  D-04, above).** These are patch-level in the sense that no public type
  signature changed beyond one additive field, but in substance they change
  what a comparison reports: cells the previous release silently reported as
  *identical* — ISO-typed dates/durations with different values, and rows
  affected by the alignment coordinate collision — will now correctly be
  reported as *different*, and a formula previously attached to the wrong
  cell will now attach to the right one. If you persist or diff against
  stored `WorkbookDiff` output from a prior release, expect these cases (if
  present in your data) to change. This is the fix, not a regression — the
  previous behaviour was silent data loss in a diff/merge context.
- **`DiffOptions::default()` now bounds alignment and input size.** Previously
  every `Limits` field defaulted to `None` (unbounded). The two new fields
  above default to `Some` (see Added), so a caller relying on
  `DiffOptions::default()` who compares a workbook pair whose row-alignment
  product or input size exceeds the new defaults will now see the alignment
  degrade to positional (no error) or the input rejected with
  `LimitExceeded` (a new error), where previously it ran unbounded. Opt back
  out with `Limits { max_alignment_product: None, max_input_bytes: None,
  ..Limits::default() }`.

### Fixed

- **ISO-typed date/time and duration values always compared equal (D-01).**
  `Data::DateTimeIso`/`Data::DurationIso` cells (calamine's `t="d"` path) had
  no genuine Excel serial — `serial` was hardcoded `0.0`, `is_1904` hardcoded
  `false` — so **any two ISO-typed values of the same kind compared equal
  regardless of their actual dates**: `2024-01-01T00:00:00` and
  `2099-12-31T23:59:59` were reported identical, as were `PT1H` and `PT99H`.
  In a diff/merge workflow this is a silent data-loss path: a real change is
  shown as "no change." `CellDateTime` gains a `has_serial: bool` field
  distinguishing a genuine serial from the `0.0` placeholder (a legitimate
  date can itself serialise to `0.0`, so the placeholder needed its own
  signal); comparison now uses `iso` when `has_serial` is `false` on both
  sides, and a value with a serial is never silently treated as equal to an
  ISO-only value with no serial. `CellValue::Duration` (always ISO-only in
  practice — see below) now compares via `iso` when present.
- **`is_1904` was hardcoded `false`, so `DateComparePolicy::NormalizeEquivalentDateTimes`
  was dead code (D-02).** The 1900/1904 epoch flag is workbook-level
  (`Xlsx::has_1904_epoch()`), not per-cell; it is now read once when a
  workbook is opened (`OpenedWorkbook::is_1904`) and threaded into every
  cell's `CellDateTime`. A caller who selected
  `NormalizeEquivalentDateTimes` previously got silence, never an error —
  the policy could never actually reconcile two dates across epochs because
  both were always flagged 1900. It now works.
- **Row alignment could silently merge two unrelated cells into one
  coordinate (D-03).** When a row-alignment mode was active, matched and
  removed rows were numbered in the *old* sheet's row space while inserted
  rows were numbered in the *new* sheet's — but both were inserted into the
  same `(row, col)` coordinate set. Whenever an inserted row's new-side
  number numerically coincided with an unrelated matched or removed old-side
  row number (common on any sheet with more than a handful of rows), the set
  silently deduplicated two distinct logical cells into one, and the lookup
  that followed could then compare the wrong pair of cells, or drop the
  inserted row's content entirely. Only reachable under a non-`Positional`
  alignment mode, which is why the fixture corpus never caught it. The
  internal coordinate key now carries which row-numbering space it came
  from, so a numeric coincidence can never merge two different cells.
- **Formula text could attach to the wrong cell (D-04).** `calamine`'s
  formula range and value range are independent `Range`s with their own
  origins — `worksheet_formula`'s range is built only from cells that
  actually carry formula text, so its top-left corner is the first *formula*
  cell, not the first populated cell. The formula lookup applied
  value-range-relative row/column indices directly to the formula range
  (`Range::get`, which is relative to *that* range's own origin), silently
  offsetting or dropping formula text whenever the two origins differed —
  for example, a text label in the first populated row with a formula
  starting further down. Now translates through absolute coordinates
  (`Range::get_value`), which is correct regardless of whether the two
  ranges' origins coincide.
- **Alignment duplicate-key diagnostic was misclassified.** `align.rs`
  reported duplicate row-alignment keys using `DiagnosticKind::UnsupportedCellValue`
  (documented meaning: "a cell value could not be normalised" — not what
  happened) with a message claiming a partial positional fallback that never
  actually occurred (LCS still ran on the full, duplicate-containing
  sequences). Replaced with `DiagnosticKind::DuplicateAlignmentKey` and a
  message that describes what actually happens.
- **Alignment's row-count guard was wired to the wrong limit.** The LCS
  matrix's row-count guard read `Limits::max_cells_compared` — a *cell*-count
  bound — as a *row* bound, and on tripping it silently built a fake
  low-confidence identity mapping with no diagnostic at all. It now reads
  the dedicated `max_alignment_product` bound (see Added, above), checked
  before any mode-specific alignment work, and degrades to the caller's
  existing true-positional path with an explicit diagnostic.
- **`src/objects.rs`'s coverage diagnostic corrected — the 2.2.3
  `cells_compared` claim documented as still wrong, not fixed.** Two
  unrelated corrections, both about claims this project made about itself:
  - The `UnsupportedWorkbookFeature` coverage message (emitted on every
    comparison) said "calamine 0.35 does not expose object content" and
    listed hyperlinks, tables, and pivot tables alongside charts and images
    as uniformly unavailable. Both are now wrong: the version is stale, and
    RFC-035 Handoff 01's spike established that calamine 0.36 *does* expose
    hyperlinks, merged regions, tables, and pivot tables — this crate simply
    does not call those APIs yet. The message now distinguishes "not
    exposed by calamine's API at all" (charts, images, comments, data
    validation, conditional formatting) from "available upstream, not yet
    used by this crate" (hyperlinks, merged regions, tables, pivot tables).
    `DiagnosticKind::code()` is unchanged (`unsupported_workbook_feature`)
    — only the human-readable message moved, which is why this changed all
    seven fixture goldens as a pure string substitution; see the corpus
    guide for what that first-bless lesson was about.
  - The 2.2.3 entry below claims `DiffMetrics.cells_compared` was fixed to
    count all coordinate pairs visited, not just changed cells. Verified at
    `0ba6aeb`: it does not, and never did — `build_sheet_diff` only ever
    pushes a `CellDiff` for a coordinate with an actual value or formula
    change, so the "compared but unchanged" term the accumulator adds is
    always zero. `cells_compared == cells_changed`, silently, since 2.2.3.
    Not fixed here — see the annotated entry below for why — but the claim
    is no longer left standing as true.

### Removed

- **The `parallel` feature is removed** (RFC-025, roadmap decision D2). It never
  compiled: `src/diff.rs` referenced `ExecutionMode::Parallel`, which
  `src/options.rs` never defined, so `cargo build --features parallel` has
  failed since 2.2.0. The design remains sound and RFC-025 stays `accepted/`,
  amended with the corrected rationale and a re-introduction gate. See the
  2.2.0 entry below, which is annotated rather than deleted.

## [2.2.3] - 2026-06-11

### Fixed (audit)

- **Dead code removed:**
  - `OpenedWorkbook::sheet_names()` was never called; removed.
  - `AlignmentModeLabel` enum and `AlignmentSummaryData.mode` field were
    written but never read; removed.
  - `make_renamed_workbook` in `benches/workbook_diff.rs` was unused; removed.
  - Crate-level `#![allow(dead_code)]` removed — no longer needed.
- **Metrics corrected:** `DiffMetrics.cells_read` now reflects the actual cell
  count from `read_sheet_cells` (was `1` per sheet). `DiffMetrics.cells_compared`
  now counts all coordinate pairs visited, not just changed cells.
  **Correction (see Unreleased):** the second half of this entry is wrong. It
  was wrong when written and is still wrong today — `cells_compared` counts
  only changed cells, exactly as before this entry claims to have fixed.
- **`compare` module made `pub(crate)`** — it is internal machinery. The
  `compare_values_pub` test helper is now `#[cfg(test)]` only.
- **Stale doc comments updated:** `WorkbookChange` / `WorkbookObjectChange` /
  `WorkbookDiff` comments no longer reference "v2.0" or "always empty in v2.0";
  they correctly describe the v2.2 state (RFC-021/023 surface through
  `diagnostics`; structured variants reserved for future).
  **Correction (see Unreleased, M4):** this entry is also wrong. `WorkbookChange`'s
  and `WorkbookObjectChange`'s doc comments still read "Always empty in v2.0"
  immediately before M4 unit 01 removed them — the same defect as the
  `cells_compared` entry two bullets above, in the same audit section that
  first named the problem, uncaught until now.
- **`criterion::black_box` deprecation** resolved — switched to
  `std::hint::black_box` throughout `benches/workbook_diff.rs`.

### Added (audit)

- `#[non_exhaustive]` added to all 26 public model types that were missing it
  (RFC-031 compliance).
- `CellDisplay::new()` and `CellSnapshot::new()` constructors — necessary
  because `#[non_exhaustive]` blocks struct literal construction outside the
  crate.
- `DiffOptionsBuilder::number_compare_policy()` builder method.
- Integration tests for `compare_readers` / `compare_readers_with_options`
  (RFC-004, previously untested) and `TypeMismatchPolicy::CompareDisplayString`
  (RFC-010, previously untested).

## [2.2.2] - 2026-06-11

### Changed

- Updated `criterion` from `0.5` to `0.8` (latest).
- Moved `criterion` from `[dependencies]` (optional) to `[dev-dependencies]`
  where it belongs — it is a benchmarking tool and has no place in the
  published dependency tree. The `bench` feature flag is removed; benches
  now compile unconditionally with `cargo build --benches`.
- Fixed two pre-existing bugs in `benches/workbook_diff.rs` that were
  previously hidden behind `required-features = ["bench"]`: a lifetime
  error in `bench_many_sheets` and a stale variable reference in
  `bench_alignment_vs_positional`.

## [2.2.1] - 2026-06-11

Additive response to integration feedback from ForskScope. No breaking changes.

### Added

- `output::view::CellChangeRow` now carries `old_formula: Option<&str>` and
  `new_formula: Option<&str>`, borrowed from the underlying `CellDiff`. GUI
  consumers can render formula changes without reaching past the view layer
  into the raw model.
- `output::view::OwnedCellChangeRow` — a fully owned counterpart to
  `CellChangeRow`, plus `CellChangeRow::to_owned_row()`. Convenience for
  consumers whose model outlives the `WorkbookDiff`.
- `ChangeAnchor` now derives `serde::Serialize` (under the `serde` feature).

### Documentation

- `Cancellation` trait: added an `Arc<AtomicBool>` cancellation example and a
  "Cancellation latency" section documenting that `is_cancelled()` is polled
  once per sheet pair (not mid-sheet).
- `DiagnosticKind::code()`: documented as the stable programmatic surface for
  diagnostics, with a full table of the current code strings.
- `CellDiff`: documented the "one `CellDiff` per address" consumer model and
  confirmed `change_kind()`'s derivation as stable API.
- `compare_paths`: documented that non-UTF-8 paths are fully supported with no
  internal `to_str()`/`unwrap()` on the path.
- `WorkbookDiff`: documented that `summary`, `metrics`, and the per-sheet
  `change` list are cheap to extract so bulky `cell_diffs` can be dropped.

## [2.2.0] - 2026-06-11

### Added

- **RFC-023 — Object / unsupported-feature coverage diagnostics**: every
  comparison emits an `Info`-level `UnsupportedWorkbookFeature` diagnostic
  explaining that charts, images, comments, hyperlinks, tables, pivot tables,
  and data validation are not compared. Non-worksheet sheet types (ChartSheet,
  MacroSheet, VBA) emit a `Warning`. Controlled by `ObjectCompareMode` (default
  `WarnIfPresent`); suppressible via `DiffOptionsBuilder::object_mode(Ignore)`.
- **RFC-024 — `DiffMetrics`**: `WorkbookDiff.metrics` carries `sheets_read`,
  `cells_read`, `cells_compared`, `diffs_emitted`, and `diagnostics_emitted`
  for benchmarking and performance analysis.
- **RFC-025 — Parallel sheet comparison** (`parallel` feature, off by default):
  `ExecutionMode::Parallel` processes sheets in parallel with `rayon`, then
  sorts results by original workbook order to guarantee identical output.
  Enable with `--features parallel`; select via
  `DiffOptionsBuilder::execution_mode(ExecutionMode::Parallel)`.
  **Correction (see Unreleased):** this entry is wrong. The feature never
  compiled — `ExecutionMode::Parallel` did not exist in `src/options.rs` — and
  its only test was gated on the same feature, so it never ran. The feature
  was removed rather than fixed; see RFC-025's amendment for why.
- **RFC-027 — Benchmarks** (`bench` feature): `benches/workbook_diff.rs`
  covers all eight RFC-027 scenarios (small-business, wide, tall, sparse,
  many-sheets, formula, rename, alignment cascade). Run with
  `cargo bench --features bench`.
- **RFC-028 — Fuzz targets** (`fuzz/`): four `cargo-fuzz` targets covering
  `compare_bytes` on arbitrary input, `col_to_label` roundtrip,
  `ComparedRange::union`, and `DiffOptionsBuilder::build`. Corpus seeds in
  `fuzz/corpus/fuzz_open_xlsx_bytes/`. See `fuzz/README.md`.
- **RFC-020 — Display formatting types**: `CellDisplay`, `CellSnapshot`,
  `CellNumberFormat`, `DisplaySource` added to the public model. `CellDisplay`
  carries a deterministic display string, an optional number-format record
  (`None` in calamine 0.35 — reserved for RFC-022), and a `DisplaySource` tag.
  `CellSnapshot` groups a `CellValue`, optional `FormulaText`, and optional
  `CellDisplay` with a `preferred_display()` helper. `CellValue::display_default()`
  is an alias for `display_string()` as per RFC-020 §6.
- **RFC-030 — Extended fixture corpus**: `tests/gen.rs` generates seven scenario
  fixtures (wide_columns, renamed_sheet, typed_values, formula, empty_sheet,
  sparse_range, row_insertion_cascade) into `tests/fixtures/generated/`, each
  with a `scenario.toml` and (with `--features serde`) an `expected.json`
  golden file. `tests/fixtures/corpus/README.md` documents the contribution policy.
- `ComparedRange::union` made `pub` (was `pub(crate)`).
- `DiffOptionsBuilder::object_mode`, `::execution_mode`, `::format_compare`
  builder methods.

## [2.1.0] - 2026-06-11

### Added

- **RFC-011 — Row alignment** (`AlignmentMode::RowKey`, `RowSignature`):
  opt-in row matching by key columns or content signature to reduce
  false-positive cascades after row insertions/deletions.
  `SheetDiff.alignment_summary` is populated when alignment is active.
- **RFC-021 — Workbook metadata diffs**: defined-name additions, removals, and
  target changes are reported as `Info`-severity diagnostics in
  `WorkbookDiff.diagnostics`. Sheet visibility changes are similarly reported.
  Defined-name scope is unavailable in calamine 0.35; a
  `DefinedNameScopeUnknown` diagnostic is attached when names are present.
- **RFC-022 — Format comparison policy**: `FormatCompareMode` enum added to
  `ComparisonOptions`. Selecting anything other than `Ignore` returns
  `SheetsDiffError::InvalidOptions` — calamine 0.35 exposes no cell-style API
  and the policy is honest about that.
- **RFC-029 — GUI view adapters** (`output::view`): `DiffView`, `CellChangeRow`,
  `SheetSummaryRow`, `ChangeAnchor`, `ViewFilter`. Framework-neutral borrowed
  iterators for sheet-tree, flat change-list, and prev/next navigation.
- `DiffOptionsBuilder::build_with_matching` convenience method.
- `FormatCompareMode` re-exported from crate root.

## [2.0.1] - 2026-06-11

### Added

- Expanded integration test corpus covering all RFC-015 fixture categories:
  corrupt inputs, wide-column A1 encoding (A–XFD), typed-value distinctions,
  formula handling, sheet rename/add/remove, empty and sparse ranges, resource
  limits, progress events, cancellation, text output, and JSON output.
- `tests/support.rs` — shared programmatic fixture builders.
- `tests/fixtures/corrupt/not_a_zip.xlsx` — committed corrupt binary fixture.
- `docs/src/migration/v1-to-v2.md` — migration guide (RFC-017): entry points,
  sheet changes, cell value model, duplicate-address policy, errors,
  diagnostics, text output, CLI exit codes, and a v1-style flattening helper.
- `docs/src/SUMMARY.md` and `docs/src/README.md` — mdbook scaffolding.

### Changed

- `compare` module is now `pub` so integration tests can call
  `compare_values_pub` directly; the function is `#[doc(hidden)]`.

## [2.0.0] - 2026-06-11

Complete rewrite.  v2 is a structured, library-first `.xlsx` diff engine.

### Breaking changes from v1

- **New public types**: `WorkbookDiff`, `SheetDiff`, `CellDiff`, `CellValue`
  replace the old `Diff`/`SheetDiff`/`CellDiff` string model.
- **Typed cell values**: `CellValue::Integer`, `Number`, `Bool`, `DateTime`,
  `Duration`, `Error`, `Empty` — no more stringly-typed old/new fields.
- **One `CellDiff` per address**: value and formula changes are subfields
  (`value`, `formula`), not separate entries.
- **Structured errors**: `SheetsDiffError` is `#[non_exhaustive]`; no more
  panics on ordinary bad input.
- **No stdout/stderr writes** from library code.
- Entry points: `compare_paths`, `compare_bytes`, `compare_readers` (and
  `_with_options` variants).

### New features

- Conservative sheet rename detection (`SheetMatchingMode::ExactNameThenConservativeRename`).
- `DiffOptions` grouped tree with builder; `Limits`, `ProgressSink`,
  `Cancellation` hooks.
- `EncryptedWorkbook` error for password-protected files.
- Correct Excel A1 addressing through column `XFD` (column 16 384).
- Deterministic result ordering by sheet index, then `(row, col)`.
- Text and unified-diff output formatters over `WorkbookDiff`.
- Optional `serde` feature: `Serialize` derives on all public model types.

### Migration from v1

See `docs/migration/v1-to-v2.md` (RFC-017 deliverable, to be added).

Quick reference:

| v1 | v2 |
|---|---|
| `Diff::new(old, new)` | `compare_paths(old, new)?` |
| `diff.cell_diffs[i].old` (String) | `diff.sheets[s].cell_diffs[c].value.as_ref().map(|v| v.old.display_string())` |
| `CellDiffKind::Value / Formula` | `CellDiff.value.is_some()` / `.formula.is_some()` |
| panic on bad input | `Err(SheetsDiffError::...)` |
