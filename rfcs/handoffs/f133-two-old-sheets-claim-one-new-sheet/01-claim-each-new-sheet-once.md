# Handoff f133-01 — Two old sheets claim the same new sheet, and a workbook differs from itself

**Defect response. In the same release as f132** (the release waits for both; owner, 2026-09-29).
**Written:** 2026-09-29. **Found by:** `fuzz_self_comparison`, validating f132-01 — the second
defect that target has found, and **the first one in our own code.**

## Purpose

`match_sheets`'s exact-name phase pairs an old sheet with the first new sheet of the same name
**without checking whether that new sheet is already claimed.** Two old sheets sharing a name both
pair to the same new sheet; the genuine second new sheet is never considered and is reported as
`Added`, with its contents as spurious cell changes.

**This is a silent wrong answer, not a crash** — the category this project treats as the worse one —
and it is ours, not a dependency's.

## Background

**Reproduced independently at review, deterministically, five runs identical:**

```
compare_bytes(w, w)   // the SAME bytes on both sides
sheets in result: 3
  name="" change=Unchanged cell_diffs=0
  name="" change=Moved     cell_diffs=0     <- the SAME new sheet as the line above
  name="" change=Added     cell_diffs=3     <- the real second new sheet, reported as new
self-comparison reports 3 changed cell(s)   (must be 0)
```

A workbook compared with itself reports three changed cells. The oracle
`fuzz_self_comparison` exists to assert — zero cell diffs on self-comparison — is violated.

**The mechanism**, `src/matcher.rs:43`:

```rust
if let Some(ni) = new_sheets.iter().position(|n| n.name == old.name) {
```

`matched_new_indices[ni] = true` is set in the body but never consulted by the `position` call, so
`position` returns the same `ni` for every old sheet with that name. Note both symptoms: one new
sheet appears in **two** `MatchedPair`s (which nothing downstream expects), and one new sheet appears
in none.

**Verified at review that the obvious fix is the right one and is small.** Skipping claimed
indices —

```rust
.enumerate().find(|(i, n)| !matched_new_indices[*i] && n.name == old.name).map(|(i, _)| i)
```

— gives `2 pairs, both Unchanged, 0 cell diffs` on the same input, and
`tests/sheet_match_classification.rs` (6) and `tests/sheet_match_reason.rs` (4) stay green. **Treat
that as evidence the diagnosis is right, not as the patch**: it is unreviewed, I wrote it to test the
mechanism, and the naming, the comment and the surrounding cleanup are yours.

**How reachable is it?** The trigger is two sheets with the same name on one side. Excel's UI forbids
duplicate sheet names, so a workbook Excel wrote will not have them; the fuzzer reached it by
corrupting bytes inside a valid archive's compressed stream, which blanked a sheet name to `""` on
more than one sheet. **Do not let that make this a curiosity.** A file we did not write is exactly
the input this crate is documented to accept (`docs/src/maintainers/threat-model.md`: files are
untrusted input), an empty name is not exotic, and the failure is a *wrong diff*, silently.

## Change scope

- `src/matcher.rs` — the exact-name phase.
- `tests/` — regression tests.
- `CHANGELOG.md` — `### Fixed`, and the *Security* bullet that currently lists this as open.
- `docs/src/maintainers/threat-model.md` — the assurance row that points at this defect.
- `fuzz/corpus/fuzz_self_comparison/` — a seed for the shape, if one is not already reachable.

## Non-change scope

- **The conservative-rename and index phases are not in scope.** Read them to be sure the same
  first-match-without-claiming pattern is not repeated there — and **if it is, report it, do not fix
  it here**; that is a second defect and wants its own measurement.
- No public API change. No new `SheetChange` variant, no new diagnostic kind, unless you find that
  duplicate names *must* be reported to a caller — in which case **stop and say so**; that is a
  design question and mine to answer.
- Do not change how sheets are read or named. An empty sheet name is the symptom that exposed this,
  not the defect.

## Required implementation

**1. A new sheet may be claimed once.** The exact-name phase must not pair an old sheet with a new
sheet another old sheet already holds.

**2. Decide, and write down, what happens to the *second* old sheet of a duplicated name.** With the
one-line fix it pairs with the second new sheet of that name, which is right when both sides carry
the same duplicates. Say what happens when they do not — two old sheets named `X`, one new sheet
named `X` — and make sure the answer is deliberate: the leftover old sheet should reach the later
phases and end up `Removed`, not silently dropped and not paired with something unrelated. **Test
the asymmetric case explicitly; it is the one the fixture does not cover.**

