# Handoff 01 — Two doc-only truths

**Governing RFCs.** RFC-021 (workbook metadata), RFC-007 (typed cell values)
**Roadmap.** M4
**Sequence.** Any. Cheapest of the three.

## Purpose

Remove two false statements from the source, and add one missing one.

Nothing here changes behaviour. Both findings are cases where a reader of the
code would conclude something untrue.

## Background

**RFC-021.** `src/meta.rs:28` says `compare_workbook_metadata` *"returns an
empty vec when metadata mode is `Ignore`"*, and `:39` says *"Metadata mode
default is `CompareAvailable` (RFC-021 §6)"*. **Neither mode exists.**
`WorkbookMetadataMode` is defined at RFC-021 line 73 and was never built; the
function's `_opts` parameter is unused and metadata comparison cannot be
configured or disabled at all.

**RFC-007.** `CellValue::Integer`, `::Duration` and `::Unsupported` cannot occur
through any `.xlsx` input this crate accepts — established by execution in
RFC-030 Handoff 01 and RFC-035 Handoff 05, and confirmed independently in
review. Six of the nine variants are reachable. Nothing in the public
documentation says so, so a consumer writing a match arm for `Integer` is
writing dead code and cannot tell.

## Change scope

`src/meta.rs` (comments), `src/model.rs` (doc comments on the three variants),
`CHANGELOG.md`. Possibly `docs/` if a natural home exists — but see non-change
scope.

## Non-change scope

- **Do not build `WorkbookMetadataMode`.** That is a public API addition and a
  later decision. This unit corrects the claim that it exists.
- **Do not remove the unreachable variants.** That is breaking and belongs to
  the v3 question.
- Do not change behaviour, signatures, or the fixture corpus.

## Required implementation

1. **`meta.rs`.** Replace both comments with what is true: metadata comparison
   always runs, is not configurable, and the `_opts` parameter is unused.
   Reference RFC-021's deferred status rather than its unbuilt design — a
   comment pointing at a design that was never implemented is how this defect
   arose.
2. **`model.rs`.** Document on each of `Integer`, `Duration` and `Unsupported`
   that it cannot be produced from `.xlsx` input, and why — `Integer` because
   calamine's xlsx reader routes all numerics through `f64`; `Duration` because
   `DurationIso` is emitted only by the `.ods` reader; `Unsupported` because
   nothing in this crate constructs it. Say what a consumer should conclude:
   a match arm for these is unreachable today, and they are retained against
   future format support rather than as live cases.
3. **CHANGELOG** under `[Unreleased]`, as documentation corrections.

## Required tests

None — no behaviour changes. Confirm the suite is unaffected rather than
assuming it.

## Acceptance criteria

1. No comment in `src/` claims `WorkbookMetadataMode` or any metadata mode
   exists. Verify with a grep and show it.
2. All three unreachable variants carry a doc comment stating the fact and its
   cause.
3. No behaviour change; the corpus is byte-identical; the suite is unaffected.
4. Gates green.

## Prohibited shortcuts

- Do not soften the `model.rs` wording into "may not be produced in all cases."
  It is *cannot*, through any input this crate accepts, and hedging it would
  reproduce the defect in gentler language.
- Do not delete `meta.rs`'s comments without replacing them. An absence is
  better than a lie but worse than the truth.
- Do not implement anything to make a comment true.

## Required evidence

- The greps showing no surviving false claim
- The diff
- Suite unaffected; corpus hash unchanged

## Review request format

Per development policy §9.2.
