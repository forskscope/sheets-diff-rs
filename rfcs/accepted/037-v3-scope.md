# RFC-037: v3 Scope — the breaks, and the closed list

**Status.** **Accepted** by the owner 2026-09-25. **§3 reopened and closed again
the same day** at the owner's direction, adding §3.8, §3.9 and the decisions for
§3.3 and §3.6 — once, rather than three amendments to a list whose value is
being closed. It is closed again now. Scope was widened the same day
(§3.7, §5) before acceptance, after 2.6.0 shipped and a planned 2.7.0 was
dropped. **§3 is now closed** — nothing joins it without the owner reopening it.
The three questions in §7 remain open and block the handoffs that depend on them,
not the RFC.
**Target:** 3.0.0
**Created:** 2026-09-25
**Author:** high-capability model (architect / design authority)

---

## 1. Why a major, and why now

M8 exists because public surfaces promise what the engine does not do. Four of
its findings cannot be fixed inside 2.x, because the fix is to **remove or
rename a public item** — and cargo hands `^2` users a new minor automatically,
so a deleted name is a build failure they did not ask for. Adding is minor;
removing is major. That is the whole of the constraint.

The alternative — document each lie in place and carry it — was the plan until
2026-09-25. The owner's objection was correct: **a major costs almost nothing
right now**, because there is no production use. It will cost more later, and
the list of things needing one is still short. This is the cheapest this will
ever be.

## 2. The rule this list is built on

Not "everything unused". The test is **whether a competent reader learns the
truth from the type**:

- **Honest reservations stay.** `ExecutionMode` has one variant and says so,
  with RFC-025's reasoning. `CellFormat`, `WorkbookChange`, `ObjectChange` are
  documented as reserved and always empty. These exist *to avoid future breaks*
  and are doing their job. Removing them would be a break now and another break
  when they are filled.
- **False or unreachable claims go.** A variant that describes an outcome the
  engine never produces, or an option that can only return an error, teaches the
  reader something untrue. Those are removed.

## 3. The closed list

### 3.1 `SheetMatchReason` — rename and remove (M8 O1)

Verified: three variants, **one** ever constructed, and it is wrong at all three
sites.

| Variant | Construction sites | Disposition |
|---|---|---|
| `ExactName` | none, and **structurally impossible** — the enum appears only inside `SheetChange::Renamed` / `RenamedAndMoved`, and a rename is not an exact-name match | **Remove** |
| `ContentSimilarity` | none; the matcher inspects no cell content anywhere | **Remove** |
| `IndexAndContent` | `matcher.rs:143`, `:151`, `:212` — **every rename this crate reports** | **Replace** with honest variants |

The three sites are two different things:

- `:143` and `:212` pair on index alone (`:212` is inside a function named
  `index_match`). → **`IndexOnly`**
- `:151` pairs the last unmatched sheet on each side, with **unequal** indices.
  Neither index nor content matched. → **`SoleRemainingPair`** (name to be
  settled in the handoff)

### 3.2 `DiagnosticKind` — remove four unreachable variants (M8 A4)

Verified repo-wide, including `#[cfg(test)]` modules: each appears only in its
own declaration and its `code()` arm.

| Variant | Why it cannot occur |
|---|---|
| `FormulaCachedValueUnverified` | nothing constructs it; **RFC-018 §146 claims the engine "may emit" it** |
| `UnsupportedCellValue { detail }` | its one use moved to `DuplicateAlignmentKey` (RFC-035); nothing replaced it |
| `DateTimeNotNormalized` | nothing constructs it |
| `LimitTruncatedCells { limit, observed }` | **structurally impossible** — limits return `Err`, so there is no truncation to report |

`code()`'s doc tells callers these strings are the stable programmatic surface
and to match on them. Four of them can never arrive.

### 3.3 Options that can only fail — remove (new, 2026-09-25)

`DiffOptions::validate()` rejects these unconditionally. They are public,
constructible, documented as if usable, and every caller who selects one gets
`InvalidOptions`:

