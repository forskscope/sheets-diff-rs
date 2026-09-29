# Handoff f134-01 — A name collision hides a leftover sheet; compare identity, not name

**Defect response. In the same release as f132 and f133.** **Written:** 2026-09-29.
**Found by:** you, in f133-01 §5, while testing the `ExactNameThenIndex` interaction.
**Depends on:** f133 (committed). **Blocks the release.**

## Purpose

Under `SheetMatchingMode::ExactNameThenIndex`, an unmatched old sheet whose *name* collides with an
already-matched pair's name is filtered out of `still_unmatched_old` and never reaches
`push_removed`. It does not become `Removed`. It does not become anything: `WorkbookDiff::sheets` is
simply one entry short, with no diagnostic. **A sheet disappears.**

## Background

**Verified at review, all three modes, on a worktree of the pre-f133 commit and on the f133 tree.**
Input: old `[Sheet@0, Sheet@1]`, new `[Sheet@0]`.

| Mode | pre-f133 | post-f133 |
|---|---|---|
| `ExactNameOnly` | 2: `Unchanged`, **`Moved`** (bogus) | 2: `Unchanged`, **`Removed`** ✓ |
| `ExactNameThenConservativeRename` (default) | 2: `Unchanged`, **`Moved`** (bogus) | 2: `Unchanged`, **`Removed`** ✓ |
| `ExactNameThenIndex` | 2: `Unchanged`, **`Moved`** (bogus) | **1: `Unchanged`** |

**f133 is correct and is not being revisited.** It fixed the default and `ExactNameOnly`. In
`ExactNameThenIndex` it replaced one wrong answer with a differently-shaped wrong answer, because it
correctly stopped the duplicate claim and so let the leftover reach a *second*, pre-existing flaw
that nothing had exercised before.

**The flaw**, both filters in the `ExactNameThenIndex` arm of `match_sheets`:

```rust
!pairs.iter().any(|p| {
    p.old_sheet.as_ref().map(|r| r.name == s.name).unwrap_or(false)
        && !matches!(p.change, SheetChange::Removed)
})
```

It asks *"is a sheet with this name already paired?"* where it means *"is **this** sheet already
paired?"*. `still_unmatched_new` has the same shape against `new_sheet` / `Added`.

**Why a sheet vanishing is worse than the bug f133 fixed**, and why this is not deferrable: a caller
iterating `WorkbookDiff::sheets` gets no signal at all. No `Removed`, no diagnostic, no count to
reconcile against. It is the strongest form of the silent-wrong-answer class this project treats as
worse than a visible failure, and it would otherwise ship in the release whose headline is that we
fixed a silent wrong answer and a denial of service.

## Change scope

- `src/matcher.rs` — the two `still_unmatched_*` filters, and the characterisation test.
- `CHANGELOG.md`, `docs/src/maintainers/threat-model.md` — this moves from open to fixed.
- `tests/` — a regression test at the public-API level if the shape is reachable there.

## Non-change scope

- **Do not revisit f133.** It is correct and committed.
- **Do not change `index_match`.** It already tracks `used_new`, and its `n.index == old.index`
  match cannot double-claim because indices are unique per side. Read it to confirm; change nothing.
- No public API change. No new diagnostic — see f133-01's review §5 for why, and note that after
  this fix the answer is *correct*, not merely non-wrong, so there is nothing to explain to a caller.
- Do not touch `conservative_rename`.

## Required implementation

**1. Compare identity, not name, in both filters.** `SheetRef::index` is the sheet's position within
its own side and is unique there, so `r.index == s.index` is the identity test — for
`still_unmatched_old` against `p.old_sheet`, and `still_unmatched_new` against `p.new_sheet`.

**Verified at review:** that change alone yields the correct `Unchanged` + `Removed` for the shape
above, and the only test that then fails is the characterisation test below. **Confirm it yourself
before relying on it** — I applied it in a throwaway build to size the unit, not to hand you a patch,
and I did not run the full matrix on it (the lib suite failing stopped the run).

**2. Satisfy yourself that `index` is the right identity, and say so in the review request.** It is
the field that distinguishes two sheets with the same name, which is the whole case. If you find a
path where two `SheetRef`s on the same side share an index, **stop and report it** — that would be a
worse defect than this one and would change the fix.

