# Handoff 01 — Pin the cached-value promise, and survey what else is unpinned

**Written:** 2026-10-01. **Origin:** ForskScope's letter of the same date, §1.
**Small.** One test, and a survey reported rather than fixed.

## Purpose

We told a consumer, in writing, that `include_formula_cached_values` will **not** be wired to its
old documented meaning — a formula cell's cached value stays compared, unconditionally. Nothing in
this repository would fail if someone did it anyway. Fix that, and find out what else is in the same
position.

## Background

**Verified 2026-10-01.** `grep -rln include_formula_cached_values tests/` finds exactly one file,
`tests/options_downstream.rs`, and its only use is a **field round-trip** — set it to `false`, read
it back, assert it is `false`. That asserts the option is settable. **It asserts nothing about what
the option does, or does not do.**

The promise currently lives in two places, neither of which is executable:

- `src/options.rs`'s doc comment on the field, rewritten 2026-09-30: *"It does not control whether
  cached formula values are compared, despite its name… setting it `false` does not hide a
  cached-value change, and never has."*
- the letter sent to ForskScope on 2026-09-30, and the CHANGELOG entry for 3.3.0.

**The behaviour is real** — verified at m9-03's review: a cell whose formula text is identical on
both sides and whose cached value differs (20 against 999) produces the same `CellDiff` with the
flag `true` and with it `false`. **Nothing guards it.**

## Change scope

- `tests/` — one test file, or an addition to an existing one. Your call where it belongs; say why.
- `CHANGELOG.md` — `### Documentation` or `### Internal`. No library change.

## Non-change scope

- **Nothing under `src/`** except temporarily, for the failing-first demonstration.
- **Do not implement the old documented meaning.** That is the thing being guarded against.
- Do not rename or deprecate the field. Its name is misleading and that is a v4 question, already
  recorded.
- Do not extend the survey in §2 into fixes. Report it.

## Required implementation

**1. Pin the promise.** A test asserting that a formula cell whose **cached value differs** while its
**formula text is identical** produces the same `CellDiff` whether `include_formula_cached_values`
is `true` or `false`. Assert the diffs are equal, not merely that both are non-empty — the promise
is *"setting it false does not hide a cached-value change"*, so the two results must agree.

**2. Name it for the promise, not the mechanism.** Someone implementing the old doc comment in good
faith will read whatever fails. The test name and its comment should say: *this option does not gate
cached-value comparison; that is a commitment, not an accident; if you are here because you made it
gate one, the decision is recorded in `src/options.rs`'s doc comment and in 3.3.0's CHANGELOG.* A
guard that explains itself is the difference between someone reverting their change and someone
deleting the test.

**3. Failing-first, and the mutation is the interesting part.** Do not break the test by deleting an
assertion. **Implement the old meaning** — make the flag actually gate cached-value comparison — and
show the new test fails. That demonstrates the guard catches the real thing rather than a proxy.
Restore `cmp`-identical.

**4. Survey, and report only.** `include_formula_cached_values` is one promise. Find the others:
**doc comments on public items that promise what the code does *not* do, or commits to *not* doing
something, where no test would fail if it changed.** Start from the ones this quarter touched —
`min_severity` ("not a resource control"), `Limits::hardened()`'s boundary table, `SheetRef::index`
("identifies a sheet; `name` does not") — and say for each whether a test pins it.

**Report the list. Do not write the tests.** If it is short, the next unit is small; if it is long,
the shape of the answer may be a convention rather than a pile of tests, and that is mine to decide.

## Required tests

The one in §1. Its own failing-first demonstration per §3.

## Acceptance criteria

1. The test exists, asserts equality of the two results, and is named and commented per §2.
2. Failing-first demonstrated **by implementing the old meaning**, not by weakening the test;
   restored `cmp`-identical.
3. The §4 survey reported, with a verdict per item.
4. `git diff src/` empty at the end. No public API change.
5. Gates as always, plus rule 003.

## Prohibited shortcuts

- Do not assert "both results are non-empty". The promise is that they are *the same*.
- Do not demonstrate failing-first by deleting the assertion.
- Do not fix anything the survey turns up.

## Known risks

**1. The test could pass for the wrong reason.** If the fixture's cached-value difference also
produces a *formula text* difference, the diff would be non-empty either way and the test would pass
even if the flag did gate cached values. **The formula text must be identical on both sides** — check
that it is, in the fixture, rather than assuming `rust_xlsxwriter` wrote what you meant.

**2. The survey may be larger than it looks.** Every doc correction this quarter created a promise.
That is the point of asking, and it is why the answer is reported rather than acted on.

## Required evidence

Under `.git-exclude/review-request/promises-01-pin-the-promise/evidence/`:

1. The test, and its output.
2. The failing-first run with the old meaning implemented, the diff of that mutation, and the `cmp`
   restore.
3. Proof the fixture's formula text is identical on both sides (Known risk 1).
4. The §4 survey, one line per item with a verdict.
5. Gate sweep, one scratch dir, deleted. `git diff src/`.

## Review request format

`.git-exclude/review-request/promises-01-pin-the-promise/README.md`, with:

- Where you put the test and why.
- The survey, and your read on whether it wants tests or a convention.
- Anything you found that is already pinned where you expected it not to be. I would rather be
  wrong about the size of this than right about its existence.