**3. Consider whether a duplicate name deserves a diagnostic.** My view: probably yes, at `Warning`,
because a caller seeing two sheets with one name has a file that Excel could not have written and
should know. But `DiagnosticKind` is public and `#[non_exhaustive]`, so a new variant is a minor-level
API addition. **Measure the case first, propose, and let me rule** — do not add it silently, and do
not skip it silently either.

## Required tests

- **The self-comparison invariant, directly:** the fixture at
  `.git-exclude/review-request/f132-01-decline-before-delegating/evidence/matcher-bug/mutated-chart-sheet-workbook.bin`
  compared with itself yields **0 cell diffs** and no `Added`/`Removed` sheet. Commit it as a
  fixture; it is small and it is the exact case.
- **Constructed, not only corrupted:** build the duplicate-name case directly at the `match_sheets`
  level — two old sheets named `X`, two new named `X`; and the asymmetric two-old/one-new case — so
  the regression does not depend on a corrupted binary staying corrupt in the same way.
- **No new sheet appears in two pairs**, asserted over the returned `Vec<MatchedPair>`. That is the
  invariant underneath this defect and it is worth stating on its own.
- **Failing first**, by removing the claim check (one expression, not a file revert), with a
  `cmp`-verified restore.
- **Nothing else moves:** 80 goldens byte-identical, the corpus clean,
  `tests/sheet_match_classification.rs` and `tests/sheet_match_reason.rs` green untouched.

## Acceptance criteria

1. The fixture compared with itself reports **0 cell diffs**; failing-first demonstrated.
2. No new sheet appears in more than one `MatchedPair`, asserted.
3. The asymmetric duplicate case behaves deliberately and is tested.
4. 80 goldens byte-identical; `cargo public-api --simplified diff 3.1.0` shows nothing new.
5. A decision on the diagnostic, proposed rather than taken (see *Required implementation* 3).
6. A report on whether the later matching phases repeat the pattern — **reported, not fixed**.
7. CHANGELOG and threat model move this from open to fixed. RFC-009 is the sheet-matching RFC (not
   RFC-006 — I have named the wrong one before); check whether its §6 or §9 describes the exact-name
   phase in a way this contradicts, and annotate if so.
8. Gates as always, plus rule 003.

## Prohibited shortcuts

- **Do not fix this by making sheet names unique on read.** Renaming a caller's sheets to make our
  matcher work would be a silent change to reported data — a worse defect than the one being fixed.
- Do not make the fix conditional on the name being empty. `""` is how the fuzzer reached it; the
  defect is duplication, not emptiness.
- Do not skip the asymmetric case because the fixture does not produce it.
- Do not reuse my one-line probe without reading it. It was written to prove a mechanism in a
  throwaway build.

## Known risks

**1. The goldens may legitimately move.** If any corpus fixture has duplicate sheet names, its
expected output changes — and that would mean this defect is in our own corpus's expected results.
**If a golden moves, stop and report it before regenerating.** A moved golden here is a finding.

**2. Pairing the second old sheet with the second new sheet assumes order carries meaning.** For
duplicate names it is the only signal available, and it is what the index-based fallback already
assumes. Say in the review request that you considered it; do not leave it implicit.

**3. This does not close `fuzz_self_comparison`'s path into CI.** The overflow panic from M9 unit 01
is still open and still reachable at CI's budget, so the target stays out of the smoke matrix and
`paired_encrypted` stays quarantined after this unit. Do not delete
`fuzz/corpus-quarantine/` here — f132-01's review changed that directory's stated reason, and it is
now waiting on the overflow defect, not on this one.

## Required evidence

Under `.git-exclude/review-request/f133-01-claim-each-new-sheet-once/evidence/`:

1. The fixture self-compared, before and after, with the per-sheet breakdown.
2. The failing-first run and the `cmp` restore.
3. The constructed symmetric and asymmetric duplicate-name cases.
4. Goldens and corpus: 0 differ, or the finding if not.
5. Whether the later phases repeat the pattern, with the code read and quoted.
6. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/f133-01-claim-each-new-sheet-once/README.md`, with:

- What happens in the asymmetric case and why that is the right answer.
- Your proposal on the diagnostic, with the case for and against.
- Whether the later matching phases share the defect.
- Anything else `fuzz_self_comparison` found while you were in here. It has now found two defects in
  two runs; assume it will find a third.
