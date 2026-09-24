# Handoffs — M8: the surface promises what the engine does not

**OPEN 2026-09-24.** 2.5.1 is published, so the constraint that held this back —
unit 01 changes a CLI exit contract, and 2.5.1 was a patch fixing a denial of
service — is discharged.

Five units, one theme: **every item is a public surface a competent reader
predicts wrongly.** That is the owner's second design principle applied to what
this crate advertises — a name, a flag or a value someone would get wrong is a
defect here, not a documentation gap.

Source: dev-team task 001's readiness review (findings A1–A6), plus the
`cells_read` question ForskScope left to us, plus **O1**, found by the
implementer during unit 01 and folded into unit 04 on 2026-09-24.

## Queue

| | Unit | Item | Release impact |
|---|---|---|---|
| **00** | [`hardened()` promises a guarantee we do not give](./00-hardened-promises-a-guarantee.md) | R1 | Documentation |
| 01 | [A reorder reports no difference](./01-moved-and-renamed-sheets.md) | A1 | **CLI exit contract** |
| 02 | [Two inert options, which are not the same option](./02-two-inert-options.md) | A2, A3 | Behaviour where there was none |
| 03 | [`--format json`](./03-format-json.md) | A5 | New CLI surface |
| 04 | [Public values that describe what the engine does not do](./04-values-that-describe-what-the-engine-does-not-do.md) | A4, **O1** | Documentation |
| 05 | What `cells_read` counts | A6 | **Public metric; every golden moves** |

**Order: 00, then 01, then the rest; 05 last.**

**00 is numbered 00 rather than 06 so the order needs no explanation** — M7 lost
a round trip to an order line that said "03 ahead of 04+" and was silent about
02. It comes first because it is small, doc-only, and most misleading right now:
2.5.1's notes say the sheet-read hole is fixed, and the sentence it corrects
invites a reader to hear "fixed" as "guaranteed".

01 is the integrity defect and is the substantive work. 05 moves every golden,
so every other unit's corpus check is cleaner before it runs — the same
reasoning that ordered M7's units 03 before 02.

**Units 00 and 01 are done** — 01 approved 2026-09-24. **Units 02, 03 and 04 are
written** and may be worked in any order or in parallel; unit 05 is written against
the tree that 02–04 leave behind, since it moves every golden and a clean corpus
check is worth more to the other three than to itself.

**Units 02 and 03 meet at one flag.** `--no-warnings` is defined by 02 and must
apply to JSON in 03, or the milestone ships a newly inert combination. Whichever
lands second wires it; both handoffs say so.

**Unit 04 needs one decision from the owner before it is worked** — whether O1 is
documented now and renamed at v3 (recommended) or given honest variants in 2.6.0.

**Unit 04 covers two shapes of the same defect.** A4 is four `DiagnosticKind`
variants nothing constructs. O1 is the mirror: `SheetMatchReason::IndexAndContent`
*is* constructed — at three sites in `src/matcher.rs` — and names a content check
the matcher never performs, while `ContentSimilarity` is constructed nowhere. A
value that is never produced and a value that is produced with a false name are
the same failure for the reader.

## Why these are one milestone

Not because they are small, and not because they are all in the CLI — unit 05 is
a library metric and unit 04 is a doc comment.

They share a reader. Someone reads `--help` and believes `--no-warnings` does
something. Someone reads `DiagnosticOptions::min_severity` and believes they can
set it. Someone reads `cells_read` and believes it counts cells that were read.
Someone reads a stable diagnostic-code table and believes `limit_truncated_cells`
can occur. Someone reads `SheetMatchReason::IndexAndContent` on a reported rename
and believes the content was compared. Someone runs the CLI on reordered sheets
and believes exit `0`.

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