- `FormulaCompareMode::NormalizedText`
- `FormulaCompareMode::RawAndNormalized`
- `FormatCompareMode::NumberFormatOnly`
- `FormatCompareMode::AllAvailable`

`NormalizedText`'s doc says *"Requires a normaliser feature; returns
`InvalidOptions` if selected without one."* **There is no such feature and none
is planned** — so the sentence describes a configuration that cannot exist.

**Decided 2026-09-25: `FormatCompareMode` and `ComparisonOptions::format` are
removed entirely.**

The question was whether the enum survives as a single-variant reservation. It
should not, because the field is worse than the variants: **`comparison.format`
is read in exactly one place — `validate()`, to reject every value except its
default.** It is a public option whose only usable setting is the one a caller
gets by not setting it. A single-variant reservation would preserve exactly
that.

RFC-022 is `accepted` and will reintroduce both when it is implemented. **That
reintroduction is additive only because of §3.9** — without it, re-adding a
field to `ComparisonOptions` would be a second break. The two decisions are
linked and both are in 3.0.0.

`FormulaCompareMode` keeps `RawText` and `Ignore`.

### 3.4 Other never-constructed public values — decide, then act (new)

**Corrected 2026-09-25 (M10 unit 02).** This list was introduced as "verified as
having zero construction sites anywhere in `src/`". That is true of eight of the
ten values and **false of `CellValue::Integer` and `::Duration`**, which are
constructed at `src/normalize.rs:28` (from `Data::Int`) and `:87` (from
`Data::DurationIso`) — arms that exist for inputs calamine's `Xlsx` reader never
emits. *Unreachable through this crate's inputs* is not the same as *never
constructed*, and the sweep collapsed the two. The decision is unchanged: M4
unit 01 kept them for exactly that reason and documented it per variant.

Note also that these are **ten values in six rows**; the count of six is a count
of rows.

The values, with zero production construction sites unless noted:

| Value | Note |
|---|---|
| `DiffStage::Open`, `::Normalize`, `::Aggregate` | `DiagnosticLocation.stage` never takes these three |
| `DisplaySource::ReaderProvided`, `::ApplicationProvided` | only `SheetsDiffDefault` is ever set |
| `SourceKind::Unknown` | `open.rs` sets `Path`, `Bytes`, `Reader` only |
| `ObjectCompareMode::CompareAvailable` | RFC-023's compare mode is unimplemented |
| `CellError::Other(String)` | appears only in a `Display` arm; nothing constructs it |
| `CellValue::Integer`, `::Duration` | **constructed** at `normalize.rs:28` / `:87`, from calamine variants the `Xlsx` reader never emits; documented by M4 unit 01 |
| `CellValue::Unsupported` | zero sites; documented by M4 unit 01 |

**These are not automatically removals.** `DiffStage` names pipeline stages that
exist; `CellValue`'s three carry M4's reasoning for keeping them. Each needs a
one-line decision in the handoff against §2's rule — the sweep's job is to make
sure none is missed, not to prejudge it.

### 3.5 `cells_read` — change what it counts (M8 A6, moved here)

`DiffMetrics::cells_read` reports **bounding-box area**, not cells read:
`sparse_range` reports **5,200** against `cells_compared`'s **2**.

This was M8 unit 05 and moves to v3, where it belongs: changing what a public
metric counts is a semantic break, and doing it here means it stops being a
golden-churning problem inside a minor. Every golden moves; each must be shown
to differ only in `cells_read`.

**Corrected 2026-09-25 (M10 unit 05, at review).** Two claims above are wrong.
**"Every golden moves" is false** — three of eighty did (`sparse_range`
5200→4, `chart_sheet` 8→6, `formula_shifted_origin` 6→4); the other sixteen
scenarios have no empty position inside their box, so the two definitions
coincide. And the extension below said the bound's strictness changes **in both
directions**; it changes in **one**. A bounding box contains every cell in it,
so `populated ≤ area` always — the new bound refuses a strict subset of what
the old one refused, and **no workbook passed in 2.6.0 and fails now**. Both
errors were the architect's, both found by the implementer.

