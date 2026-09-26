# RFC-033: Public Model Lexicon

**Status.** Implemented (2.0.0–2.3.0) — **reconstructed 2026-08-16** from the 20
normative citations in `src/` after the original document was found to not
exist anywhere in this repository, its git history, or the v2 planning
package. This is not the original text. It is recovered from the code that
cites it, per M3 track B Handoff 01. See §0 for method and limits.
**Target:** v2.0.0, consolidated and extended through v2.3.0
**Created:** unknown (original); this reconstruction dated 2026-08-16
**Category:** Data model / cross-cutting reference
**Related:** RFC-003, RFC-005, RFC-006, RFC-007, RFC-009, RFC-010, RFC-012,
RFC-018, RFC-019, RFC-020, RFC-021, RFC-023, RFC-026 — see §0 for how this
document relates to each

---

## §0. Provenance and method (reconstruction-specific; not part of the original)

### What happened

`src/` cites "RFC-033" as normative at 20 sites across seven files
(`model.rs`, `options.rs`, `error.rs`, `diff.rs`, `normalize.rs`, `meta.rs`,
`lib.rs`), naming it the *"canonical lexicon"* for the public result model
(`model.rs`'s own module doc comment). No copy of it exists anywhere this
project can search. `rfcs/README.md`'s restoration notes (2026-08-15)
already recorded this as a known gap, undercounting the citations at 11 —
the correct count, verified by `grep -rn "RFC-033" src/ --include="*.rs"`,
is **20**.

### Why reconstruction, not de-citing

The handoff governing this work offered two legitimate resolutions:
reconstruct, or de-cite (redirect each reference to whichever RFC actually
survives — 007 for typed values, 003 for the result model, 005 for
diagnostics, and so on). De-citing was seriously considered, because those
per-topic RFCs (003, 005, 006, 007, 009, 010, 012) do genuinely exist and
do cover adjacent ground.

**They are not sufficient substitutes, and that is why reconstruction is
the right call.** Every one of them was `Created: 2026-06-11` — the
earliest wave of v2 design, before this crate had `CellError`, `CellDateTime`'s
`has_serial` distinction, `DiagnosticKind`'s eleven codes with a stable
`code()` surface, or most of what RFC-035/036 (M2/M3) added. RFC-033 is
cited *alongside* those earlier RFCs throughout `src/` (e.g. `model.rs`:
`"CellValue and components (RFC-007 / RFC-033 §2–§3)"`, `"Cell change model
(RFC-010 / RFC-033 §5)"`) — a pattern, not a coincidence. The most coherent
reading is that RFC-033 was written later, after 032, as a **consolidating
lexicon**: one document giving the *current, as-shipped* shape of the
public model a single numbered structure, while the earlier per-topic RFCs
remain the historical record of *why* each piece was first decided.
De-citing to the 2026-06-11 documents would point every citation at a
source that predates roughly half of what it is supposed to describe.
Reconstructing this document, from code whose complete current shape is
directly readable, recovers real information the June RFCs cannot supply.

### Method

For each of the 20 citation sites, I read the section number(s) implied,
then wrote what the *current* code at that location actually is — not what
I believe it was originally intended to be, and not a description improved
or corrected from what ships. Per the handoff's explicit instruction, no
design rationale is invented anywhere below; where a citation implies a
decision the code doesn't fully explain, that is recorded as a divergence
in §13, not silently resolved either direction.

### What this reconstruction cannot recover

**§7 is missing from the citation evidence entirely.** The cited sections
run §1, §2–§3, §4, §5, §6, §8, §9, §10, §11, §12 — nothing in `src/` cites
§7 anywhere. I looked for a plausible candidate (formula comparison and
row/column alignment are the two adjacent shipped features without their
own RFC-033 section, sitting between §6 "sheet change" and §8
"diagnostics") and declined to guess. Per the handoff's prohibited
shortcuts — do not invent design rationale, say the reconstruction is
incomplete rather than fabricate — **§7 is recorded here as unknown, not
reconstructed.** If it resurfaces, it is a real gap in this document, not
a numbering error to silently close by shifting later sections.

This document also cannot recover **why** any individual field or variant
was shaped the way it was, beyond what the shipped code and its own
comments state. That rationale, where it survives, is in the per-topic
RFCs listed above; this lexicon states *what*, not *why*.

---

## §1. Normalization mapping table

**Source:** `src/normalize.rs`, `normalize_cell_value()`. Every row has a
unit test in that module's `tests` submodule (the comment at the top of
`normalize.rs` states this is "the source of truth").

