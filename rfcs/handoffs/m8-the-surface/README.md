# Handoffs — M8: the surface promises what the engine does not

**Authorized 2026-09-24. Opens when 2.5.1 ships**, because unit 01 changes a CLI
exit contract and 2.5.1 is a patch fixing a denial of service.

Five units, one theme: **every item is a public surface a competent reader
predicts wrongly.** That is the owner's second design principle applied to what
this crate advertises — a name, a flag or a value someone would get wrong is a
defect here, not a documentation gap.

Source: dev-team task 001's readiness review (findings A1–A6), plus the
`cells_read` question ForskScope left to us.

## Queue

| | Unit | Item | Release impact |
|---|---|---|---|
| 01 | [A reorder reports no difference](./01-moved-and-renamed-sheets.md) | A1 | **CLI exit contract** |
| 02 | Two inert options | A2, A3 | Behaviour where there was none |
| 03 | `--format json` | A5 | New CLI surface |
| 04 | Four diagnostic codes nothing produces | A4 | Documentation |
| 05 | What `cells_read` counts | A6 | **Public metric; every golden moves** |

**Order: 01 first, 05 last.** 01 is the integrity defect. 05 moves every
golden, so every other unit's corpus check is cleaner before it runs — the same
reasoning that ordered M7's units 03 before 02.

**Only unit 01 is written.** Units 02–05 will be written when 01 lands, against
the tree as it stands then, rather than now against a tree that 2.5.1 and unit
01 are both about to change. Their scope is known and recorded in `ROADMAP.md`;
what is not yet known is what the code looks like when they start.

## Why these are one milestone

Not because they are small, and not because they are all in the CLI — unit 05 is
a library metric and unit 04 is a doc comment.

They share a reader. Someone reads `--help` and believes `--no-warnings` does
something. Someone reads `DiagnosticOptions::min_severity` and believes they can
set it. Someone reads `cells_read` and believes it counts cells that were read.
Someone reads a stable diagnostic-code table and believes `limit_truncated_cells`
can occur. Someone runs the CLI on reordered sheets and believes exit `0`.

Every one of them is wrong, and none of them was careless — each is a surface
that was built honestly and then drifted, or that promised ahead of the engine.

## Standing constraints

- **The fixture corpus must not move** — except in unit 05, where it will, and
  where each moved golden must be shown to differ only in `cells_read`.
- **Do not remove a public variant to fix a finding.** `DiagnosticKind` is
  `#[non_exhaustive]`, but a removal still breaks a matcher. Removal is a v3
  question; documenting is unit 04's answer.
- **Every fix arrives with a test that fails without it**, demonstrated by
  removing the specific code under test — never by reverting a whole file. That
  mistake is why M7's cancellation test did not test anything, and it was the
  architect's.
- Gates as always: fmt, clippy `-D warnings`, the scoped stdout gate, `deny`,
  MSRV 1.88, doctests at 1.88, the full matrix.