**3. Delete the characterisation test** `known_defect_an_unrelated_matched_pairs_name_hides_a_leftover_sheet`
and replace it with the positive assertion: `ExactNameThenIndex` on that input gives one `Unchanged`
and one `Removed`, matching what the other two modes already give. **The other two modes' results
are the specification here** — the bug was that one mode disagreed with them.

**4. Check the same name-for-identity confusion nowhere else.** f133-01 §5 says these two filters
were the only other place found. Confirm across `src/matcher.rs`, and say where you looked. A
`.name ==` used as an identity test is the pattern; a `.name ==` used to *match by name* is correct
and expected.

## Required tests

- **The positive assertion** from *Required implementation* 3.
- **Mode agreement, stated as such**: the same input through all three `SheetMatchingMode`s produces
  the same sheet *set* (one matched, one removed) — differing, if at all, only in `change`
  classification. This is the invariant the defect broke and it is worth pinning directly.
- **The `still_unmatched_new` side too.** The symmetric case (one old `X`, two new `X`,
  `ExactNameThenIndex`) must yield one match and one `Added`, never a vanished new sheet. **The
  report did not demonstrate this side; do not assume it behaves like the old side — test it.**
- **Failing first**, by reverting each filter's comparison one at a time (two demonstrations, not
  one), each `cmp`-verified.
- **Nothing else moves**: 82 goldens byte-identical, corpus clean, the f133 tests green untouched,
  `cargo public-api --simplified diff 3.1.0` unchanged.

## Acceptance criteria

1. `ExactNameThenIndex` on two old `X` / one new `X` gives one `Unchanged` and one `Removed`.
2. The symmetric new-side case gives one match and one `Added`.
3. All three modes agree on the sheet set for both shapes.
4. The characterisation test is gone, replaced by positive assertions.
5. Both filters demonstrated failing-first, independently.
6. No other name-as-identity comparison in `src/matcher.rs`, or it is reported.
7. 82 goldens byte-identical; public API unchanged; gates green including rule 003.
8. CHANGELOG and threat model move this from open to fixed.

## Prohibited shortcuts

- **Do not fix it by suppressing the filter** (dropping the `!pairs.iter().any(..)` guard entirely).
  The guard exists to stop a sheet being reported twice; removing it trades a vanished sheet for a
  duplicated one. Fix what it compares, not whether it compares.
- Do not make the fix conditional on duplicate names being present. The comparison is wrong in
  general; it is merely unobservable when names are unique.
- Do not add a diagnostic to paper over a case you are unsure about. If you are unsure, say so.

## Known risks

**1. `index` may not mean what I think.** I have read it as the sheet's position within its own
workbook, unique per side — that is what makes it an identity. If `SheetRef::index` is ever a
*global* or cross-side value, the fix is wrong in a way that tests on one shape might not reveal.
**Check the type's definition and its construction sites, not just its name.**

**2. The goldens could move.** No corpus fixture should have duplicate sheet names, so they should
not. **If one moves, stop and report it** — it would mean a corpus workbook has a shape we did not
know it had, and that is a finding about the corpus.

**3. This is the fourth defect in this area in two units.** Two were found by `fuzz_self_comparison`,
one by testing its neighbourhood. If while fixing this you find a fifth, **report it and stop** — at
that point the right answer is a design review of `match_sheets` as a whole, which is mine to run,
not another one-line fix.

## Required evidence

Under `.git-exclude/review-request/f134-01-compare-identity-not-name/evidence/`:

1. All three modes × both shapes (two old / one new, and one old / two new), before and after.
2. Both failing-first demonstrations, with `cmp`-verified restores.
3. `SheetRef::index`'s definition and construction sites, supporting risk 1.
4. Where you looked for other name-as-identity comparisons.
5. Goldens and corpus: 0 differ. Public API diff.
6. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/f134-01-compare-identity-not-name/README.md`, with:

- Your evidence that `index` is a per-side identity.
- What the new-side symmetric case did **before** your fix — I want to know whether it was already
  broken or whether only the old side was.
- Anything else in `match_sheets` that compares a name where it means a sheet.
- Whether you think `match_sheets` now warrants a design review rather than further point fixes.
  You have been inside it more than anyone; say so if it does.