The single normalization boundary from calamine's `Data` enum to the public
`CellValue` (§2–§3). Calamine types never appear anywhere in the public API
(RFC-026).

| `calamine::Data` | → `CellValue` |
|---|---|
| `Empty` | `Empty` |
| `String(s)` | `Text(s)` |
| `Int(i)` | `Integer(i)` |
| `Float(f)` | `Number(f)` |
| `Bool(b)` | `Bool(b)` |
| `DateTime(dt)` | `DateTime(CellDateTime { serial: dt.as_f64(), is_1904, kind, iso, has_serial: true })` — `kind` is `Time` if `dt.is_duration()`, else `DateTime` if `dt.is_datetime()`, else `Date`; `iso` synthesized only when the `chrono` feature is enabled |
| `DateTimeIso(s)` | `DateTime(CellDateTime { serial: 0.0, is_1904, kind: DateTime, iso: Some(s), has_serial: false })` — `serial` is a placeholder, never a real Excel serial (RFC-019 / D-01) |
| `DurationIso(s)` | `Duration(CellDuration { serial: 0.0, iso: Some(s) })` |
| `Error(e)` | `Error(CellError::<matching variant>)`, 1-to-1 with `calamine::CellErrorType` |

`is_1904` is the workbook-level date epoch flag (`Xlsx::has_1904_epoch()`),
read once per workbook and threaded into every cell (RFC-019 / D-02) — it
is not re-derived per cell.

## §2–§3. `CellValue` and its components

**Source:** `src/model.rs`. Governed originally by RFC-007 (typed values);
this section states the current shape.

```rust
pub enum CellValue {
    Empty,
    Text(String),
    Integer(i64),
    Number(f64),
    Bool(bool),
    DateTime(CellDateTime),
    Duration(CellDuration),
    Error(CellError),
    Unsupported { display: String, reason: String },
}
```

`Integer` and `Number` are kept distinct, reflecting calamine's `Data::Int`
/ `Data::Float` split. Default comparison treats `Integer(1)` and
`Number(1.0)` as a `TypeChanged` difference; cross-type numeric equality is
opt-in (§4, RFC-019).

**Component types**, all `#[non_exhaustive]`:

- `CellDateTime { serial: f64, is_1904: bool, kind: DateTimeKind, iso: Option<String>, has_serial: bool }`.
  `has_serial` distinguishes a genuine Excel serial from the `0.0`
  placeholder used when only an ISO string is available — a real date can
  itself serialise to `0.0`, so the placeholder is not otherwise
  distinguishable (added for D-01, RFC-035 Handoff 05).
- `DateTimeKind { DateTime, Date, Time }`.
- `CellDuration { serial: f64, iso: Option<String> }`.
- `CellError { Div0, NA, Name, Null, Num, Ref, Value, GettingData, Other(String) }`
  — 1-to-1 with `calamine::CellErrorType`; `Other` is forward-compatibility
  headroom, not currently constructed by any code path.
- `display_string()` (also aliased `display_default()`, RFC-020) — a
  human-readable rendering, never used as an equality key.

