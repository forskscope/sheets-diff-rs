# Handoff 03 — The doc comments the patch falsified

**Governing.** Unit 02's criterion-5 findings N1, N2, N3
**Release.** 2.5.1 — the last unit before the cut
**Sequence.** After unit 02. Doc comments only; no behaviour changes.

## Purpose

PR #28 made two public doc comments false. 2.5.1 must not ship a release whose
own patch falsified its own documentation — in this project least of all.

## Background

Unit 02's criterion-5 search found these. They are **caused by the streaming
read**, not pre-existing.

### N1 — `cells_read` describes something the code no longer does

`src/model.rs`, public rustdoc on `DiffMetrics::cells_read`:

> Every cell **physically visited** while reading both workbooks, **including
> empty cells inside the used range** … On the `sparse_range` corpus fixture …
> this reads **5200** against `cells_compared`'s **2**: almost the entire count
> is empty cells the used range spans but never populates.

Verified against the streamed reader: it `continue`s on `DataRef::Empty` before
the bounding box is touched, and the figure is computed as
`(r1 - r0 + 1) * (c1 - c0 + 1)` — **arithmetic from the box corners.**

So no empty position is visited, nothing is "physically visited" that the phrase
describes, and "almost the entire count is empty cells" describes a count that is
no longer arrived at by counting. **The value is unchanged** — 5,200 is still
5,200, a test pins it, and nothing failed. Only the explanation is false.

This comment is M6 unit 04's, which the architect reviewed and approved with the
observation that its concrete contrast "is what makes it land". It still lands;
it is just no longer true.

### N2 — a latency guarantee derived from code that no longer exists

`src/options.rs` (`Cancellation`, "Cancellation latency") and `src/diff.rs`
(`CANCEL_POLL_INTERVAL`) both state the poll happens **"every 50,000 cells"** and
bound worst-case latency to **"roughly 100 ms"**, derived from a per-cell rate
measured on the **dense** read.

After PR #28 the read poll counts **streamed cell records, blank or not**, across
two passes that share one counter. The unit is no longer "cells" in the sense the
comment means, and the ~95 ms figure has never been measured on the streamed
reader.

**This is very likely better than 100 ms now**, not worse — there is no dense
allocation to wait for. That is not the point: it is an unmeasured number
presented as a derived guarantee.

### N3 — "No streaming", now ambiguous

`docs/src/api-guide.md` says **"No streaming — a workbook larger than available
memory cannot be compared."** Still true of the *input bytes*, and now sitting
beside release notes saying sheets are read by streaming.

## Change scope

`src/model.rs`, `src/options.rs`, `src/diff.rs`, `docs/src/api-guide.md`,
`docs/src/maintainers/performance.md` (N2's matching figure only), `CHANGELOG.md`.

## Non-change scope

- **Doc comments and prose only. No behaviour change anywhere.** If a correction
  seems to need a code change, stop and report.
- **Do not change what `cells_read` counts.** Its meaning is M8 unit 05's
  decision, scheduled for 2.6.0. This unit makes the *description* true of the
  current code, nothing more.
- Do not re-measure the cancellation interval. That is M9's, and guessing is
  worse than stating the provenance.
- The fixture corpus must not move.

## Required implementation

1. **Rewrite `cells_read`'s doc to describe what the code does**: the area of the
   bounding box of the populated cells, computed from the box corners as cells
   stream, accumulated across sheets and sides. Keep the `sparse_range` contrast
   — 5,200 against 2 — because the number is what makes the gap visible; fix the
   explanation of where it comes from.

   **Do not write "cells read".** That phrase is what makes it misreadable, and
   the name itself is M8 unit 05's problem, not yours.

2. **State N2's provenance rather than inventing a number.** The poll counts
   streamed cell records; the ~100 ms figure was derived on the dense read and
   has not been re-measured since. Say both. Apply the same to
   `performance.md`'s matching "≈ 95 ms".

3. **Disambiguate N3** — "the input is not streamed", or equivalent. One clause.

4. **CHANGELOG** under `### Documentation`, in the existing `[Unreleased]`
   section, noting that PR #28 falsified these.

## Required tests

None — no behaviour changes. But:

1. **`cargo doc --all-features --no-deps` produces no new warnings.**
2. **Corpus byte-identical**, checked rather than inferred.
3. **Doctests green on stable and 1.88**, since `api-guide.md` is in the harness.

## Acceptance criteria

1. `cells_read`'s doc describes the bounding-box computation, keeps the
   `sparse_range` contrast, and does not say "cells read".
2. The `Cancellation` and `CANCEL_POLL_INTERVAL` docs state that the poll counts
   streamed records and that the latency figure is pre-streaming and unmeasured
   since; `performance.md`'s matching figure says the same.
3. `api-guide.md`'s "No streaming" is unambiguous.
4. No behaviour change; corpus byte-identical; `cargo doc` clean.
5. CHANGELOG under `### Documentation` in the existing `[Unreleased]`.
6. Gates green, full matrix, including MSRV doctests.

## Prohibited shortcuts

- **Do not delete the `sparse_range` contrast.** A reader who cannot see 5,200
  against 2 will not understand why the field needs a paragraph at all.
- Do not soften N2 into "approximately 100 ms". The problem is not the
  approximation; it is that the derivation is gone.
- Do not fix N1 by renaming the field. That is M8 unit 05 and it moves goldens.

## Known risks

- These are the last changes before the cut. **If you find a third falsified
  comment, report it rather than folding it in silently** — the release note
  should say how many there were.

## Required evidence

- The diff
- `cargo doc` output
- Corpus byte-comparison
- CI run link

## Review request format

Per development policy §9.2.