**Extended by the owner 2026-09-25, after the unit was scoped.** This section
asked only that the *metric* change. It cannot: `DiffMetrics::cells_read` and
`Limits::max_cells_read` are one accumulator in `src/diff.rs`, so the metric's
meaning and a documented security preset's bound are the same number.
**Both now count populated cells** (option (a) of three put to the owner).

The reason the alternatives lost is worth keeping: before f123,
`worksheet_range` allocated a dense `Range` whose size *was* the bounding-box
area, so bounding the area bounded the memory. Streaming removed that
allocation — `src/diff.rs` records it as *"memory no longer tracks it"* — and
the bound has guarded nothing since. Keeping it (options (b) and (c)) would
preserve exactly the kind of value this milestone exists to remove.

**Consequence to be stated, not discovered:** a workbook with a vast box and few
populated cells is no longer rejected by `max_cells_read`. It is cheap to read,
so this is a correction; the threat model must say so rather than let the case
drop quietly. **Measured at implementation:** the f123 workbook — two cells in a
200,001 × 101 box — is now accepted by `hardened()` and peaks at **134,591
bytes** against a 64 MiB budget, where the dense read it replaced measured
~646 MB.

### 3.6 `AlignmentMode::HeaderColumn` — the name describes something else (new)

Found during this sweep. Its doc says *"Match rows using the first row as a
column-header identity."* `header_column_alignment` (`align.rs:187`) delegates
to `row_key_alignment` with `columns = [1]` and never reads a header:

```rust
let key_col: Vec<u32> = vec![1]; // row-1 = header row; match data by that col
row_key_alignment(old_cells, new_cells, &key_col, diagnostics)
```

So `HeaderColumn` is exactly `RowKey { columns: vec![1] }` under a name that
promises header inference. Two public modes, one behaviour, and the second's
name and documentation describe a third thing. Same family as §3.1.

**Decided 2026-09-25: remove it.** Three defects stack, and the third is the
one that settles it:

1. It is **exactly `RowKey { columns: vec![1] }`**. Renaming it would leave two
   public names for one behaviour — the `number_compare` defect §3.7 removes.
2. Implementing it is a **feature**, which §4 forbids in this milestone.
3. **The thing its name promises does not exist anywhere in the crate.**
   RFC-011 §3 lists *"column alignment based on header names or column
   signatures"* as a goal; there is no column alignment at all — `RowMapping` is
   the only mapping type. And **RFC-011's Status says "Implemented
   (2.0.0–2.2.3) — verified 2026-08-16"**, the same class of false record as
   RFC-009 §6, which was the root cause of unit 01.

A caller who wants the behaviour writes `RowKey { columns: vec![1] }`.
RFC-011's Status and §3 goal are annotated the way RFC-009's were.

### 3.7 The builder's surface, settled once (M8 unit 07 + unit 08, folded in 2026-09-25)

**Additions and removals here are one decision, not two.** M8 unit 08 was
scheduled to *add* two setters in a 2.x minor while this section *removed* two;
that would have settled the builder's naming rule twice. Unit 08 is withdrawn
into this section.

**Remove** (each a break):

- `number_compare` — identical to `number_compare_policy`, same argument type,
  same field. The implementer's view, recorded: keep `number_compare_policy`,
  which matches its siblings `numeric_type_policy` and `type_mismatch_policy`;
  `number_compare` matches `formula_compare` / `format_compare`. **Settle the
  rule for the whole builder, then apply it**, rather than choosing per method.
- `build_with_matching` — it assigns the whole `MatchingOptions`, so it silently
  discards a `sheet_matching` set earlier in the chain. With `.alignment()` and
  `.sheet_matching()` (M8 unit 07) it has no remaining use a chain does not
  cover. Demonstrated, not asserted: `.sheet_matching(ExactNameOnly)` followed by
  `build_with_matching(..)` ends with the default.

