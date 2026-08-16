# Handoff 06 — Threat model, advisory policy, and honest disclosure

**Governing RFC.** [RFC-035](../../accepted/035-resource-safety-and-supply-chain-governance.md) §5.7–5.8, RFC-016
**Roadmap.** M2, decision D4 — *"a sufficient threat model is required, not a minimal one"*
**Sequence.** Last. It documents what the other units actually built.

## Purpose

Write down what this crate defends against, what it does not, and how each
claim is checked — so a consumer's decision to re-enable is informed rather
than a leap of faith. Then correct the places where the project still says
things that are not true.

This is the last unit in M2.

## Background

Units 02–05 changed the security posture materially: both `quick-xml`
advisories cleared, a `deny.toml` gate on every build, superlinear paths
bounded by default, `#![forbid(unsafe_code)]` in force, and four
silent-wrong-answer defects fixed. None of it is written down anywhere a
consumer can read.

ForskScope's own procedure names a threat model as an input to their
re-enablement decision. We are asking them to trust a library whose safety
properties exist only in review records.

## Applicable requirements

NF-015 (no network), NF-016 (no telemetry), NF-017 (no macro/formula
execution), NF-018 (no path leakage), NF-019, NF-026 (document non-goals and
limitations), RFC-016, RFC-035 §5.7–5.8, roadmap §6.

## Change scope

- New `docs/src/maintainers/threat-model.md`
- `docs/src/SUMMARY.md` — add it to the mdbook nav
- `src/objects.rs` — the four stale/incorrect coverage strings (§3)
- All seven `tests/fixtures/generated/*/expected.json` — re-blessed as a
  consequence of §3
- `CHANGELOG.md` — the 2.2.3 metrics correction (§4)
- `tests/fixtures/corpus/README.md` — the first-bless lesson (§5)

## Non-change scope

- Do **not** fix `DiffMetrics.cells_compared` itself. See §4 — it is a real
  defect, it is M3's, and correcting the *claim* is this unit's job.
- Do not implement hyperlinks, merged regions, tables, or pivot tables. §3 is
  about describing what we do and do not compare, not extending it.
- Do not change comparison behaviour. The goldens move in this unit for exactly
  one reason: a diagnostic message string changed. If anything *else* moves,
  stop and report.

## Required implementation

### 1. The threat model — `docs/src/maintainers/threat-model.md`

Path mirrors ForskScope's own so the two sit side by side. "Sufficient, not
minimal" means a reader can decide whether to accept a given risk. Cover:

**Assets.** Confidentiality of workbook content; availability of the host
process; and — first-class, per roadmap §6 — **integrity of the diff result**.
State plainly why the third belongs: in a diff/merge workstation a silently
missed difference means a user is shown "identical", accepts a merge, and loses
data. Unit 05 fixed four such paths; that section should say so, with the
`formula` fixture as the worked example of how one hid in plain sight.

**Trust boundary.** The workbook bytes are untrusted. The *caller* is trusted —
we defend against hostile input, not a hostile consumer.

**Actors.** Someone who supplies a workbook. Not a network attacker: there is no
network.

**Surfaces, with mitigation and residual risk each.** At minimum: the zip
container; XML parsing; our normalisation; alignment; and the bounds themselves.
For each, say what checks it and what remains. Be specific — "bounded" is not a
mitigation, `max_alignment_product = 25_000_000` is.

**Explicit non-defences.** Say what we do not do: no sandboxing, no defence
against a malicious caller, no guarantee for arbitrarily large input, no macro
or formula execution (NF-017), no attempt at Excel-complete semantics.

**Verification map.** For each control, name where it is machine-checked:
`deny.toml` bans versus NF-015, the `deps` job, `forbid(unsafe_code)`, the
`fuzz-smoke` targets, the feature matrix, the golden corpus. A control nothing
checks should be marked as such rather than quietly listed alongside ones that
are.

**Residual risks worth naming**, all surfaced during M2 and none currently
fixed: `CellValue::Duration` is unreachable through `.xlsx` (unit 05 §6); two
correctly-computed diffs can share a display address (unit 05 §12); the bytes
path owns a copy where it could borrow, doubling peak memory (unit 04 review
§3); `DiffMetrics.cells_compared` is wrong (§4 below).

### 2. The advisory-response policy

Into the same document, per RFC-035 §5.7. Four steps: the `deny.toml` gate fails
the build — that is the trigger, there is no manual watch; assess reachability
from untrusted workbook input; if reachable, respond with a bump, an upstream
fix, or a documented exception carrying an expiry — never silence; notify known
consumers, because a consumer's fail-closed posture costs them more than it
costs us.

Say what happened here: the advisories landed, nobody noticed, the consumer
noticed and switched us off, and it took two months. The policy exists because
that happened.

