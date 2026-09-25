# Handoff 06 — Options a caller cannot usefully set

**Governing.** [RFC-037 §3.3 and §3.6](../../accepted/037-v3-scope.md);
RFC-011 (row/column alignment); RFC-022 (styles and formatting)
**Roadmap.** M10 — unit 06
**Sequence.** Parallel with 07. **Before 08**, which must see the final leaf set.

## Purpose

Four comparison settings return `InvalidOptions` if selected, one whole option
can only hold its default, and one alignment mode is a second name for another
mode while promising something the crate does not have. Remove all of it.

## Background

### §3.3 — settings that can only fail

`DiffOptions::validate()` rejects these unconditionally:

- `FormulaCompareMode::NormalizedText` — its doc promises *"a normaliser
  feature"*. **There is no such feature and none is planned.**
- `FormulaCompareMode::RawAndNormalized`
- `FormatCompareMode::NumberFormatOnly`
- `FormatCompareMode::AllAvailable`

**And the field is worse than the variants.** `comparison.format` is read in
exactly one place — `validate()`, to reject everything but `Ignore`. It is a
public option whose only usable setting is the one a caller gets by not setting
it. Keeping `FormatCompareMode { Ignore }` as a single-variant reservation would
preserve exactly that, so both go.

`FormulaCompareMode` keeps `RawText` and `Ignore`, which do work.

### §3.6 — `AlignmentMode::HeaderColumn`

`header_column_alignment` (`src/align.rs`) delegates to `row_key_alignment` with
`columns = [1]` and never reads a header. **It is exactly
`RowKey { columns: vec![1] }`.**

Its name promises column identity from header names. **There is no column
alignment anywhere in this crate** — `RowMapping` is the only mapping type.
RFC-011 §3 lists *"column alignment based on header names or column
signatures"* as a goal, and **RFC-011's Status says "Implemented (2.0.0–2.2.3) —
verified 2026-08-16."** That is the same class of false record as RFC-009 §6,
which was the root cause of unit 01.

## Change scope

- `src/options.rs` — the two enums, the `format` field, `validate()`
- `src/align.rs` — `header_column_alignment` and its dispatch arm
- `src/compare.rs` or wherever `comparison.format` is threaded, if anywhere
- `tests/`
- `rfcs/done/011-*.md`, `rfcs/accepted/022-*.md`, `rfcs/done/033-*.md`
- `docs/`
- `CHANGELOG.md`

## Non-change scope

- **Do not implement column alignment, a formula normaliser, or style
  comparison.** RFC-037 §4 forbids features here. All three are why the removed
  names existed.
- Do not touch `RowKey`, `RowSignature` or `Positional`.
- Do not change `validate()`'s other checks.
- Do not remove `FormulaCompareMode` itself — two of its variants work.

## Required implementation

1. **Remove `FormulaCompareMode::{NormalizedText, RawAndNormalized}`** and the
   `validate()` arm that rejected them, which becomes dead with them.
2. **Remove `FormatCompareMode` entirely and `ComparisonOptions::format`**, its
   builder setter `format_compare`, and the `validate()` arm.
3. **Remove `AlignmentMode::HeaderColumn`** and `header_column_alignment`.
4. **RFC-022 is annotated**: the enum and field it specified are removed until
   it is implemented, and reintroducing them is additive **because of §3.9**
   (unit 08). Say that explicitly — it is the reason removal was chosen over a
   single-variant reservation, and a future reader will otherwise re-litigate it.
5. **RFC-011's Status and §3 goal are annotated** the way RFC-009's were in unit
   01: the column-alignment goal was never implemented, `HeaderColumn` delegated
   to row alignment on column 1, and the Status claim of "Implemented" covers
   the row half only. **Do not change the Status word without saying why** —
   unit 01 established that a design sketch not built is not the same as an
   unmet acceptance criterion; check which this is before deciding.
6. **`validate()`'s doc and `Limits::hardened()`'s** — sweep for any sentence
   that names a removed value.

## Required tests

- A caller cannot name the removed variants: demonstrated by compile failure,
  said plainly as such.
- **`RowKey { columns: vec![1] }` produces the result `HeaderColumn` produced**
  on a fixture where alignment changes the outcome. This is the migration's
  proof and it must exist, not just be asserted in the CHANGELOG.
- `validate()` still rejects what it should and accepts what it should — the
  surviving arms, tested in both directions.
- The alignment suite still passes with `HeaderColumn` gone; **say how many
  tests used it and what they use now.**

## Acceptance criteria

1. Four settings, one enum, one field, one mode, and one builder setter removed.
2. `FormulaCompareMode` keeps `RawText` and `Ignore`, both working.
3. `validate()` has no arm that can never be reached.
4. **The `RowKey{[1]}` equivalence is demonstrated by a test**, not asserted.
5. RFC-011 and RFC-022 annotated; RFC-033's lexicon agrees.
6. No sentence anywhere still names a removed value — `grep` and show it.
7. Corpus byte-identical. **Goldens: state whether any scenario used
   `HeaderColumn`**; the corpus has alignment fixtures, so check.
8. CHANGELOG `### Removed`, with the migration for each: `HeaderColumn` →
   `RowKey { columns: vec![1] }`; the format and formula settings → nothing to
   migrate to, and say when they may return.
9. Gates green.

## Prohibited shortcuts

- **Do not keep `FormatCompareMode { Ignore }`.** §3.3 decided against it; a
  single-variant enum for a field that is read only to be rejected is the defect
  itself, preserved.
- Do not rename `HeaderColumn` to `RowKeyFirstColumn`. That leaves two names for
  one behaviour, which is the defect §3.7 removed from the builder.
- Do not make `comparison.format` private instead of removing it.
- Do not "temporarily" keep a deprecated alias. This is the major.

## Compatibility constraints

**Breaking, in three ways, and the CHANGELOG must separate them:**

1. Naming a removed variant fails to compile — the intended signal.
2. **Constructing `ComparisonOptions` by struct literal fails to compile**,
   because a field is gone. That break is unavoidable here and is being made
   permanent by unit 08, which is the point: after 3.0.0 the same change is
   additive.
3. `format_compare` disappears from the builder.

A caller using `HeaderColumn` has an exact replacement. A caller who set the
format or formula settings was receiving `Err(InvalidOptions)` and has nothing
to migrate to; say so rather than implying a replacement exists.

## Known risks

- **`docs/src/` executes.** The API guide and semantics page may name
  `format_compare` or `HeaderColumn`.
- **Check whether any corpus scenario or bench uses `HeaderColumn`.** Unit 03
  found two `build_with_matching` sites in `benches/` that a `tests/`-only
  search missed, and `--all-targets` compiles benches.
- RFC-022 is `accepted`, not `done`. Removing its surface does not withdraw the
  RFC; do not change its Status to something that implies it was abandoned.

## Required evidence

- Compile-failure transcripts for each removal
- The `RowKey{[1]}` equivalence test
- Your own search for surviving mentions, command shown, across `src/ tests/
  benches/ examples/ docs/ README.md`
- Corpus byte-comparison and the `HeaderColumn`-usage statement
- Gates, each with exit status
- CI run link

## Review request format

Per development policy §9.2. Additionally: state how many tests or benches used
`HeaderColumn` and what they use now, and say whether RFC-011's Status word
should change — with the unit-01 distinction applied, not assumed.