**Add**, so the builder covers every leaf option and `DiffOptions`'s doc comment
needs no "except":

- `date_compare(DateComparePolicy)` — `comparison.value.date` is reachable
  through no builder method at all. Found by the implementer's audit; the
  architect's list had named the wrong two options.
- `max_cells_read(Option<u64>)` — its five `Limits` siblings each have a setter
  and it does not.

**Also**: remove `#[allow(dead_code)]` from `AlignmentMode` and its
`HeaderColumn` variant. Verified redundant — rustc never reports a `pub` variant
of a public enum as dead, and clippy stays clean without them on the pre-unit-07
source as well. Two lines, no risk, and it interacts with §3.6.

### 3.8 Values that never arrive, and a count that is always zero (added 2026-09-25)

Found by the implementer while working §3.2, outside its scope, and reported
rather than absorbed. Each appears only in its declaration, a `Display` arm, and
an exit-code or renderer arm; **none is constructed anywhere**.

**`Severity::Error` is structurally impossible, not merely unused.** RFC-005's
model is two-tier: fatal conditions are `SheetsDiffError`, recoverable ones are
`Diagnostic`. A diagnostic that is an "error" has no room in that design — the
closest real case, `DuplicateAlignmentKey` ("your rows may be paired wrongly"),
is correctly a `Warning`. All eleven diagnostic sites emit `Info` or `Warning`.

Its cost is not abstract: **`render_summary` prints
`diagnostics: N error(s), M warning(s)` on every run and `N` can only ever be
0.** Removing it takes `DiagnosticSummary::errors` and `render_unified`'s
unreachable `"ERROR"` prefix with it — the point, not a side effect.

| Value | Disposition |
|---|---|
| `Severity::Error` | **Remove**, with `DiagnosticSummary::errors` and the `N error(s)` output |
| `SheetsDiffError::UnsupportedFormat` | **Remove** — duplicates `OpenWorkbook { kind: NotXlsx }`, which is what a non-`.xlsx` file actually produces |
| `SheetsDiffError::Internal` | **Remove** — the escape hatch for our own bugs, never needed. `SheetsDiffError` is `#[non_exhaustive]`, so re-adding is a minor |
| `OpenErrorKind::Locked` | **Remove** — see below |

**`Locked` is not simply dead.** A locked file (Excel holding it open) is a real
condition; `error.rs:259` maps io errors to `NotFound`, `PermissionDenied` or
`Other`, and nothing produces `Locked`, so we report a locked file as
"permission denied" — nearly true. Detecting it properly needs raw per-platform
OS error codes, which is a **feature** and §4 forbids it here. A near-true
classification beats a variant that never arrives; re-adding is additive.

### 3.9 The options tree cannot be extended without a break (added 2026-09-25)

**The largest item in this RFC, and it was found by checking whether §3.3 was
reversible.**

Every one of the eight public **model** structs is `#[non_exhaustive]`:
`WorkbookDiff`, `SheetDiff`, `CellDiff`, `DiffSummary`, `DiffMetrics`,
`Diagnostic`, `DiagnosticLocation`, `SheetRef`.

**None of the eight public options structs is:** `DiffOptions`,
`ComparisonOptions`, `ValueCompareOptions`, `MatchingOptions`, `Limits`,
`ExecutionOptions`, `DiagnosticOptions`, `OutputOptions`. Every field is `pub`,
so a caller may write `ComparisonOptions { value, formula, … }`.

**Therefore every option this crate ever adds is a breaking change, while every
result field it adds is additive.** That asymmetry is not a decision anyone
made; the results got it right and the options were overlooked. It is invisible
in the API surface, it can only be fixed at a major, and left alone it taxes
every future release — RFC-011's column alignment, RFC-022's formatting, a
formula normaliser: each one adds an option.

**Add `#[non_exhaustive]` to all eight.** Two consequences, both to be stated:

- Callers constructing options by struct literal move to
  `DiffOptions::builder()` or `..Default::default()`. The builder covers all
  leaf options (§3.7), so the recommended path is complete.