**Divergence recorded, not resolved here — see §13, item 1:** `Integer`,
`Duration`, and `Unsupported` cannot occur through any `.xlsx` input this
crate opens. This is not new information — it was found and recorded
during M3 track A (`Duration` in RFC-035 Handoff 05 §6; `Integer` and
`Unsupported` in RFC-030 Handoff 01's coverage-dimension report) — but §2's
citation implies these are ordinary reachable variants, and they are not.
Recorded here so this lexicon does not silently repeat the overclaim.

## §4. Equality policy

**Source:** `src/normalize.rs`'s `tests` module (comment: `"RFC-033 §4
equality policy tests"`), and the comparison logic in `src/compare.rs`.

The base rule: same-variant `CellValue`s compare by content; different
variants are a type mismatch by default. Two policies loosen this, both
opt-in via `ValueCompareOptions` (§11, originally RFC-019):

- `NumericTypePolicy::CompareMathematicalValue` — `Integer(1)` and
  `Number(1.0)` compare equal by mathematical value instead of
  `TypeChanged`. Default is `PreserveType` (they differ).
- `TypeMismatchPolicy::CompareDisplayString` — any cross-type mismatch
  compares by `display_string()` instead of always being `TypeChanged`.
  Default is `Different`.

Date/time equality (RFC-019, extended by D-01/D-02 in RFC-035 Handoff 05):
two values with `has_serial: true` on both sides compare by
`(serial, is_1904, kind)`; two with `has_serial: false` on both sides
compare by `iso`; a `true`/`false` mismatch is never silently equal.
`DateComparePolicy::NormalizeEquivalentDateTimes` additionally reconciles
two `has_serial: true` values recorded under different `is_1904` epochs
(1900 vs. 1904 systems, a fixed 1462-day offset) when `ExactRepresentation`
(the default) would otherwise report them different.

## §5. Cell change model

**Source:** `src/model.rs`. Governed originally by RFC-010.

```rust
pub struct CellDiff {
    pub address: CellAddress,
    pub value: Option<ValueChange>,
    pub formula: Option<FormulaChange>,
    pub format: Option<FormatChange>,   // reserved, always None — RFC-022 blocked upstream
    pub diagnostics: Vec<Diagnostic>,
}
```

**One `CellDiff` per logical address.** A value change and a formula change
at the same address are facets of one change, in independent `value` and
`formula` sub-fields — never two separate entries. This is stated in
`model.rs` as the intended consumer model, and the `output::view` row
projection follows the same rule.

`change_kind()` (→ `CellChangeKind { Added, Removed, Modified }`) is
derived from the sub-fields, never stored, and is documented as **stable
API**: Added means every present sub-change has an empty/absent `old`
side; Removed means every present sub-change has an empty/absent `new`
side; otherwise Modified.

`ValueChange { old: CellValue, new: CellValue, reason: ValueDifferenceKind }`.
`ValueDifferenceKind` has six variants: `TypeChanged`, `ContentChanged`,
`NumericOutsideTolerance`, `DateTimeChanged`, `ErrorKindChanged`,
`DisplayStringChanged`.

`FormulaChange { old: Option<FormulaText>, new: Option<FormulaText> }` —
`None` in either side means the formula was added or removed.
`FormulaText { raw: String, normalized: Option<String> }` — `normalized`
is always `None`: no formula normaliser exists (M10 unit 06 removed the `NormalizedText` mode that
would have filled it; §11).

`FormatChange` is a zero-field placeholder, reserved for RFC-022, which
remains blocked: calamine 0.36 does not expose a cell-style API.

## §6. Sheet change classification

**Source:** `src/model.rs`. Governed originally by RFC-009.

```rust
pub enum SheetChange {
    Unchanged,
    Modified,
    Added,
    Removed,
    Moved,
    Renamed { confidence: MatchConfidence, reason: SheetMatchReason },
    RenamedAndMoved { confidence: MatchConfidence, reason: SheetMatchReason },
}
```

Names and indices live on `SheetDiff.old_sheet` / `.new_sheet`, not
duplicated inside the variant payloads. `MatchConfidence { Exact, High,
Medium, Low }`; `SheetMatchReason { SameIndex, SoleRemainingPair }`.

**Corrected M10 unit 01 (3.0.0):** this lexicon recorded `SheetMatchReason {
ExactName, IndexAndContent, ContentSimilarity }`. Two of the three were never
constructed — `ExactName` could not be, since the enum appears only inside `Renamed` and
`RenamedAndMoved` and a rename is not an exact-name match — and the third,
`IndexAndContent`, asserted that cell content had been compared at all three sites that
built it, when the matcher inspects no cell content anywhere. The enum is now
`{ SameIndex, SoleRemainingPair }`: the two things that actually happen (the sheets sat
at the same tab position; or they were the only unmatched sheets left, with different
positions). Classification, confidence and matching are unchanged.

`Moved` — name-matched, cell-identical, but the tab index differs between
workbooks — was, until RFC-036 (M3 track A), never distinguished from
`Unchanged` by any assertion in the test suite, though the variant and its
derivation logic (`src/matcher.rs`) existed and were exercised. Now
covered by the `sheet_reordered` corpus scenario.

## §7. *(unknown — not recoverable from citation evidence; see §0)*

## §8. Diagnostics

**Source:** `src/model.rs`. Governed originally by RFC-005.

```rust
pub struct Diagnostic {
    pub severity: Severity,           // Info, Warning (no Error: RFC-005's two-tier model; removed M10 unit 07)
    pub kind: DiagnosticKind,
    pub location: DiagnosticLocation, // stage, sheet_order, sheet_name, address
    pub message: String,              // display only, never a matching key
}
```

**`DiagnosticLocation` (M10 unit 04):** `sheet_order` and `sheet_name` are set together or not at all — a
diagnostic about a particular sheet names it (for a matched pair, the new workbook's sheet, else the old
one's); one that is not about a sheet leaves both `None`, meaning "not about a sheet", not "not recorded". The
rule and its history are in RFC-005; the sheet-level diagnostics sit in `SheetDiff::diagnostics`, the
workbook-level ones in `WorkbookDiff::diagnostics`.

`DiffStage { Open, Metadata, Match, Read, Normalize, Compare, Aggregate }`. **Live:**
`Metadata`, `Match`, `Read`, `Compare` — diagnostics are attributed to these. **Vocabulary,
never produced:** `Open`, `Normalize`, `Aggregate` — stages the pipeline has, to which no
diagnostic is attributed (documented in `model.rs` as *cannot occur*; guarded by
`tests/diagnostic_codes.rs`). Decided 2026-09-25 (RFC-037 §3.4) to stay.

`DiagnosticKind` is `#[non_exhaustive]` with seven variants (eleven as of 2.3.0 through
2.6.0): `FormulaUnavailable`, `AmbiguousSheetMatch { candidates }`,
`UnsupportedWorkbookFeature { feature }`, `UnsupportedWorkbookMetadata { category }`,
`DefinedNameScopeUnknown`, `AlignmentBoundExceeded { limit, observed }` (RFC-035),
`DuplicateAlignmentKey { old_count, new_count }` (RFC-035). **All seven are live** — the engine
produces each — which is asserted by `tests/diagnostic_codes.rs`.

**Extended f130 (3.1.0):** an eighth, `MissingAlignmentKey { old_count, new_count }` (code `missing_alignment_key`), is
raised under `RowKey` alignment when rows have no cell in any key column (RFC-011's f130 correction). **All eight are
live**, asserted by the same test.

**Corrected M10 unit 02 (3.0.0):** this lexicon listed four more, none of which any code
path could construct: `FormulaCachedValueUnverified`, `UnsupportedCellValue { detail }` (its one
use, the duplicate-alignment-key condition, moved to `DuplicateAlignmentKey` in RFC-035 and
nothing replaced it), `DateTimeNotNormalized`, and `LimitTruncatedCells { limit, observed }`
(structurally impossible: a cell limit returns `Err(LimitExceeded)`, so there is no truncation
to report). Removed in 3.0.0 along with their codes `formula_cached_value_unverified`,
`unsupported_cell_value`, `datetime_not_normalized` and `limit_truncated_cells`.
`SourceKind::Unknown` (§ the `SourceDescription` type) was removed with them: the crate builds
every `SourceDescription` itself at one of three entry points.

**`DiagnosticKind::code()` is the stable programmatic surface**, explicitly
documented as such in `model.rs`: match on the code string, not the
`#[non_exhaustive]` enum variant, since new variants may arrive in a minor
release but an existing code string is never renamed within a major
version. The seven current codes are listed in `model.rs`'s own doc table
and are not reproduced here to avoid a second copy drifting out of sync —
that table is the authoritative one.

**`DiagnosticSummary` (M10 unit 07):** `{ warnings, info }` — there is no `errors` count, because a diagnostic is never
fatal (RFC-005), so it could only ever be zero. `Severity` is `{ Info, Warning }` with `Info < Warning`, on which
`min_severity`'s `>=` depends. The CLI's summary line is `diagnostics: M warning(s)` (absent when M is 0).

**Public values the engine never produces, and stay (M10 unit 02; decided 2026-09-25, RFC-037
§3.4).** The rule that separates them from the removals above: a value the *engine* would have to
produce and never does is a dead arm and goes; a value a *caller* may hand us, or that names a
stage the pipeline really has, is vocabulary and may be wider than today's usage. Each is
documented in `model.rs` / `objects.rs` and pinned by a documentation guard in
`tests/diagnostic_codes.rs`.

| Value | Kind | Note |
|---|---|---|
| `DiffStage::Open`, `::Normalize`, `::Aggregate` | pipeline vocabulary | *cannot occur*; no diagnostic is attributed to these stages |
| `DisplaySource::ReaderProvided`, `::ApplicationProvided` | **caller vocabulary** | reachable: `CellDisplay::new` is public with a public `source`; the crate itself only ever sets `SheetsDiffDefault` |
| `ObjectCompareMode::CompareAvailable` | honest reservation | selectable; behaves as `WarnIfPresent` |
| `CellError::Other(String)` | forward-compatible catch-all | *cannot occur*: calamine's error conversion is an exhaustive match over eight kinds |
| `CellValue::Integer`, `::Duration` | unreachable through `.xlsx` | *do* have construction sites (`normalize.rs`, from `Data::Int` / `Data::DurationIso`) on input paths the `.xlsx` reader never takes — RFC-037 §3.4 listed them as having none |
| `CellValue::Unsupported` | forward-compatible catch-all | no construction site anywhere |

## §9. Errors

**Source:** `src/error.rs`. Governed originally by RFC-005.

`SheetsDiffError` is `#[non_exhaustive]` with six variants:
`OpenWorkbook { side, source, kind, inner }`, `ReadSheet { side, sheet,
kind, inner }`, `EncryptedWorkbook
{ side }`, `InvalidOptions { detail }`, `Cancelled`, `LimitExceeded { limit:
LimitKind, observed: u64 }`.

**Corrected M10 unit 07 (3.0.0):** this lexicon listed eight. `UnsupportedFormat { side, detail }` (a duplicate of
`OpenWorkbook { kind: NotXlsx }`) and `Internal { detail }` (an escape hatch never needed) were never constructed and were
removed in 3.0.0, as was `OpenErrorKind::Locked`; `Severity::Error` and `DiagnosticSummary::errors` were removed with them
(§8; RFC-005 states why an error *severity* cannot exist). **Of the six left, one is also never constructed:**
`InvalidOptions`, since M10 unit 06 left `validate()` with no arm — kept as the seam for the next option that can be unusable.

`OpenErrorKind { NotFound, PermissionDenied, NotXlsx, Corrupt,
Other }`; `ReadErrorKind { SheetNotFound, MalformedSheet, Other }`. The
`calamine::XlsxError` source is preserved behind
`std::error::Error::source()` (a boxed, opaque `CalamiLineError`) and never
appears in any public variant type, per RFC-026.

## §10. Limits

**Source:** `src/options.rs` (`Limits`), `src/error.rs` (`LimitKind`),
`src/diff.rs` (the `max_sheets` check). Governed originally by RFC-012;
substantially extended by RFC-035 (M2).

```rust
pub struct Limits {
    pub max_sheets: Option<u32>,
    pub max_cells_read: Option<u64>,
    pub max_cells_compared: Option<u64>,
    pub max_diffs_returned: Option<u64>,
    pub max_alignment_product: Option<u64>,  // RFC-035
    pub max_input_bytes: Option<u64>,        // RFC-035
}
```

`LimitKind { Sheets, CellsRead, CellsCompared, DiffsReturned,
InputBytes }` — the values a `LimitExceeded` error can name.

**Corrected M10 unit 05 (3.0.0):** `max_cells_read` and `DiffMetrics::cells_read` are one number
(one accumulator in `src/diff.rs`) and were the **area of the bounding box** of each sheet's populated
cells through 2.6.0 — the memory a dense read allocated, spent by nothing since the read streamed. Both
now count **populated cells retained**. The limit rejects a subset of what it rejected before (a box
contains its cells); the sparse-box workbook is no longer refused. `LimitExceeded.observed` for
`CellsRead` is the running count at the breaking cell, i.e. `max + 1`.

Split by default behaviour (RFC-035 §5.1): the four linear fields default
to `None` (unbounded) — their cost scales predictably with input the
caller already chose to open. `max_alignment_product` (default
25,000,000, empirically measured — RFC-035 §9) and `max_input_bytes`
(default 500 MiB) default to `Some`, because their unbounded cost is
superlinear or is incurred before any comparison logic can observe it.
Exceeding `max_alignment_product` degrades the sheet to positional
comparison with an `AlignmentBoundExceeded` diagnostic (§8) — it never
errors. Exceeding any other limit, including `max_input_bytes`, returns
`SheetsDiffError::LimitExceeded`. `Limits::hardened()` sets a concrete,
conservative value on every dimension for callers who do not trust their
input (RFC-035 §5.3).

## §11. `DiffOptions`

**Source:** `src/options.rs`. Governed originally by RFC-006.

```rust
pub struct DiffOptions {
    pub comparison: ComparisonOptions,
    pub matching: MatchingOptions,
    pub limits: Limits,               // §10
    pub execution: ExecutionOptions,
    pub diagnostics: DiagnosticOptions,
    pub output: OutputOptions,
}
```

Constructed via `DiffOptions::default()` or `DiffOptions::builder()` →
`DiffOptionsBuilder`, a consuming fluent builder whose `.build()` runs
`DiffOptions::validate()` before returning.

**Builder coverage — final state (M10 unit 03; 19 leaves after unit 06), audited by script and by test.** All **19** leaf
options in the tree have a builder method of their own; none depends on a whole-struct setter;
`DiffOptions`'s doc comment says so without an exception. The history, so the counts are not quoted
again: before M8 unit 07 the audit found 16 with a method, `alignment` and `max_cells_read` reachable
only through a whole-struct method, and `date` and `min_severity` through none (the handoff had said
"two", with the wrong two); unit 07 added `alignment` and `min_severity`; **unit 03 added
`date_compare_policy` and `max_cells_read`**, removed the duplicate `number_compare` and the
whole-struct `build_with_matching` (which replaced `sheet_matching` too, silently), and removed the two
redundant `#[allow(dead_code)]` on `AlignmentMode`. Every field is still `pub`, so any option can also be
set by assignment. `tests/builder_coverage.rs` sets each leaf through the builder, reads it back, checks
that each setter changes its own leaf and no other, and fails when an option is added without a setter.

**Extensibility (M10 unit 08, 3.0.0) — new options are additive from 3.0.0 on.** All eight options structs in this
tree are `#[non_exhaustive]`, as every result struct always was: an option added to any of them is an added field, not a
breaking change, and the coverage audit (`src/builder_coverage.rs`, which destructures each struct exhaustively and so
must live inside the crate) fails to compile until the new option has a builder setter. Through 2.x the options were the
one part of the public surface that was not extensible — an oversight, not a decision — so every option ever added was a
break, and RFC-011's column alignment, RFC-022's formatting and a formula normaliser each threatened to wait for 4.0.0.
Construction from outside the crate is the builder, or `Default::default()` plus field assignment; a struct expression —
**including `..Default::default()`** — is rejected (`E0639`).

**Naming rule (one sentence).** *A setter for a `…Policy`-typed option is named for its type in
snake_case — `number_compare_policy`, `numeric_type_policy`, `type_mismatch_policy`, `date_compare_policy` —
and every other setter already carries the name of its option or its type and keeps it.* It required
exactly one removal (`number_compare`, which was `number_compare_policy` under a second name) and no
rename. **Known residue, not fixed here:** the `…Mode`-typed setters do not follow one convention
(`formula_compare`, `sheet_matching`, `alignment` drop `Mode`; `object_mode`,
`execution_mode` keep it). Making them uniform is a rename of surviving setters — a break each — and was
not required to settle the duplicate.

`ComparisonOptions { value: ValueCompareOptions, formula:
FormulaCompareMode, include_formula_cached_values: bool }`
(`FormulaCompareMode { RawText, Ignore }`; M10 unit 06 removed `format:
FormatCompareMode` and the two formula modes that could only fail — see the
note below). `ValueCompareOptions { number: NumberComparePolicy,
numeric_type: NumericTypePolicy, date: DateComparePolicy, type_mismatch:
TypeMismatchPolicy }` — the policies described in §4.

`MatchingOptions { sheet_matching: SheetMatchingMode, alignment:
AlignmentMode }` (RFC-009, RFC-011) — not itself cited to an RFC-033
section anywhere in `src/`, included here only because it is a direct
field of the §11 struct.

**Resolved M10 unit 06 (3.0.0) — the §13 item 2 divergence.** `validate()` used to reject
`FormulaCompareMode::NormalizedText` and `RawAndNormalized` (`InvalidOptions`, "no formula normaliser is
implemented yet") and any `FormatCompareMode` other than `Ignore`; `comparison.format` was read in exactly one
place — that rejection. Three settings that could only fail and a field that could only hold its default. All are
**removed** (`FormulaCompareMode` keeps `RawText` and `Ignore`; `FormatCompareMode` and `ComparisonOptions::format` are
gone). `validate()` now has **no arm**: no combination of options is invalid today. It stays as the seam a future
option that can be set to something unusable would use, and `SheetsDiffError::InvalidOptions` stays as what it would
return — but nothing constructs that variant today (reported in the M10 unit 06 review request). RFC-022's format
option returns when RFC-022 is implemented, additively (M10 unit 08).

`ExecutionOptions { progress, cancellation, mode: ExecutionMode }` —
`ExecutionMode` has one variant (`Sequential`); a parallel mode was
removed (RFC-025) and the type kept only so a future re-introduction needs
no API break.

`DiagnosticOptions { min_severity: Option<Severity> }`, set by
`DiffOptionsBuilder::min_severity(Option<Severity>)` (M8 unit 07) or by assigning the field.
**Note (M8 unit 02,
2.6.0):** until then nothing read this field; a caller who set it observed no
change. It is a *collection* filter — a diagnostic below it is not kept, in the
workbook's vector or any sheet's — applied once in `src/diff.rs`, and
`DiffSummary::diagnostics` and `DiffMetrics::diagnostics_emitted` count what was
kept. Its default is `None` (collect everything). It is distinct from a display
threshold: the CLI's `--no-warnings` and the unified renderer's `>= Warning`
threshold are display controls that leave the counters alone.

`OutputOptions { objects: ObjectCompareMode }` (RFC-023) — default
`WarnIfPresent`.

## §12. Entry points and top-level result

**Source:** `src/lib.rs` (entry points), `src/model.rs` (`WorkbookDiff` and
the reserved change types), `src/meta.rs` (the `WorkbookChange` reserved
field, explicitly cited: `"RFC-021, RFC-033 §12 reserved field"`).

Six public entry points, each `compare_*` and `compare_*_with_options`
pair over three input kinds:

- `compare_paths` / `compare_paths_with_options` — `impl AsRef<Path>` ×2.
- `compare_bytes` / `compare_bytes_with_options` — `impl AsRef<[u8]>` ×2.
- `compare_readers` / `compare_readers_with_options` — `impl Read + Seek` ×2.

All six return `Result<WorkbookDiff, SheetsDiffError>`.

```rust
pub struct WorkbookDiff {
    pub old: WorkbookSideInfo,
    pub new: WorkbookSideInfo,
    pub sheets: Vec<SheetDiff>,
    pub workbook_changes: Vec<WorkbookChange>,   // always empty — RFC-021
    pub object_changes: Vec<WorkbookObjectChange>, // always empty — RFC-023
    pub diagnostics: Vec<Diagnostic>,
    pub summary: DiffSummary,
    pub metrics: DiffMetrics,                     // RFC-024/027
}
```

`WorkbookChange` and `WorkbookObjectChange` are zero-field placeholders,
`#[non_exhaustive]` so they can be populated additively without a breaking
change. As of 2.3.0, RFC-021 and RFC-023 both surface their findings
exclusively through `diagnostics` (§8); the structured variants these two
types are reserved for have never been populated in any shipped version.

**`address::CellAddress` and `address::ComparedRange` are `#[non_exhaustive]` (M10 unit 09, 3.0.0).** `ComparedRange` is a **result**
field (`SheetDiff::compared_range`) and `CellAddress` is `CellDiff::address`; through 2.6.0 neither was marked, so this lexicon's
statement above that the result types are `#[non_exhaustive]` was true of the structs in `model.rs` and not of every struct a result
contains (RFC-037 §3.9 repeated the overstatement). Both are made by their public constructors — `CellAddress::new(row, col)`
(bounds-checked, `Option`) and `ComparedRange::empty()` / `::union(..)` — not by literal; their fields stay public to read and assign.
`output::json::to_json` / `to_json_pretty` return `String` (RFC-014).

`SheetDiff { old_sheet, new_sheet, change: SheetChange, cell_diffs:
Vec<CellDiff>, compared_range: ComparedRange, alignment_summary:
Option<AlignmentSummary>, diagnostics: Vec<Diagnostic>, summary:
SheetSummary }`. `cell_diffs` is sorted by `(row, col)`.
`alignment_summary` is `None` when `AlignmentMode::Positional` (the
default).

`DiffSummary`, `SheetSummary`, `DiagnosticSummary`, `DiffMetrics` are all
plain count-rollup structs, `#[non_exhaustive]`, cheap to clone —
`WorkbookDiff`'s own doc comment demonstrates dropping the (potentially
large) `sheets[..].cell_diffs` vectors while retaining `summary`,
`metrics`, and each sheet's `change` at a consumer's adapter boundary.

---

## §13. Divergences found during reconstruction

Per the handoff's instruction: where a citation implies a decision the
code no longer reflects, that is a finding, recorded here rather than
smoothed over in the section text above.

1. **§2–§3: three `CellValue` variants are unreachable through any
   `.xlsx` input.** `Integer`, `Duration`, `Unsupported` — already found
   and recorded (RFC-035 Handoff 05 §6 for `Duration`; RFC-030 Handoff 01
   for `Integer` and `Unsupported`). Not new here; cross-referenced so this
   lexicon doesn't silently present all nine variants as equally live.
   Still an open design question per RFC-036 §8 (deferred, not decided):
   whether these stay, documented as reserved, or the model shrinks —
   which would be breaking.
2. **§11: `FormulaCompareMode` and `FormatCompareMode` each have variants
   that can be selected but never successfully built.** *(Resolved M10 unit 06: the unusable variants, the
   `FormatCompareMode` enum and `ComparisonOptions::format` were removed; see §11.)* `validate()` rejected
   `NormalizedText`/`RawAndNormalized` and anything but
   `FormatCompareMode::Ignore`. This was documented at the call site (the
   error messages said why), so it was not a *silent* trap — but a citation
   describing §11 as "the options surface" without this caveat would
   have overstated what was usable.
3. **§0 (not a numbered lexicon section, but worth recording plainly):
   `rfcs/README.md`'s restoration notes undercounted the citation sites at
   11; the true count, by direct grep, is 20.** Corrected in this
   document and in `rfcs/README.md` in the same change.
4. **§7 has no citation evidence anywhere in `src/`.** Recorded as unknown
   in §0, not guessed at. This is the one point where reconstruction is
   genuinely incomplete, not merely terse.

No divergence was found in §1, §4, §5, §6, §8, §9, §10, or §12 — the code
at each of those citation sites is internally consistent with what the
comment nearby implies it should be.

## §14. What this document is not

This is a **lexicon**, not a design record. It states the current shape of
the public model, its section numbering as the code already cites it, and
where that shape and its citations diverge. It does not restate *why* any
individual decision was made — that belongs to RFC-003, RFC-005, RFC-006,
RFC-007, RFC-009, RFC-010, RFC-012, RFC-018, RFC-019, RFC-020, RFC-021,
RFC-023, and RFC-026, each of which remains the record of its own
decision. Where this document and one of those disagree on a current
field shape, this document (checked directly against `src/` on
2026-08-16) is the more current source; the older RFC is the historical
one.
