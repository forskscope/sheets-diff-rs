# Handoff 01 — Close the two findings the streaming patch does not

**Governing RFCs.** RFC-012 (cancellation and resource bounds), RFC-035
(resource safety), RFC-016
**Release.** 2.5.1 — must land before it is cut
**Sequence.** After PR #28 (the streaming read) is green. Independent of the
advisory decision, which is the owner's and runs in parallel.

## Purpose

The streaming patch fixes the defect. It does not fix the test that should have
caught it, and it does not update the document that claims the defence it
breaks. Both must land in the same release.

## Background

ForskScope reported a resource defect: `read_sheet_cells` read through
`worksheet_range`, which allocates densely over the bounding box of the
populated cells. A **5.4 KB workbook with two populated cells** could abort the
calling process. PR #28 streams instead, and is verified.

Two things it deliberately left alone.

### F-T — our cancellation test does not test the read phase

`tests/integration.rs::cancellation_observed_during_read_phase` **passes with
the read-phase poll removed.** Verified during review by deleting only that
poll and leaving the formula-pass and compare-phase polls intact:

```
test cancellation_observed_during_read_phase ... ok       ← should have failed
test the_read_loop_polls_for_cancellation_on_its_own ... FAILED   ← ForskScope's, works
```

The comparison phase cancels instead, so the test passes for the wrong reason.

**This is the architect's miss, not the implementer's.** M7 unit 03's review
claimed the two phase tests were what mattered, and verified it by reverting the
whole of `src/diff.rs` — which removed *both* pollers, so the mutation could not
tell which one carried the test. The flaw being checked for was the flaw the
check had.

**The pattern that works**, from ForskScope, and confirmed against the streamed
code: the read loop increments its poll counter on **every streamed record,
blank or not** — `poll_count += 1` happens before the `DataRef::Empty` skip —
while the comparison loop iterates only the union of *populated* coordinates. So
a sheet with many styled-blank records and few populated cells gives the read
loop thousands of poll opportunities and the comparison almost none.

### F-U — the threat model does not list this surface

`docs/src/maintainers/threat-model.md` names as an asset:

> **Availability of the host process.** A workbook must not be able to crash,
> hang, or exhaust memory on the process that calls this crate.

and holds up `max_alignment_product` as the pattern — it caps the LCS matrix
***before* it is allocated**, with the failure mode it prevents named as
*"~10GB, process-aborting"*.

The read path had exactly that failure mode, unbounded, in every published
version, with `max_cells_read` firing *after* the spend — the inverse of the
pattern the same document praises. `worksheet_range`'s dense allocation appears
nowhere in the document.

## Change scope

`tests/integration.rs`, `docs/src/maintainers/threat-model.md`,
`docs/src/api-guide.md`, `docs/src/non-goals.md`, `CHANGELOG.md`.
*(The two `docs/` pages were added by the 2026-09-24 amendment at the end.)*

## Non-change scope

- **Nothing under `src/`.** PR #28 owns the fix; this closes what it left.
- **Do not weaken or delete `cancellation_observed_during_read_phase`.** Rebuild
  it so it discriminates. A deleted test is not a fixed test.
- Do not change `cells_read`'s meaning. That question is real and is scheduled
  separately — see "Explicitly out of scope" below.
- The fixture corpus must not move.

## Required implementation

1. **Rebuild `cancellation_observed_during_read_phase` so it fails when the
   read-phase poll is removed.** Use the blank-record asymmetry above, or
   another mechanism you can defend — the requirement is the discrimination,
   not the technique.
2. **Leave `cancellation_observed_during_compare_phase` able to discriminate
   too.** Check it the same way: remove only the compare-phase poll and confirm
   it fails. If it does not, that is a second instance of the same finding and
   it is in scope.
3. **Record the surface in the threat model**, under the resource-exhaustion
   section beside `max_alignment_product`:
   - what the surface was (`worksheet_range` dense over the bounding box of
     populated cells; a stray far cell sets the box; ~31 bytes per box position);
   - that `max_cells_read` fired *after* the allocation it exists to prevent;
   - that it is fixed as of 2.5.1 by streaming, and the bound now fires before
     the spend, matching `max_alignment_product`'s pattern;
   - **that it was present in 2.0.0–2.5.0.** Do not write the entry as though
     the crate had always been safe here.
4. **CHANGELOG entry** under `### Fixed`, naming both observable changes:
   `LimitExceeded { CellsRead, observed }` now reports the running box area at
   the breaking cell rather than `max + 1`; and the cancellation poll counts
   streamed records rather than dense-range positions, so a sheet with a huge
   box and few cells no longer polls — and no longer takes any time either.

## Required tests

1. **Demonstrate the rebuilt test failing** with only the read-phase poll
   removed, and passing with it restored. Capture both.
2. **Demonstrate the compare-phase test the same way** (item 2).
3. Corpus byte-identical, stated explicitly rather than inferred from a green
   suite.

## Acceptance criteria

1. `cancellation_observed_during_read_phase` fails when only the read-phase poll
   is removed, demonstrated with a transcript.
2. `cancellation_observed_during_compare_phase` fails when only the
   compare-phase poll is removed, demonstrated — or the finding is reported if
   it cannot be made to.
3. Neither test was weakened or deleted to achieve this.
4. The threat model records the surface, that the bound fired after the spend,
   the fix, and the affected version range.
5. CHANGELOG names both observable changes.
6. Nothing under `src/` changed.
7. Corpus byte-identical.
8. Gates green: fmt, clippy `-D warnings`, the scoped stdout gate, `deny`,
   MSRV 1.88, doctests at 1.88, full matrix.

## Prohibited shortcuts

- **Do not verify by reverting a whole file.** That is precisely the mistake
  that let F-T through. Remove the specific code under test.
- Do not describe the threat-model surface in the past tense only. A reader
  needs to know which published versions are affected.
- Do not soften "aborted the calling process" into "used significant memory".

## Known risks

- A blank-record fixture may need XML patching; `tests/support.rs` has
  `patch_xlsx_xml` and `examples/gen-fixtures.rs` has its own copy. Prefer a
  test-local construction over a new corpus scenario — this is a cancellation
  test, not a comparison scenario, and the corpus must not move.
- Removing a poll to check discrimination leaves the tree dirty. Restore from a
  copy, and confirm `git diff` is empty before moving on.

## Explicitly out of scope, and scheduled separately

ForskScope's §4 raised, and did not answer, whether `cells_read` should count
**populated cells** rather than the bounding-box area. It is a real question —
`cells_read` reports 5,200 against 2 compared cells on `sparse_range`, which is
not what the name suggests, and M6 unit 04 had to document the gap rather than
close it.

It changes a public metric and moves every golden, so it cannot ride a patch
release fixing a denial of service. It is recorded for decision after 2.5.1.

---

## Amendment — 2026-09-24 — **WITHDRAWN, moved to unit 02**

This amendment was added *after* work on unit 01 had already started, against a
handoff the implementer had already read at `8fe7c2c`. They never saw it, and
submitted correctly against the version they had.

**Amending a handoff that is in flight is a process error and it was mine.** The
content — the record sweep across `api-guide.md`, `non-goals.md` and two stale
threat-model claims — is now
[unit 02](./02-the-record-sweep.md), unchanged in substance.

Criteria 1–8 of this unit stand as originally written and were all met.
