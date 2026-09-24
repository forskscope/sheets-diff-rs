# Handoff — 2.5.1 release preparation

**Release.** 2.5.1 (patch)
**Sequence.** Last. All three f123 units are merged; `main` is at `1f82677`,
CI 18/18.

## Purpose

Prepare the release. **You do not cut it.** The commit, tag, push and
`cargo publish` are the architect's and the owner's — those are the irreversible
steps and they stay with us. Everything up to them is yours.

## What 2.5.1 is

A **patch**, and the reasoning matters because it is the thing most likely to be
got wrong: no public API changed, and **no comparison that previously succeeded
now fails.** `cells_read` kept its meaning, which is why the goldens held. What
changed is that a workbook which previously *aborted the calling process* now
returns a result or a clean error. Fixing a crash is a patch.

Two observable changes, both already in the CHANGELOG:
`LimitExceeded { CellsRead, observed }` reports the running box area at the
breaking cell rather than `max + 1`; and the cancellation poll counts streamed
records rather than dense-range positions.

Contents: the streaming read (PR #28), the two cancellation tests rebuilt so each
fails on the poll it names, and the documentation corrections — including two
public doc comments the streaming patch itself falsified.

## Change scope

`CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`.

## Non-change scope

- **Do not commit, tag, push or publish.** Leave everything uncommitted, as
  usual.
- **Nothing under `src/`, `tests/` or `docs/`.** If the sweep in item 4 finds
  something wrong, **report it** — a release-prep unit that starts editing
  documentation is how a release acquires unreviewed changes.
- Do not add entries to the CHANGELOG. Assembling what is there is the task;
  writing new claims is not.

## Required implementation

1. **Stamp `[Unreleased]` as `[2.5.1]`** with today's date, matching the
   format of the entries below it — check `[2.5.0]`'s heading for the exact
   separator this file uses rather than assuming.
2. **Order the sections per Keep a Changelog** — Added, Changed, Fixed,
   Documentation. Check what order the existing entries use before reordering;
   this file has a convention and it is not always the canonical one.
3. **Write the release summary paragraph** beneath the heading, in the style of
   `[2.5.0]` and `[2.4.1]`. It should say what the release is for in a few
   sentences: a workbook of a few kilobytes could abort the calling process, the
   bound fired after the spend, and both are fixed. **Do not overstate.** We did
   not observe the abort ourselves; it is ForskScope's report, and the threat
   model already phrases that correctly — follow it.
4. **Bump the version** in `Cargo.toml` to `2.5.1` and update `Cargo.lock`
   (a `cargo build` does it; confirm the workspace entry, not the `1.2.0`
   dev-dependency, which is the benchmark's and must stay).
5. **Run the pre-release verification sweep** and report each result:
   - the full gate set: fmt, clippy `-D warnings` all targets all features, the
     scoped stdout gate, `cargo deny check`, MSRV 1.88 build, doctests on stable
     and 1.88;
   - the five-feature-combination test matrix;
   - fixture corpus byte-identical to `HEAD`;
   - `cargo publish --dry-run` packaging cleanly at 2.5.1;
   - **a last read of `docs/` and the CHANGELOG against each other** — does any
     page still contradict what the release notes say? Two sweeps have already
     found five items between them; treat that as evidence of more rather than a
     reason to skip it.

## Required tests

None new. The sweep in item 5 is the test.

## Acceptance criteria

1. `[Unreleased]` is stamped `[2.5.1]` with the correct date format.
2. Sections are in a stated order, and the review request says which convention
   was followed and why.
3. A release summary exists and does not claim we observed the process abort.
4. `Cargo.toml` and `Cargo.lock` read `2.5.1`; the `1.2.0` dev-dependency is
   untouched.
5. Every item in the sweep is reported with its result, including the ones that
   passed.
6. Nothing committed, tagged, pushed or published.
7. Anything the sweep finds is **reported, not fixed**.

## Prohibited shortcuts

- **Do not soften or embellish the security wording.** The threat model and the
  CHANGELOG entry were both reviewed; the summary should agree with them, not
  improve on them.
- Do not skip a sweep item because the previous unit already ran it. The point
  of a pre-release sweep is that it runs against the release commit.
- Do not "tidy" a CHANGELOG entry while stamping it. Those entries are reviewed.

## Known risks

- `Cargo.lock` has **two** `sheets-diff` entries: this workspace at 2.5.0, and
  `1.2.0` from crates.io as the v1.2-vs-v2 benchmark's dev-dependency. Only the
  workspace one moves.
- The threat model says the fix is "planned as 2.5.1 … not been cut". That
  wording becomes wrong the moment it is cut — but changing it is **not yours**,
  because it would be an unreviewed edit landing in the release. Report it; the
  architect will handle it at the cut.

## Required evidence

- The diff
- Every sweep result from item 5, including the passes
- `cargo publish --dry-run` output
- What item 5's documentation read covered, and what it found

## Review request format

Per development policy §9.2, plus the sweep results as a table.
