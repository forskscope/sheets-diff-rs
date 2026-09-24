# Handoff 02 — The record sweep

**Governing.** Dev-team task 001 findings C1, C2, C3(a), C3(b); RFC-016
**Release.** 2.5.1 — must land before it is cut
**Sequence.** After unit 01 (merged). Last work before the release.

## Purpose

Four documents still describe behaviour that 2.5.0 or 2.5.1 changes. 2.5.1 must
not ship a user-facing page that contradicts its own release notes.

## Why this is a separate unit

It was an amendment to unit 01, added after that unit was already in flight. The
implementer never saw it and submitted correctly against the handoff they had.
**That was the architect's process error**; the work is unchanged, only its
home. Nothing here reflects on unit 01, which is approved and merged.

## Background

Found by dev-team task 001 while reading for an unrelated purpose, plus one the
unit 01 implementer found while measuring.

### C1 — the API guide still claims the doubling we retracted

`docs/src/api-guide.md:82` tells readers **"peak memory is roughly double"** for
`compare_bytes`, and calls it "a real, current cost".

M7 unit 01 measured **2.6–4.8%**. The threat model was corrected, the CHANGELOG
was corrected, and **we wrote to ForskScope that we had overstated it and should
not have said it without measuring.** The page we point users at still says it.

This is the most consequential item here: it is user-facing, it is wrong, and a
reader could restructure their code on the strength of it — which is precisely
what we told ForskScope they might have done.

### C2 — the non-goals page predates two releases

`docs/src/non-goals.md`'s header says "current as of 2.4.x". Its RFC-024 row
(around line 145) says cancellation is "polled once per sheet pair", which M7
unit 03 fixed in 2.5.0.

### C3 — two stale claims in the threat model

Unit 01 added the sheet-reading surface to this file and correctly left these,
as they were outside the handoff it had:

- **(a)** Around lines 189–192: source-path privacy is not "verified by a
  dedicated test". `tests/source_path_privacy.rs` has existed since M5 unit 02.
  **Check this against the test file itself, not against RFC-016's Status
  line** — the readiness review flagged that its own claim rested on the Status
  rather than on reading the test.
- **(b)** Around line 250: `cell_map_to_align` is named as the dominant memory
  cost. That function was deleted in 2.5.0 (M7 unit 04).

### C-new — a comment that overstates its own fixture

`tests/streaming_read.rs` says the dense read allocates "over a gigabyte" and is
"more than a hundred times over the threshold". Unit 01 measured the fixture at
**646.5 MB**, about **9.6×** the 64 MiB budget.

The test still discriminates; the comment is wrong. It originated in my handoff's
framing and propagated. Correct it to the measured figures.

## Change scope

`docs/src/api-guide.md`, `docs/src/non-goals.md`,
`docs/src/maintainers/threat-model.md`, `tests/streaming_read.rs` (the comment
only), `CHANGELOG.md`.

## Non-change scope

- **Nothing under `src/`.** No behaviour changes in this unit at all.
- **Do not touch unit 01's new *Sheet reading* section** beyond what C3 requires
  elsewhere in the file. It is reviewed and correct.
- Do not change any test's logic in `tests/streaming_read.rs`. The comment only.
- The fixture corpus must not move.

## Required implementation

1. **Correct `api-guide.md`'s memory claim** to the measured figure, linking
   `performance.md`. **Do not simply delete the sentence** — a reader who chose
   `compare_paths` on the strength of the old claim is owed the correction, not
   silence.
2. **Update `non-goals.md`**: the version header, and the cancellation row.
3. **Correct the threat model's two stale claims**, with (a) verified against
   `tests/source_path_privacy.rs` directly.
4. **Correct the `tests/streaming_read.rs` comment** to 646.5 MB and ~9.6×.
5. **CHANGELOG** under `### Documentation`, folded into unit 01's existing
   `[Unreleased]` section rather than added as a second block.

## Required tests

None — no behaviour changes. But:

1. **State that the corpus is byte-identical**, checked rather than inferred.
2. **Re-run the doctest harness.** `api-guide.md` and `non-goals.md` are both in
   it (M6 unit 01), so a botched edit to a fenced block fails `cargo test --doc`.

## Acceptance criteria

1. `api-guide.md` states the measured figure, with the old claim corrected
   rather than removed.
2. `non-goals.md`'s version header and cancellation row are current.
3. The threat model's path-privacy and `cell_map_to_align` claims are corrected,
   (a) checked against the test file.
4. `tests/streaming_read.rs`'s comment matches the measurement.
5. **Report anything else in `docs/` that 2.5.0 or 2.5.1 falsifies.** Five items
   were found by two people reading for other reasons; treat that as evidence of
   more rather than as a complete list.
6. Nothing under `src/`; corpus byte-identical; doctests green.
7. CHANGELOG under `### Documentation`, in the existing `[Unreleased]` section.
8. Gates green, full matrix, including MSRV doctests.

## Prohibited shortcuts

- Do not delete a wrong sentence instead of correcting it. Twice now this
  project has had to tell a consumer it overstated something; silence would have
  left them believing it.
- Do not mark criterion 5 complete without having looked. "I found nothing else"
  after a search is a result; not searching is not.

## Known risks

- `docs/src/semantics.md`'s examples **execute** and assert on real output (M6
  unit 03). If an edit touches a fenced block there, CI will say so — which is
  the harness working.
- `performance.md`'s figures pre-date the streaming read (readiness review C7).
  **That is out of scope here** and is M9's; do not start re-measuring.

## Required evidence

- The diff
- Corpus byte-comparison
- Doctest run
- What criterion 5's search covered, and what it found
- CI run link

## Review request format

Per development policy §9.2, plus criterion 5's search and its outcome.
