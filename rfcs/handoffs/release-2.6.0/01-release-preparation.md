# Handoff — 2.6.0 release preparation

**Governing.** ROADMAP §6 (release cycles); Keep a Changelog
**Roadmap.** M8 closes with this release
**Sequence.** After M8 units 06 and 07 (merged, `beac0d8`). Before the cut.

## Purpose

Prepare 2.6.0 for cutting. **Preparation only — the cut is the owner's and the
architect's**, as with 2.5.1.

## What 2.6.0 is

M8: *the surface promises what the engine does not*. Six units.

It is a **minor** for three independent reasons, and the release summary must
say all three rather than leading with the new feature:

1. **A CLI exit-code contract changed.** A workbook whose only difference was
   sheet order exited `0` and printed "no differences found". It exits `1` now.
   A pure rename exited `1` and rendered nothing. Both were present from 2.0.0
   through 2.5.1. **This is the release's most important content** — a missed
   difference, which ROADMAP §6 rates with a crash.
2. **Default output changed for library callers.** `DiffSummary::diagnostics`
   now counts sheet-level diagnostics, which it never did, so it no longer
   disagrees with `DiffMetrics::diagnostics_emitted`. Counts rise on affected
   workbooks; the CLI's own output is byte-identical across the corpus.
3. **The `cli` feature's closure changed.** It now enables `serde` and `chrono`.

Plus: `--format json`; `min_severity` and `--no-warnings` made real; two new
builder methods; an installable, documented CLI.

**Unit 08 is not in this release** (`date_compare`, `max_cells_read`, removing
two `#[allow(dead_code)]`). It is small and additive and it is not worth holding
2.6.0 for — ForskScope's adoption target has moved twice already. It goes to
2.7.0. Do not pull it in.

## Change scope

`CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`.

## Non-change scope

- **Do not commit, tag, push or publish.** Leave everything uncommitted.
- **Nothing under `src/`, `tests/`, `docs/` or `rfcs/`.** If the sweep finds
  something wrong, **report it**. A release-prep unit that starts editing
  documentation is how a release acquires unreviewed changes.
- Do not add CHANGELOG entries. Assembling what is there is the task; writing
  new claims is not.
- Do not touch RFC-037 or the M8 handoff README. Marking the milestone closed is
  part of the cut.

## Required implementation

1. **Stamp `[Unreleased]` as `[2.6.0]`** with today's date. Check `[2.5.1]`'s
   heading for the exact separator this file uses rather than assuming.
2. **Order the sections per Keep a Changelog** — Added, Changed, Fixed,
   Documentation — after checking what order the existing entries actually use.
   This file has a convention and it is not always the canonical one.
3. **Write the release summary paragraph**, in the style of `[2.5.1]` and
   `[2.5.0]`. It must name the reorder/rename defect first, the two other
   behaviour changes second, and the features third. **Do not lead with
   `--format json`** — it is the most visible item and the least important.
   **Do not overstate:** we found the reorder defect ourselves, in a readiness
   review; no user reported it. Say that plainly or not at all.
4. **Bump the version** in `Cargo.toml` to `2.6.0` and update `Cargo.lock`
   (a `cargo build` does it; confirm the workspace entry, not the `1.2.0`
   dev-dependency, which is the benchmark's and must stay).
5. **Run the pre-release verification sweep** and report each result:
   - the full gate set: fmt; clippy `-D warnings` `--all-targets` under both
     `--all-features` and `--no-default-features`; the scoped stdout gate;
     `cargo deny check`; MSRV 1.88 build; doctests on stable and 1.88;
   - the five-feature test matrix — **and see the note below about your shell**;
   - fixture corpus byte-identical to `HEAD`;
   - **`cargo publish --dry-run` packaging cleanly at 2.6.0.** Check the
     packaged file list: `cli` now pulls `serde` and `chrono`, so confirm the
     dry run resolves the new closure and that nothing under `.git-exclude/` is
     packaged;
   - **the documented install command, run again at 2.6.0's version number**,
     from this tree into a scratch `--root`, with `--format json` producing a
     populated `iso`. It was verified in unit 06; verify it once more against
     the version being shipped;
   - **a last read of `docs/` and the CHANGELOG against each other.** Three
     sweeps have found eight items between them. Treat that as evidence of more,
     not as a reason to skip it. Pay particular attention to any statement about
     what `cargo install` does, what `cli` enables, and whether `iso` is
     populated — three claims this milestone changed.

## A note on your shell, and mine

Both of us have now produced a check that printed zeros because it never ran —
yours a glob in unit 06's anchor check, mine a `cargo test $f` that did not
word-split. See `.git-exclude/rules/002-comparing-two-builds.md`.

**For every command in the sweep, report the exit status and a line of real
output**, not a count alone. A `0` with no context is indistinguishable from a
command that did not execute.

## Required tests

None new. The sweep in item 5 is the test.

## Acceptance criteria

1. `[2.6.0]` stamped, dated, sections ordered per the file's own convention.
2. The summary names the reorder/rename defect first and does not overstate how
   it was found.
3. `Cargo.toml` at 2.6.0; `Cargo.lock` updated; the benchmark dev-dependency
   untouched.
4. Every sweep item reported **with its exit status and real output**.
5. `cargo publish --dry-run` clean, with the packaged file list checked.
6. The install command run at 2.6.0 and `iso` populated.
7. Corpus byte-identical.
8. Nothing outside the three files in the change scope is modified.
9. Nothing committed, tagged, pushed or published.

## Prohibited shortcuts

- **Do not `cargo publish`.** Not with `--dry-run` removed, not by accident.
  The cut is the owner's.
- Do not rewrite a shipped CHANGELOG entry. Entries under `[2.5.1]` and older
  are annotated, never edited.
- Do not "tidy" a CHANGELOG bullet from this release's units. They were
  reviewed as written.
- Do not fix anything the sweep finds. Report it.

## Known risks

- **The CHANGELOG's `[Unreleased]` section is large** — six units, three of
  which corrected sentences written by earlier units in the same section. Read
  it end to end for contradictions before stamping it; that is the most likely
  place for one.
- `cargo publish --dry-run` may warn about files it will include. Report the
  warning rather than acting on it.

## Required evidence

- The stamped CHANGELOG section
- Each sweep command, its exit status, and real output
- The `publish --dry-run` output including the file list
- The install run at 2.6.0
- Corpus byte-comparison

## Review request format

Per development policy §9.2. Additionally: state anything the docs-vs-CHANGELOG
read turned up, **including "nothing"** — and if it is nothing, say what you
compared, because the last three sweeps each found something.
