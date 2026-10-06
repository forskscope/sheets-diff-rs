# Handoff 02 — Say why the two texts differ

**Unit:** formulas-move-with-their-rows 02. **Added 2026-10-06.** **Scoped by:** the architect.
**Semver: minor** — `FormulaChange` is `#[non_exhaustive]`, so the field is additive.
**Depends on unit 01**, which must be reviewed and approved first.
**Rule 005: propose the enum before implementing.**

## Purpose

`FormulaChange` gains a field saying **why** its two texts differ: because the formula was edited, or
because its references moved with the row. **The reported set of `cell_diffs` does not change.**

## The shape, to propose and not to assume

My intent, for you to argue with:

```rust
/// Why a [`FormulaChange`]'s two texts differ.
#[non_exhaustive]
pub enum FormulaDifference {
    /// Mapping the old formula's row references through this sheet's alignment makes the two texts
    /// equal. The formula was not edited; its references moved with the row.
    ExplainedByRowMapping,
    /// Mapping was attempted and does not make the texts equal. The formula differs for some other
    /// reason, which may include an edit.
    NotExplainedByRowMapping,
    /// No mapping was attempted: this sheet was compared positionally, or this cell's placement is
    /// not `PairedByAlignment`, so no row moved and there is nothing to explain.
    NoRowMovement,
    /// Mapping was attempted and declined: part of the formula was not understood, or a reference
    /// pointed at an unpaired row. **Nothing is claimed either way.**
    NotDetermined,
}
```

Four decisions in that, each of which you may dispute:

- **Four variants, not a `bool`.** A bool collapses `NotDetermined` into
  `NotExplainedByRowMapping` — a marker credited with more than it measured, which is the thing this
  project has spent a quarter deleting. The distinction between *"we checked and it is a real
  change"* and *"we could not tell"* is the one a consumer must have.
- **`NoRowMovement` separate from `NotDetermined`.** Both mean "no claim", but for opposite reasons:
  one because there was nothing to check, one because we failed to check. A consumer counting how
  often we decline needs them apart, and so do we.
- **Always present, not `Option`.** An `Option<FormulaDifference>` would give two ways to say
  nothing. `NoRowMovement` is the "not applicable" value.
- **`#[non_exhaustive]`, unlike `RowPlacement`.** The domain is not closed — stage 2 plausibly adds
  outcomes (a reference that maps to `#REF!`, a cross-sheet mapping that was unavailable). RFC-031 §6's
  "likely to grow" criterion, applied the same way `ConfidenceReason` was.

Name it however reads best; `FormulaDifference` is my suggestion, and the variant names matter more
than the type's.

## Change scope

- `src/model.rs` — the enum and the field on `FormulaChange`.
- `src/lib.rs` — the crate-root re-export, consistent with the other model types.
- `src/compare.rs` / `src/diff.rs` — `compare_formulas` needs the mapping and the placement to decide
  the value. **Propose the plumbing**: it currently takes three arguments and none of them is the
  mapping.
- `tests/integration.rs`, the goldens, `docs/src/semantics.md`, `docs/src/migration/`, `CHANGELOG.md`.
- `src/options.rs` — `AlignmentMode`'s documentation. See *Required implementation* 4.

## Non-change scope

- **Do not change which `cell_diffs` are reported, or any value, or any other field.** See the
  acceptance criteria: this is pinned by a test, not by intention.
- Do not suppress a formula change, under any circumstance, for any variant.
- Do not change unit 01's refusal behaviour. If you find a case it should refuse, that is a finding
  against unit 01, reported, not fixed here.
- Do not touch `FormulaText::normalized`. It stays `None`; this is not the normaliser.

## Required implementation

**1. Propose the enum and the plumbing** (rule 005,
`.git-exclude/proposal/formula-difference-02/README.md`).

**2. Compute it at the one construction site**, from the placement and the mapping, both already in
scope at `src/diff.rs:580` where `compare_formulas` is called. `PairedByAlignment` is the only
placement for which mapping is attempted.

**3. The release note is the hard part of this unit, and it is consumer-facing.** It must say:
- that **nothing is suppressed** and the set of reported changes is unchanged;
- that a consumer who wants the formula cascade gone filters on `ExplainedByRowMapping`;
- that `NotDetermined` means we declined, **not** that the formula is unchanged, and that stage 1
  declines any formula containing a string literal, a sheet qualifier, a structured reference, or a
  reference to an unpaired row — **name the declined classes**, because a consumer filtering on
  `ExplainedByRowMapping` will see those as changes and must know why;
