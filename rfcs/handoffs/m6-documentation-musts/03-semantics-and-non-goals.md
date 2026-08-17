# Handoff 03 — Semantics and non-goals

**Governing requirements.** NF-026 (**MUST**) — *"Document non-goals and
limitations clearly."* NF-027 (SHOULD) — *"Document comparison semantics with
examples: typed value change, formula change, sheet rename, inserted row, and
warning handling."*
**Roadmap.** M6
**Sequence.** After unit 01. Independent of 02, though both add pages.

## Purpose

Meet NF-026, and NF-027 with it. They pair because they are the same question
from opposite sides: what this engine does, and what it does not.

## Background

Neither page exists. NF-026 has been unmet since v2.0.0.

The threat model has a non-defences section, but that is security scope — what
an attacker cannot achieve. NF-026 is broader: what the library does not attempt
at all, and where it is limited.

**This is the harder of M6's two writing units, and not because of length.**
The material is uncomfortable. An honest limitations page has to say that four
public types are permanently empty, that three `CellValue` variants cannot occur,
that number formats are unavailable because calamine keeps `mod formats`
private, and that thirteen RFCs shipped in part. All of that is already true and
already recorded across `CHANGELOG.md`, RFC statuses, and M4's doc comments. It
has never been collected where a consumer would find it.

Collecting it is the requirement. It is also the inventory the v3 question needs
— see this milestone's README.

## Change scope

Two new pages under `docs/src/`, `docs/src/SUMMARY.md`,
`docs/src/README.md`'s contents list, the harness inclusions from unit 01,
`CHANGELOG.md`.

## Non-change scope

- **Nothing under `src/`** except harness inclusions.
- **Do not fix anything you document.** If writing the limitations page reveals
  a defect rather than a limitation, stop and report. The two are different: a
  limitation is a deliberate or upstream-imposed boundary; a defect is code not
  doing what it says.
- Do not propose v3 or recommend model changes. The inventory is the
  deliverable; the decision is the owner's.

## Required implementation

### The semantics page (NF-027)

1. **One worked example per named scenario**: typed value change, formula
   change, sheet rename, inserted row, warning handling. All five are named in
   the requirement.
2. **Each must show what the caller actually observes** — which `CellDiff`
   fields are populated, which `SheetChange` variant appears, what
   `DiffSummary` reports. A reader should be able to predict the output shape
   before running it.
3. **The inserted-row example must state which `AlignmentMode` it assumes.**
   Under `Positional` an inserted row makes every subsequent row differ; under
   `RowSignature` it is detected as an insertion. That difference is the single
   most surprising behaviour in the engine and the most likely to be
   misattributed to a bug.
4. **Warning handling means diagnostics.** Show a `Diagnostic` reaching the
   caller — `DiagnosticSeverity`, and that a comparison can succeed while
   reporting them.

### The non-goals and limitations page (NF-026)

5. **Non-goals** — what is deliberately out of scope. Cell formatting
   comparison, decryption of encrypted workbooks, formula evaluation, writing
   or merging workbooks, formats other than `.xlsx`. Say *deliberately* where
   it is deliberate.
6. **Limitations**, each with its cause, distinguishing three kinds because a
   reader's response differs for each:
   - **Upstream** — `CellNumberFormat` is always `None` because calamine keeps
     `mod formats` private; `WorkbookObjectChange` is empty because object
     content is unavailable. These may change when upstream changes.
   - **Deliberate deferral** — `serde::Deserialize` is not implemented;
     `FormatChange` and `WorkbookChange` are reserved.
   - **Unreachable-by-construction** — `CellValue::Integer`, `::Duration`,
     `::Unsupported`, and `ReadErrorKind::Other` cannot occur through any
     `.xlsx` input this crate accepts. M4 unit 01 and M5 unit 03 established
     the wording; reuse it.
7. **A consumer-facing statement of what the limits do.** `Limits::default()`
   leaves the linear bounds unset; `hardened()` sets all of them. M4 unit 04
   fixed `max_cells_compared` to bound coordinates rather than diffs — a
   `hardened()` caller may now see `LimitExceeded` where 2.3.0 succeeded. Link
   the threat model rather than duplicating it.
8. **Do not soften any of it.** "Currently limited" where the honest word is
   "not implemented" is how the statements M4 removed got written in the first
   place.

## Required tests

Both pages' Rust examples must be covered by unit 01's harness. State the count
per page and demonstrate one failing when deliberately broken.

The semantics page's examples should be **run**, not merely compiled, wherever
they do not need a real file — an example claiming a specific `DiffSummary`
output is a claim, and a claim this milestone can check cheaply should be
checked. Say which examples run and which are `no_run`, and why.

## Acceptance criteria

1. A semantics page exists with a worked example for each of NF-027's five
   scenarios.
2. Each shows the observable result, not just the call.
3. The inserted-row example names its `AlignmentMode` and explains the contrast.
4. Warning handling shows a `Diagnostic` reaching the caller.
5. A non-goals and limitations page exists, linked from `SUMMARY.md` and the
   index.
6. Non-goals are listed and marked as deliberate where they are.
7. Limitations are listed with causes, separated into upstream / deferred /
   unreachable.
8. The limits' consumer-facing behaviour is stated, including M4 unit 04's
   change.
9. Both pages' examples are harness-covered, counts stated, one shown failing;
   which examples run versus `no_run` is stated with reasons.
10. No behaviour change; corpus byte-identical; CHANGELOG under `### Added`
    naming NF-026 met and NF-027 addressed; gates green, full matrix.

## Prohibited shortcuts

- **Do not describe a limitation as a feature.** Four permanently-empty types
  are not "extensibility points".
- Do not omit a limitation because it is embarrassing. The audience is deciding
  whether to depend on this crate; finding it out later is worse for them and
  for us.
- Do not restate the threat model. Link it. Two documents drifting apart is how
  the `max_cells_compared` claim survived four releases.
- Do not write NF-027's examples from the source. Run them and record what came
  back.

## Known risks

- The limitations inventory is spread across `CHANGELOG.md`, RFC status lines,
  and doc comments in `src/`. Expect to find at least one item recorded in one
  place and contradicted in another — **report it rather than picking one.**
  That is a finding of exactly the kind M4 was built for.
- Examples asserting a specific `DiffSummary` will need fixtures. `no_run` is
  acceptable where a file is needed; do not weaken an example to make it
  runnable.

## Required evidence

- Both pages
- Harness counts per page, one deliberate-failure transcript
- The run-versus-`no_run` split, with reasons
- Any contradiction found while assembling the inventory
- CI run link

## Review request format

Per development policy §9.2, plus a mapping from NF-027's five named scenarios
to the examples that satisfy them, and the limitations inventory as a list — the
list itself is an input to the v3 decision that closes this milestone.
