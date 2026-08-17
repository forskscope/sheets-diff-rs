# Handoff 05 — Record corrections

**Governing.** F-I, F-L (architect findings); RFC 000 (lifecycle and record
integrity)
**Roadmap.** M6
**Sequence.** Any. Independent of everything. The smallest unit in the
milestone.

## Purpose

Two wrong statements in `CHANGELOG.md`, one shipped and one not, plus two RFC
status lines that M5 made stale.

## Background

### F-I — the corpus count, shipped in 2.4.0

`CHANGELOG.md`'s 2.4.0 entry reads *"The fixture corpus grew from 7 to 18
scenarios."*

It grew to **19**. Seven scenarios predate M3 and twelve were added in
`2679870` — verified by first-appearance date across
`tests/fixtures/generated/*/`, and `ls -d | wc -l` returns 19 today.

The entry was wrong when written and shipped in the release whose theme was
removing statements that are not true. **Annotate, do not rewrite** — 2.4.0 is
tagged and published, and this project's convention (applied four times in that
file already) is that a shipped entry is a record of what was claimed.

Worth noting how it was caught: by needing to state the number to someone
outside the project. Nothing in CI checks a count asserted in prose, and that
remains true after this unit.

### F-L — "hard compile error", unreleased

`[Unreleased]`'s stdout-gate entry says the `#[allow]` bypass becomes
*"`error[E0453]`, a hard compile error an inner `#[allow]` cannot downgrade"*,
and two sentences later that `cargo build` *"still passes unchanged"*.

Both are true; together they invite the inference that a build catches a bypass.
It does not — verified: with the `#[allow]` and a `println!` present,
`cargo build --all-features` exits **0**, because `rustc` does not evaluate
`clippy::` tool-lint level conflicts when clippy is not the driver. Only clippy
does.

**This entry is unreleased, so correct it in place** — no annotation. The
distinction from F-I is the point: annotate what shipped, fix what has not.

The imprecision originated in my handoff wording and propagated; it is recorded
here as a correction to the record, not to the implementer.

### Two stale RFC status lines

M5 closed work that two RFC statuses do not reflect:

- **NF-022** (*SHOULD* — "Non-UTF-8 path fixtures should be tested on platforms
  where practical") is now satisfied on Unix by M5 unit 02's
  `#[cfg(unix)]` test. Find where NF-022's state is recorded, if anywhere, and
  make it true. **If it is recorded nowhere, say so** — that is a finding about
  the requirement register, not a task to invent one.
- Check RFC-016's and RFC-032's status lines, both rewritten during M5, against
  what actually landed. They were written by the implementer and reviewed by me,
  but neither of us re-read them after all four M5 units merged.

## Change scope

`CHANGELOG.md`, and whichever files under `rfcs/done/` the status check
requires.

## Non-change scope

- **Nothing under `src/`, `tests/`, or `docs/`.** This is record-keeping.
- **Do not rewrite 2.4.0's entry.** Annotate beneath it.
- Do not restate a milestone's history. A correction says what was claimed, what
  is true, and moves on.

## Required implementation

1. **Annotate 2.4.0's corpus-count claim** — state the correct number and the
   basis for it, matching the annotation style already used four times in that
   file.
2. **Correct the F-L clause in `[Unreleased]` in place** — the protection holds
   under any clippy invocation, not under `cargo build`. Say which, precisely;
   the surrounding sentence about `cargo build` passing is correct and should
   stay.
3. **Verify RFC-016's and RFC-032's status lines** against the merged state and
   correct anything that does not hold. Report what you checked, including if
   the answer is "both correct".
4. **Determine whether NF-022's state is recorded anywhere**, and make it true
   if so. Report the answer either way.

## Required tests

None — no code changes. The verification *is* the work: every claim this unit
touches must be independently confirmed rather than taken from the finding text,
including F-I's count. Re-derive 19 yourself.

## Acceptance criteria

1. 2.4.0's corpus-count claim is annotated, not rewritten, with the correct
   count and its basis.
2. The count was independently re-derived, and the review request says how.
3. `[Unreleased]`'s F-L clause is corrected in place and states that the
   protection is clippy-scoped.
4. RFC-016's and RFC-032's status lines are verified; findings reported, or
   confirmed correct.
5. NF-022's recorded state is determined and reported.
6. Nothing under `src/`, `tests/`, or `docs/` changed.
7. Corpus byte-identical.
8. No shipped entry rewritten.
9. Gates green, full matrix.
10. The review request lists every claim checked and its outcome, including
    those that needed no change.

## Prohibited shortcuts

- **Do not take F-I's count on faith because I wrote it.** I have been wrong
  about a count in this project before — `rfcs/README.md` claimed RFC-033 was
  cited 11 times when `git grep` proved 20, and the dev team caught it.
- Do not annotate 2.4.0 by editing its sentence "just slightly". Either it
  stands with a correction beneath it or the convention means nothing.
- Do not skip criterion 4 because a status line looks plausible. Both were
  rewritten mid-milestone and neither was re-read after the last unit merged.

## Known risks

- Whether NF-022 is recorded anywhere is genuinely unknown to me. The
  requirements register lives outside the tracked tree in
  `.git-exclude/specs/`, and nothing in `rfcs/` tracks per-NF status. "Recorded
  nowhere" is a legitimate and likely answer — report it plainly rather than
  creating a tracking mechanism this unit did not ask for.

## Required evidence

- The annotation and the in-place correction
- The independent re-derivation of the corpus count
- What was checked for criteria 4 and 5, with outcomes
- CI run link

## Review request format

Per development policy §9.2, plus the full list of claims checked and their
outcomes — including the ones that turned out correct.
