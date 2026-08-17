# Non-goals and limitations

What this engine deliberately does not attempt, and where it is limited
despite trying. Both lists are current as of 2.4.x, checked against the
code rather than against what an earlier document said about it — several
items below were previously recorded elsewhere as either more complete or
more current than they actually are; see [Corrections found writing this
page](#corrections-found-writing-this-page).

---

## Non-goals

Deliberately out of scope — not a gap this project intends to close:

- **Cell formatting comparison.** Styles, number formats, cell-level
  formatting. `FormatChange` exists in the public model, reserved,
  permanently empty — see [Limitations](#limitations) for why (both a
  library decision and an upstream constraint).
- **Decrypting password-protected workbooks.** Detecting and refusing is
  the entire designed behaviour (`SheetsDiffError::EncryptedWorkbook`, CLI
  exit code 3) — not a step toward eventually reading them.
- **Evaluating formulas.** Formula *text* is compared as a string; it is
  never evaluated, and no code path in this crate executes anything a
  workbook contains. Two formulas that would compute the same result but
  are written differently (`=1+1` vs `=2+0`, the [semantics
  page](semantics.md#formula-change)'s own example) are reported as
  changed — this engine has no concept of formula equivalence, only text
  equivalence.
- **Writing or merging workbooks.** This is a diff engine. It reads two
  inputs and reports differences; it has no write path, and producing a
  merged or patched `.xlsx` is out of scope entirely, not deferred.
- **Formats other than `.xlsx`.** No `.xls`, no `.ods`, no CSV. The
  `calamine::Data::DurationIso` mapping exists in `normalize.rs` because
  calamine's `Data` enum is shared across formats it supports, not because
  this crate reads any of them — see `CellValue::Duration` under
  [Limitations](#limitations).

---

## Limitations

Three kinds, because the right response differs for each: an **upstream**
limitation may lift when `calamine` changes; a **deliberate deferral** is a
decision this project made and could revisit; **unreachable-by-construction**
describes public API surface that exists for forward compatibility but has
no live case today.

### Upstream

- **`CellNumberFormat` is always `None`.** `calamine::formats` is a private
  module — number-format capture has no data to read.
- **Charts, images, comments, data validation, and conditional formatting
  are not comparable at all.** Not exposed by calamine 0.36's public API —
  there is no data to compare, upstream or otherwise.

### Deliberate deferral

- **`serde::Deserialize` is not implemented** on public model types.
  `Serialize` ships; round-tripping a `WorkbookDiff` back into this crate's
  types from JSON is not supported.
- **`FormatChange` is reserved and permanently empty** (RFC-022) — no cell-
  style comparison exists. Partly forced (calamine 0.36 does not expose a
  cell-style API either, so this is upstream *and* deferred at once), but
  recorded here because the *decision* not to build partial style support
  around what little calamine exposes was this project's, not calamine's.
- **`WorkbookChange` is reserved and permanently empty** (RFC-021).
  Defined-name and sheet-visibility differences are real and are surfaced
  — but only as `Diagnostic` entries, never as structured, matchable
  `WorkbookChange` values. `compare_workbook_metadata` always runs
  unconditionally; there is no mode to disable it.
- **Hyperlinks, merged regions, tables, and pivot tables are not compared,
  despite calamine 0.36 exposing the data for all four**
  (`Xlsx::hyperlinks_by_sheet_name`, `Xlsx::merged_regions`,
  `Xlsx::table_by_name`, `Xlsx::pivot_tables`). This crate simply does not
  call those APIs yet — a different cause from the charts/images/comments
  group above, and worth keeping distinct: this one is only a matter of
  someone doing the work, not of data that doesn't exist. `WorkbookObjectChange`
  stays empty for these too, for now.

### Unreachable-by-construction

Public API surface that exists — for forward compatibility, or because the
underlying enum is shared across input formats this crate doesn't read —
but that no `.xlsx` input this crate accepts can actually produce today:

- **`CellValue::Integer`, `::Duration`, and `::Unsupported`.** Calamine's
  `Xlsx` reader routes every numeric cell through `Data::Float`, never
  `Data::Int`; `Data::DurationIso` (the only source `::Duration` maps
  from) is emitted by calamine's ODS reader only, and this crate opens
  workbooks exclusively via `calamine::Xlsx`; nothing in this crate
  constructs `::Unsupported` at all. Retained against future input-format
  support, not as live cases — six of `CellValue`'s nine variants are
  reachable in practice.
- **`ReadErrorKind::Other`.** Reserved for an I/O failure mid-read, but
  `open_reader`/`open_bytes` fully drain their input into an owned buffer
  before any calamine parsing begins, so every sheet read operates on an
  in-memory cursor — there is no I/O left to fail against at that point.
  Unreachable by construction, not merely by current calamine behaviour;
  it stays unreachable even if this crate's bytes-input path moves to a
  borrowing reader in the future, since a borrowed slice is still a cursor.

### Resource limits

`DiffOptions::default()` leaves every *linear* bound (`max_sheets`,
`max_cells_read`, `max_cells_compared`, `max_diffs_returned`) unset —
their cost scales predictably with input the caller chose to open, so
bounding them by default would break working code for no safety benefit
the caller couldn't have anticipated. `Limits::hardened()` sets all of
them, for input the caller doesn't trust. Full reasoning, the specific
default values, and what remains genuinely unprotected either way: the
[threat model](maintainers/threat-model.md#the-bounds-themselves-limits).

**One behaviour change worth naming directly rather than only linking:**
`max_cells_compared` used to bound the number of *differences found*, not
coordinates visited — a defect fixed in 2.4.0 (M4 unit 04). A
`Limits::hardened()` caller comparing a large, low-difference-rate
workbook, which succeeded under 2.3.0's enforcement, can now return
`LimitExceeded` under 2.4.0 — the limit finally doing what it was always
documented to do, and a real compatibility event for anyone relying on the
old (broken) behaviour.

### RFCs that shipped in part

Nine RFCs are `Implemented` for their core design but carry a named,
specific gap in their `Status` field — not "mostly done," but a stated
remainder. Reading the RFC's own Status line is the authoritative source;
this list exists so a reader doesn't have to open nine files to get the
inventory:

| RFC | Gap |
|---|---|
| [007](../../rfcs/done/007-typed-cell-values-and-normalization.md) | Three of nine `CellValue` variants unreachable (above) |
| [014](../../rfcs/done/014-serde-feature-and-stable-report-schema.md) | `Deserialize` not implemented (above) |
| [017](../../rfcs/done/017-v1-to-v2-migration-guide-and-adapter.md) | No JSON section and no ForskScope-adapter example in the migration guide. Its code blocks *are* compiled as of M6 unit 01 |
| [019](../../rfcs/done/019-numeric-date-and-tolerance-comparison-policies.md) | `CellValue::Duration` unreachable, so duration-tolerance comparison is unexercised |
| [020](../../rfcs/done/020-display-formatting-and-number-format-capture.md) | `CellNumberFormat` always `None` (above) |
| [021](../../rfcs/done/021-workbook-metadata-and-defined-name-diffs.md) | `WorkbookChange` reserved (above); defined-name/visibility diffing untested |
| [023](../../rfcs/done/023-non-cell-workbook-objects-and-unsupported-features.md) | `WorkbookObjectChange` always empty (above, with the upstream/deferred split) |
| [024](../../rfcs/done/024-large-workbook-memory-strategy.md) | Cancellation is polled once per sheet pair, not between row chunks or cell batches as the RFC's own acceptance criterion specifies |
| [027](../../rfcs/done/027-benchmark-and-performance-governance.md) | No v1.2-vs-v2 benchmark comparison documentation (moved to M7 — this is measurement work, not documentation) |

---

## Corrections found writing this page

Assembling this inventory surfaced five places where a record elsewhere in
this project disagreed with the current code. They were reported rather than
edited here, because this page's non-change scope forbids touching RFC files —
and **all five have since been corrected** by the architect, who owns the RFC
record. They are kept below because the inventory above is only trustworthy if
the reader can see what it was checked against.

1. **RFC-013's Status said exit code 3 is never emitted.** It was fixed in
   M4 unit 03 (2.4.0). *Corrected — RFC-013 is now `Implemented`, which is why
   it no longer appears in the table above.*
2. **RFC-015's Status said the CLI has no subprocess test.** `tests/cli.rs`
   has existed since M4 unit 03, extended by M5 unit 03. *Corrected — also now
   `Implemented`, and also dropped from the table above.*
3. **RFC-017's Status said the migration guide's 11 code blocks are "not
   compiled or verified anywhere."** M6 unit 01 built exactly that harness.
   *Corrected — RFC-017 stays partial, because the JSON-section and
   ForskScope-adapter gaps it also names are still real.*
4. **RFC-021's Status said `meta.rs`'s code comments "incorrectly claim"
   `WorkbookMetadataMode` works.** M4 unit 01 removed those comments.
   *Corrected surgically — only that clause; the underlying gap
   (`WorkbookMetadataMode` never built) is still real and RFC-021 stays
   partial.*
5. **M6's own handoff README said "thirteen partially-implemented RFCs."**
   M5 closed two (016, 032) after that text was written. *Corrected. The count
   was eleven when this page was written and is **nine** now, because
   correcting items 1 and 2 closed RFC-013 and RFC-015 outright — the table
   above reflects the current state.*

None of these five are limitations or defects in themselves — they are the
record lagging behind work that has already landed, which is the precise
failure mode M4 and M5 exist to catch.

The count moving from eleven to nine *while this page was being reviewed* is
the same failure mode in miniature, and worth leaving visible: a number is only
true as of the moment it was derived. The table above is authoritative for the
list; each RFC's own `Status` field is authoritative for its gap.
