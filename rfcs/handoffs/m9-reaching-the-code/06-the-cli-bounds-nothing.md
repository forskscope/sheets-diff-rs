# Handoff 06 — The CLI bounds nothing, and JSON builds the whole result as one `String`

**Milestone:** M9, unit 06. **Items:** F-3, and F-5's leftover.
**Written:** 2026-09-30. **Measure before deciding** — that was the condition when this was routed,
and f135 has since shown why.

## Purpose

A CLI user comparing a workbook they did not author gets `Limits::default()` and no way to tighten
it, and `--format json` assembles the entire result into one `String` before writing a byte.
Establish what that actually costs, then decide what to change. **Do not add flags before the
measurement.**

## Background

**What is true today, verified 2026-09-30.** `src/main.rs` builds options with
`DiffOptions::builder()` … `.build()` (`:173`–`:177`), so the CLI gets `Limits::default()` —
`max_alignment_product` 25,000,000 and `max_input_bytes` 500 MiB. It exposes **no flag** for any
limit: `grep -cE "max_input_bytes|max_cells_read|max_sheets|max_diffs_returned|hardened"
src/main.rs` is **0**. So the four limits that default to `None` — `max_sheets`, `max_cells_read`,
`max_cells_compared`, `max_diffs_returned` — are unbounded for every CLI invocation and cannot be
set.

**Why this is worth measuring rather than assuming.** f135 measured a 666 KiB workbook producing a
**158.9 MiB** serialised result, from diagnostics alone. That specific cause is fixed. The general
shape is not: `to_json_pretty` returns a `String`, so the whole result exists in memory twice — as
the `WorkbookDiff` and as its serialisation — before anything reaches stdout, and nothing in the CLI
bounds either. **The question is whether a plausible untrusted workbook can still make that large.**

**F-5's leftover, inherited here.** The `cli`-only CI leg now also compiles `serde`, so no leg
exercises a genuinely minimal build. Whatever else this unit does, **one leg must stay honest about
a build with neither feature.**

## Change scope

- `src/main.rs` — only if the measurement justifies it.
- `.github/workflows/ci.yaml` — the minimal-build leg (F-5).
- `docs/` — whatever the measurement makes true or false.
- `CHANGELOG.md` if anything user-visible changes.

## Non-change scope

- **Do not change `Limits::default()`.** Those are library defaults chosen by measurement in
  RFC-035; the CLI is not the place to relitigate them.
- **Do not change the JSON output's shape or field names.** The goldens are the contract.
- Nothing in `src/` beyond `main.rs` unless the measurement says the library is at fault, in which
  case **stop and report** — that would be a defect unit, not this.

## Required implementation

**1. Measure, and report before changing anything.** Three numbers, on a workbook shaped like real
data rather than a pathological one:
   - peak RSS of the `sheets-diff` binary for a large-but-ordinary comparison, `--format json` and
     the text renderer, at a few sizes;
   - the serialised length against the input size — the ratio, and whether it is bounded;
   - what the four `None` limits would have to be set to in order to matter at those sizes.

   **If the ratio is unremarkable, say so and change nothing but F-5 and the docs.** A CLI that
   allocates proportionally to its input is not a defect. The finding this unit is looking for is
   *disproportion*.

**2. Only if the measurement shows disproportion**, propose — do not implement — the smallest thing
that fixes it. My expectation, to be confirmed or contradicted: streaming the JSON writer so the
result is not doubled in memory is a better answer than a new flag, because it helps every caller
rather than the ones who know to pass it. **A flag that a user must know to set does not protect the
user who does not know.**

**3. Whether the CLI should expose limits at all is a design question, and mine.** If your
measurement says a bound is needed, propose the shape and let me rule. Candidates, so you can argue
against them rather than invent from scratch: a single `--hardened` switch selecting
`Limits::hardened()`; individual `--max-*` flags; or nothing, on the grounds that a CLI user who
needs limits should use the library. I lean to the first and against the second.

**4. Fix F-5.** One CI leg must build and test with `--no-default-features` and no `cli`, so a
minimal build stays honest. Check what the legs currently are before adding one — the point is
coverage, not a fifth leg.

## Required tests

- Whatever the measurement justifies. If nothing changes in `src/`, this unit adds no test and says
  so — **do not invent a test to look thorough.**
- If the JSON writer changes, the goldens must be byte-identical: same bytes, different path to
  them.
- The minimal-build CI leg is the test for F-5.

## Acceptance criteria

1. The three measurements, reported with the workbook shapes used.
2. A stated conclusion: disproportionate or not, with the number that decides it.
3. If disproportionate: a proposal, not an implementation, for §3.
4. F-5 fixed: a leg exists that builds with neither feature.
5. Goldens byte-identical if the JSON path changed at all.
6. Gates as always, plus rule 003.

## Prohibited shortcuts

- **Do not add flags because the item's title says "no `Limits`".** The title is the finding, not the
  fix; the routing note said measure first for exactly this reason.
- Do not measure with a pathological fixture and report it as the cost of ordinary use. State the
  shape; if you also measure a hostile shape, label it as such.
- Do not "fix" it by changing a default. A CLI that behaves differently from the library it wraps is
  a worse surprise than an unbounded one.

## Known risks

**1. The interesting answer may be "nothing is wrong".** That is a real result and the unit closes on
it. Five of this milestone's units found something; this one may not, and reporting no finding is
better than manufacturing one.

**2. Peak RSS is easy to measure badly.** Measure the whole process, from a cold start, and state the
build profile — a release binary and a debug one are not comparable. Do not use the tracking
allocator here; that measures the library, and the question is about the binary.

**3. Streaming JSON could change bytes.** Serde's pretty printer and a streaming writer can differ in
trailing newlines or escaping. If the goldens move by a single byte, **stop** — that is a change to
the output contract and it is mine to rule on, not a rounding error.

## Required evidence

Under `.git-exclude/review-request/m9-06-the-cli-bounds-nothing/evidence/`:

1. The measurements, with the generating script and the workbook shapes.
2. `grep` showing the CLI's current limit surface, before and after.
3. The CI legs before and after, and what the new one covers that none did.
4. Goldens: 0 differ, or the finding.
5. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/m9-06-the-cli-bounds-nothing/README.md`, with:

- The conclusion first: is it disproportionate, and by what number.
- Your proposal for §3 if one is needed, with the case against your own preference.
- What F-5's new leg catches that the existing four do not.
- Anything you found while measuring that is not this unit's, **reported and not fixed**.
