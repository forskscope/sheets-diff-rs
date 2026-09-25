# Handoffs — M8: the surface promises what the engine does not

**COMPLETE 2026-09-25.** All six units merged; 2.6.0 is prepared and awaiting the cut.

**Opened 2026-09-24.** 2.5.1 is published, so the constraint that held this back —
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
| **00** ✅ | [`hardened()` promises a guarantee we do not give](./00-hardened-promises-a-guarantee.md) | R1 | Documentation |
| 01 ✅ | [A reorder reports no difference](./01-moved-and-renamed-sheets.md) | A1 | **CLI exit contract** |
| 02 ✅ | [Two inert options, which are not the same option](./02-two-inert-options.md) | A2, A3, O3 | Behaviour where there was none |
| 03 ✅ | [`--format json`](./03-format-json.md) | A5 | New CLI surface |
| ~~04~~ | ~~Public values that describe what the engine does not do~~ — **withdrawn to [RFC-037](../../accepted/037-v3-scope.md)** | A4, O1 | — |
| 06 ✅ | [The CLI you can actually install](./06-the-cli-you-can-install.md) | F-1 + `cargo install` | **Feature set of the shipped binary** |
| 07 ✅ | [The builder omits two options](./07-the-builder-omits-two-options.md) | O-D | Additive API |
| ~~05~~ | ~~What `cells_read` counts~~ — **withdrawn to [RFC-037](../../accepted/037-v3-scope.md)** | A6 | — |

**M8's implementation is complete** — all six units merged as of `beac0d8`,
2026-09-25. What remains is the release: see
[`rfcs/handoffs/release-2.6.0/01-release-preparation.md`](../release-2.6.0/01-release-preparation.md).

**Order was: 00, then 01, then 06 and 07 in either order.**

**Units 04 and 05 were withdrawn on 2026-09-25** to
[RFC-037 (v3 scope)](../../accepted/037-v3-scope.md). Both are fixed by removing
or renaming public items, and cargo hands `^2` users a new minor automatically,
so a deleted name is a build failure they did not ask for. The owner's decision
was to stop paying for that with documentation workarounds and take the major
while it is still cheap.

The handoff files for 04 and 05 stay in place as source material for the v3
handoffs; **do not work them as written.**

**00 is numbered 00 rather than appended at the end so the order needs no
explanation** — M7 lost a round trip to an order line that said "03 ahead of
04+" and was silent about 02. Units added later (06, 07) are appended and carry
no ordering claim beyond the one rule: **05 is always last.** It comes first because it is small, doc-only, and most misleading right now:
2.5.1's notes say the sheet-read hole is fixed, and the sentence it corrects
invites a reader to hear "fixed" as "guaranteed".

01 is the integrity defect and is the substantive work. 05 moves every golden,
so every other unit's corpus check is cleaner before it runs — the same
reasoning that ordered M7's units 03 before 02.

**Units 00, 01, 02 and 03 are done** — 02 and 03 approved 2026-09-25.
**Units 04, 06 and 07 are written** and may be worked in any order or in
parallel; unit 05 is written against
the tree that 02–04 leave behind, since it moves every golden and a clean corpus
check is worth more to the other three than to itself.

**Units 06 and 07 came out of reviewing 02 and 03**, which is now the pattern
rather than the exception: every unit of this milestone has produced at least one
finding from someone looking at something adjacent.

**Unit 06 must ship in 2.6.0**, in the same release as `--format json`. Without
it, the release publishes a JSON format whose documented `iso` field is null for
everyone who installs the tool the documented way — and there is no documented
way, because `cargo install sheets-diff` produces no binary at all.

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
