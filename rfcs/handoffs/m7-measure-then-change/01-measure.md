# Handoff 01 — Measure

**Governing RFCs.** RFC-024 (large-workbook memory), RFC-012 (progress,
cancellation and resource bounds)
**Roadmap.** M7 — gates every later unit
**Sequence.** First. Unit 02 is independent; units 03+ cannot be written until
this reports.

## Purpose

Produce the numbers M7's scope depends on. **This unit changes no library code
and optimises nothing.** Its deliverable is a report.

## Background

Four questions are open, and each has been reasoned about without being
measured.

### Q1 — how much does `compare_bytes`'s copy actually cost?

`src/open.rs:102` — `open_bytes_inner(bytes.to_vec(), side, source)`. The caller
already owns the bytes; we take a second copy because the reader is
`Xlsx<Cursor<Vec<u8>>>` and needs an owned buffer.

`docs/src/maintainers/threat-model.md` records this as *"doubling peak memory
relative to a hypothetical borrowing implementation"*, and ForskScope confirmed
their adapter passes bytes, so it is a real cost for the only known consumer.

**"Doubling" is an inference from reading the code, not a measurement.** If the
copy is 8 MB against a 300 MB peak it is noise; if it is 8 MB against a 20 MB
peak it is the single biggest win available. Nobody knows which.

### Q2 — where does peak memory actually go?

Three suspects, never attributed:

- **Both `CellMap`s resident at once.** Old and new sheets are both fully
  normalised before comparison.
- **`cell_map_to_align` clones every `CellValue`** — `src/diff.rs:42`:
  `cells.iter().map(|(k, v)| (*k, v.value.clone())).collect()`. A whole second
  copy of every value in both sheets, built only to feed alignment.
- **Calamine's own buffers**, which we do not control.

Without attribution, any change is a guess. The clone at `diff.rs:42` looks like
the obvious target precisely because it is easy to see — which is a reason for
suspicion, not confidence.

### Q3 — is the `BTreeMap` the wrong structure for dense sheets?

RFC-024 §7 proposes choosing per sheet:

```rust
enum SheetCells {
    Sparse(BTreeMap<Coord, NormalizedCell>),
    Dense(Vec<RowCells>),
}
```

Never built. `BTreeMap` is used unconditionally. On a dense sheet that is one
node allocation per cell where a row vector would be one per row.

### Q4 — how long does a caller actually wait after cancelling?

Cancellation is polled once per sheet pair (`check_cancel` at `src/diff.rs:226`).
**Two documents disagree about whether this is a defect**: RFC-024's Status
calls it a gap against acceptance criteria specifying row chunks or cell
batches; RFC-012's own goals say *"provide cancellation checks at major pipeline
stages"*, and a sheet pair is arguably one.

**That disagreement cannot be settled by reading either document.** It is
settled by how long a GUI user stares at an unresponsive cancel button. If the
largest realistic single sheet takes 150 ms, the deferral should be closed as a
non-issue. If it takes 20 s, finer polling is a real requirement.

## Change scope

`benches/` (a new bench target), `Cargo.toml` (a `[[bench]]` entry only), and a
report at `docs/src/maintainers/performance.md` plus its `SUMMARY.md` link.
`CHANGELOG.md`.

## Non-change scope

- **Nothing under `src/`.** Not one line. If measuring reveals a defect, **stop
  and report** — fixing it is a later unit's job, scoped by this report.
- **No optimisation, not even an obvious one.** If you find a one-line
  improvement, record it in the report as a candidate with its measured size.
- Do not add a dependency. The mechanism below needs none.
- Do not change the existing `workbook_diff` bench. It measures time and is
  fine; this is a separate target.

## Required implementation

1. **A peak-allocation harness.** A `#[global_allocator]` wrapper over `System`
   that tracks live bytes and records a high-water mark. Verified working in
   this repository before this handoff was written: a bench target can do this,
   because `#![forbid(unsafe_code)]` applies to the library crate and not to
   `benches/`, and an 8 MiB allocation is reported as exactly 8388608 bytes.

   No dependency, no external profiler, no platform-specific code.

2. **Measure peak across a size ladder**, for both `compare_paths` and
   `compare_bytes`, on workbooks spanning at least two orders of magnitude of
   cell count. Reuse `benches/workbook_diff.rs`'s existing generators
   (`make_workbook`, `make_sparse`) rather than inventing fixtures.

   Report peak bytes **and peak bytes per cell** — the second is what shows
   whether cost is linear, and it is the number a later unit will need.