- the JSON gains a key on every formula change.

**4. Correct `AlignmentMode`'s documentation**, which is where this defect is visible as a false
claim: **`RowKey`** says it *"reduces cascades after row insertion/deletion"* (`src/options.rs:166`;
corrected 2026-10-07 from this handoff's original misattribution to `RowSignature`), and on a sheet with a
formula column over moved rows it does not — it exchanges a value cascade for a formula one. Say that
plainly, and say what the annotation now lets a consumer do about it. **This paragraph is owed
whatever happens to the rest of the unit**; it was true before the annotation existed.

## Required tests

1. **The 200-row measurement, as a test.** 200 rows keyed on column A, one row inserted at row 3,
   column D carrying `=C{r}*2`. Before: 199 formula changes, none annotated. After: the same 199
   reported, every one `ExplainedByRowMapping`. Assert **both** halves — the count and the
   annotation — because the count is what proves nothing was suppressed.
2. **A genuine edit is not explained.** `=C5*2` → `=C5*3` across a shift is
   `NotExplainedByRowMapping`.
3. **A genuine edit and a shift together** — `=C5*2` at old row 5 becoming `=C6*3` at new row 6 — is
   `NotExplainedByRowMapping`. This is the one a careless implementation gets wrong by mapping and
   then comparing loosely.
4. **Positional gives `NoRowMovement`** for every formula change.
5. **Each declined class gives `NotDetermined`**: a string literal, a sheet qualifier, a structured
   reference, a reference to an unpaired row. One test each, named for the class.
6. **`NotDetermined` is not `NotExplainedByRowMapping`** — assert they are distinguishable on two
   sheets that differ only in whether the formula is understood.
7. **The safety test: the feature changes no `cell_diffs`.** For every corpus fixture and for the
   reproduction, the sequence of `(address, change_kind)` is identical to 3.5.0's. **If this fails,
   stop — something is being suppressed.**
8. **Failing-first** for 1, 3 and 7.

## Acceptance criteria

1. The enum and plumbing proposed and agreed before implementation.
2. All four variants reachable, each by a named test.
3. Test 7 passing: **no reported change moved, anywhere.**
4. The release note covering all four points of *Required implementation* 3, including the declined
   classes by name.
5. `AlignmentMode`'s documentation corrected.
6. Goldens blessed and **read**: expect every formula change to gain a key, and expect the corpus's
   default-options path to produce `NoRowMovement` throughout, since the golden path is `Positional`.
   **A golden showing anything else is a finding.**
7. `cargo public-api`: additions only.
8. Gates green, rule 003, one scratch dir, deleted. Nothing committed.

## Prohibited shortcuts

- **Never suppress.** Not for `ExplainedByRowMapping`, not behind an option, not "for convenience".
- Do not fold `NotDetermined` into any other variant.
- Do not add an option to turn this on or off. It is information, always present, and
  `m10-the-names-become-true/06` removed the last options a caller could only fail with.
- Do not claim a class works that unit 01 refuses.

## Known risks

**1. The annotation is a claim to a consumer, and they will filter on it.** ForskScope's renderer will
hide `ExplainedByRowMapping`. So a wrong `ExplainedByRowMapping` hides a real formula change **in
their product**. That is why unit 01 refuses first and why test 3 exists.

**2. `NotDetermined` will be common in stage 1** — any formula with a string literal or a sheet
qualifier. That is expected and must be stated in the release note, or a consumer will read our
declining as our asserting.

**3. The goldens will move for every formula-bearing fixture.** Read the diff; do not bless it.

## Required evidence

Under `.git-exclude/review-request/formula-difference-02/evidence/`:

1. The proposal and my reply.
2. The 200-row test's before and after numbers.
3. Test 7's comparison against 3.5.0, in full.
4. One test per declined class.
5. The golden diff, with the `NoRowMovement`-throughout result confirmed.
6. `cargo public-api`, detached worktree. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/formula-difference-02/README.md`, with:

- The enum as shipped, and the release-note text quoted in full — I review those words, because they
  are what a consumer acts on.
- Test 7's result, stated first.
- The declined classes as a list, exactly as the release note names them.
- Whether `AlignmentMode`'s corrected paragraph says enough for a consumer to decide what to do.