- **`tests/builder_coverage.rs` destructures these exhaustively and is a
  separate crate**, so `#[non_exhaustive]` breaks its `E0027` guard — the best
  structural test this milestone produced. **Move it into `src/` as a unit
  test**, where `#[non_exhaustive]` does not apply. The guard survives and gets
  stronger for sitting beside what it guards.

## 4. Explicitly **not** in scope

Stating these closes the list, which is the point of writing it down.

- **Reserved placeholders stay**: `ExecutionMode`, `CellFormat`, `FormatChange`,
  `WorkbookChange`, `ObjectChange`, and the `Reserved until RFC-0xx` fields.
  They are honest and they exist to prevent the next break.
- **`Deserialize`** stays declined (RFC-014, ForskScope asked directly).
- **No new features.** Not style comparison, not a formula normaliser, not
  parallel execution. v3 is a correction release, not a capability release.
- **MSRV stays 1.88** unless something here forces otherwise.
- **No module reorganisation.** `view.rs`'s status (M9 O5) is a documentation
  decision, not a layout change.
- **Nothing found after this RFC is accepted joins it** without the owner
  reopening the list. That is what "closed" means, and it is the mechanism that
  stops a major becoming a six-month project. **It was reopened once, on
  2026-09-25, for §3.8 and §3.9 and the §3.3/§3.6 decisions — deliberately in a
  single amendment — and closed again.**
- **`hardened()`'s `max_cells_read` value (5,000,000)** is *not* in scope. It
  was chosen when the field meant bounding-box area and now bounds retained
  cells, so it should be re-derived from a stated memory budget — but changing
  a value is not a breaking change, so it is a task for any release, not a v3
  item. It must be **measured, not estimated**: the standing rule since M7.

## 5. Sequence

1. **2.6.0 ships first**, carrying M8 units 00–03 (done), 06 and 07. It is the
   release ForskScope adopts. It must not wait for v3.
2. **M8 units 04, 05 and 08 are withdrawn** into this RFC — 04 becomes
   §3.1/§3.2, 05 becomes §3.5, 08 becomes part of §3.7. **M8 closed at unit 07,
   published as 2.6.0 on 2026-09-25.**
3. **M9's one observable unit (O-A) moves here** — populating
   `DiagnosticLocation` at the push sites changes `location.sheet_name` in
   serialised output, which `--format json` now publishes. M9's other six units
   are invisible and need no release.
4. **3.0.0 is the next release.** There is no planned 2.7.0; see `ROADMAP.md`.
5. **v3 handoffs are written against the 2.6.0 tree**, not this one.
6. **A `v2-to-v3` migration guide** is required, matching
   `docs/src/migration/v1-to-v2.md` in form, and every removal here needs a row
   in it saying what to use instead.

## 6. Acceptance criteria for this RFC

1. Every item in §3 has a decision recorded before a handoff is written.
2. The migration guide covers every removal.
3. No item is added to §3 after acceptance without the owner reopening it.
4. ~~ForskScope is told before 3.0.0 is cut, not at it.~~ **Superseded by the
   owner, 2026-09-25:** the letter goes **after** the final stable release, not
   before the cut. One letter covering 2.6.0 and 3.0.0 together, rather than a
   notice per release — their adoption target has already moved three times and
   a fourth pre-announcement buys them nothing they can act on. The draft is
   held at `.git-exclude/tmp/sheets-diff-to-forskscope-v3-notice.md` and must be
   rewritten in the past tense before it goes.

## 7. Open questions for the owner

**These do not block acceptance; each blocks the handoff that depends on it.**

1. **Does 2.x get anything after 3.0.0?** Security-only, or nothing? ForskScope
   will likely be on 2.6.x. Cheaper to decide now than at the cut — and now
   more so, since the letter arrives *after* 3.0.0 and will have to state the
   answer rather than ask it.
2. **§3.3** — does `FormatCompareMode` survive as a single-variant reservation?
3. **§3.6** — implement, rename, or remove `HeaderColumn`?