### 3. `src/objects.rs` — a rewrite, not a version bump

Four strings say variants of *"calamine 0.35 does not expose object content
(charts, images, comments, hyperlinks, tables, pivot tables…)"*. Two problems:
the version is stale, and **the claim is now partly false** — unit 01's spike
established that 0.36 does expose hyperlinks, merged regions, tables and pivot
tables. We simply do not call them.

So the fix is not `5`→`6`. The message must become true: this crate does not
compare those objects; for some of them the data is available upstream and
unused; for others (styles, number formats — `mod formats` is still private)
it is not available at all. Distinguish the two cases, because a consumer
reading "not exposed by the parser" plans differently from one reading "exposed
but not yet used".

This is the diagnostic that appears in **every** comparison result, so the
wording is user-facing. Keep it short.

Changing it moves all seven `expected.json` files, because the string is
embedded verbatim in each. That is expected here and is the reason this work was
held back from unit 05.

### 4. `CHANGELOG.md` — correct the 2.2.3 metrics claim

The 2.2.3 entry says `DiffMetrics.cells_compared` *"now counts all coordinate
pairs visited, not just changed cells."* It does not, and did not.

Verified at `0ba6aeb`: the accumulator is
`summary.cells_changed + count(cell_diffs where value.is_none() && formula.is_none())`,
while `build_sheet_diff` skips any coordinate where both are `None`. The second
term is therefore always zero, and `cells_compared == cells_changed`.

Annotate the 2.2.3 entry in place — do not delete it — as the 2.2.0 parallel
entry already was. Say the claim was wrong when written and remains wrong, and
that the metric itself is scheduled for M3.

The 2.2.0 correction already landed with the parallel removal; leave it alone.

### 5. The corpus guide — record the first-bless lesson

`tests/fixtures/corpus/README.md` should carry what unit 05 taught, because it
will otherwise be relearned.

The `formula` fixture existed from RFC-015 to test formula-versus-value changes.
It recorded the change at **A1**, the label cell, with spurious diagnostics at
**A2** where the formula actually was. It was wrong from the day it was blessed
and stayed wrong through every subsequent run, because a golden only detects
*change* — it cannot detect having been born wrong. Unit 05's D-04 fix moved it
to A2.

The lesson: **a golden's first bless is the one moment its content is
unreviewed.** Blessing a new scenario means reading the produced JSON and
deciding it is right, not observing that the test then passes. Say that plainly.

## Required tests

No new behavioural tests. The existing suite covers §3's change: the seven
goldens must move and everything else must not.

## Acceptance criteria

1. `docs/src/maintainers/threat-model.md` exists, covers every element in §1,
   and is linked from `SUMMARY.md`.
2. The advisory-response policy is written per §2.
3. `src/objects.rs`'s messages are true of calamine 0.36 and distinguish
   "unavailable upstream" from "available but not used".
4. All seven goldens are re-blessed, and the review request shows the diff is
   **only** the changed message string.
5. The 2.2.3 metrics entry is annotated in place, saying the claim was and is
   wrong and that the fix is M3's.
6. The corpus guide carries the first-bless lesson.
7. Matrix, `fmt`, `clippy -D warnings`, `deny`, MSRV, CI all green.

## Prohibited shortcuts

- Do not write a threat model that only lists what we defend against. The
  non-defences and residual risks are the half a reader actually needs.
- Do not fix `cells_compared` here, and do not describe it as fixed.
- Do not claim a control is verified where nothing verifies it.
- Do not let any golden change beyond the message string. If one does, that is a
  behaviour change and a finding.
- Do not soften §3's wording into implying we compare objects we do not.

## Compatibility constraints

The diagnostic message is user-visible. Its `DiagnosticKind::code()` —
`unsupported_workbook_feature` — is the stable programmatic surface and **must
not change**; only the human-readable message does. Consumers matching on
`code()` are unaffected, which is the contract we published.

## Security constraints

This unit is the disclosure half of a security release. Its failure mode is
overclaiming — a threat model that reads as a guarantee is worse than none,
because it invites reliance the code does not support. Where a control is
partial, say so.

## Known risks

- Writing a threat model tempts one toward reassurance. Resist it; a reader
  deciding whether to re-enable needs the limits, not comfort.
- Re-blessing seven goldens at once is exactly the operation §5 warns about.
  Read the diff.

## Required evidence

- The threat model and the `SUMMARY.md` link
- The `objects.rs` diff and the resulting golden diff, shown to be the message
  string only
- The annotated 2.2.3 entry
- The corpus-guide addition
- Full matrix, gates, MSRV, CI run link

## Review request format

Per development policy §9.2, plus an explicit statement that the golden diff
contains nothing but the changed message.