3. **Attribute the peak (Q2).** Getting three separate numbers from one process
   is the hard part of this unit, and how you do it is your decision to make and
   justify. Options include sampling the high-water mark at known points in a
   comparison, or measuring a workbook shaped to isolate one suspect. **State
   the method and what it cannot separate.** A number without a method is not a
   measurement.

4. **Q1 falls out of item 2** — `compare_bytes` peak minus `compare_paths` peak
   at the same size, plus the input size, tells you whether "doubling" is right.
   Say so explicitly, with the ratio, at each size.

5. **Q3: measure `BTreeMap` overhead on a dense sheet.** You do not need to
   build `SheetCells` to answer this — measure bytes per cell on a dense sheet
   against a sparse one of the same populated-cell count. If per-cell cost is
   the same, RFC-024 §7 is not worth building and should be recorded as
   declined rather than deferred.

6. **Q4: measure cancellation latency.** Time from a cancellation request to
   `Err(Cancelled)` on the largest single-sheet workbook the ladder covers.
   `Cancellation` is a trait — a test implementation that flips after a delay,
   or that reports cancelled on first poll while timing the surrounding call,
   is enough. Report the worst case in milliseconds.

7. **Write the report** at `docs/src/maintainers/performance.md`: method, the
   numbers, what the method does not capture, and a **candidates section**
   listing each possible change with its measured size and your confidence.
   That section is what units 03+ will be scoped from.

## Required tests

This unit produces measurements, not assertions, and **must not add a
performance assertion to CI** — a threshold that fails on a noisy runner is
worse than no check.

What must be demonstrated:

1. **The harness is correct.** Show it reporting a known allocation accurately
   (allocate N bytes, observe N). A memory measurement nobody sanity-checked is
   the same failure this project has spent three milestones removing.
2. **The numbers are reproducible.** Run the ladder at least twice and report
   the variance. If peak differs run to run by more than a few percent, say so —
   that is a result about the method.

## Acceptance criteria

1. A bench target measures peak allocation with no new dependency.
2. The harness is demonstrated accurate against a known allocation.
3. Peak is reported across a size ladder spanning ≥2 orders of magnitude, for
   both `compare_paths` and `compare_bytes`, as total and per-cell.
4. Q1 is answered with a ratio at each size — is it doubling, or not?
5. Peak is attributed, with the method stated and its limits named.
6. Q3 is answered: dense versus sparse per-cell cost, and a recommendation to
   build or decline RFC-024 §7.
7. Q4 is answered in milliseconds, with a recommendation to close or pursue
   RFC-024's cancellation deferral.
8. Run-to-run variance is reported.
9. `docs/src/maintainers/performance.md` exists, linked from `SUMMARY.md`, with
   a candidates section giving each change a measured size and a confidence.
10. **Nothing under `src/` changed**; corpus byte-identical; no performance
    threshold added to CI; gates green, full matrix.

## Prohibited shortcuts

- **Do not optimise anything.** Not one line, however obvious. The report is the
  deliverable and a change would compromise the numbers it rests on.
- Do not report a peak without saying what the allocator sees and what it does
  not. Allocator-tracked bytes are not RSS; they exclude the allocator's own
  overhead, memory-mapped regions and stack.
- Do not extrapolate. If the ladder tops out at 500k cells, say so; do not
  project to 5M.
- Do not answer Q4 by reasoning about the code. Time it.
- Do not present a single run as a measurement.

## Known risks

- **`black_box` matters.** An unused comparison result can be optimised away and
  a peak of nothing measured. `criterion::black_box` is already used in this
  repository's benches (`benches/workbook_diff.rs`); the same discipline
  applies.
- Peak is a high-water mark across the whole process, so harness allocations
  count too. Reset or baseline between measurements and say which.
- The generators build workbooks in memory, so generation allocates before
  measurement begins. Bench only the comparison.
- Q2's attribution is genuinely hard and may not fully separate the three
  suspects. **A partial answer with its limits stated is a good result**; a
  clean-looking answer that overstates what the method can distinguish is not.

## Required evidence

- The harness, and its accuracy demonstration
- The full ladder, both entry points, both runs
- The attribution method and its output
- Q4's timing
- The report
- CI run link

## Review request format

Per development policy §9.2, plus: the four questions answered in order, each
with the number and the method; and the candidates section reproduced, since it
is the input to M7's remaining scope.
