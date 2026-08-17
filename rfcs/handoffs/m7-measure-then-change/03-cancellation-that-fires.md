# Handoff 03 — Cancellation that fires

**Governing RFCs.** RFC-012 (progress, cancellation and resource bounds),
RFC-024 (large-workbook memory — its Status carries this as a deferral)
**Roadmap.** M7
**Sequence.** After unit 01 (uses its harness). Ahead of the memory candidates:
those are optimisations, this is a feature that does not work.

## Purpose

Make cancellation observable during a single sheet, and choose the polling
granularity by measurement rather than taste.

## Background

Unit 01 measured what four releases had reasoned about:

```
single-sheet workbook (15000x20), cancel-after-first-poll:
  result=Ok -- cancellation never observed
```

`check_cancel` is called **once in the entire library**, at `src/diff.rs:226`,
at the top of the per-sheet loop. For a one-sheet workbook it fires before any
work begins and never again. A caller who cancels *in response to the comparison
taking time* — the only realistic case — is never observed, and the comparison
runs to completion and returns `Ok`.

**One sheet is the ordinary shape of a spreadsheet**, and RFC-012 exists because
*"GUI applications need to keep the UI responsive and allow users to cancel."*

### The record understates it, in a way worth fixing carefully

`Cancellation`'s own doc comment (`src/options.rs:333`) says:

> `is_cancelled()` is polled **once before each sheet pair** is processed. On a
> workbook with many sheets, cancellation is observed promptly. On a single very
> large sheet, cancellation is **not** observed mid-sheet in the current
> implementation — it fires before the next sheet begins.

Every clause is true, and the whole is misleading. *"It fires before the next
sheet begins"* presumes a next sheet. **For a one-sheet workbook there is no
next sheet and it never fires at all** — the reader is told about latency and
would not conclude "never".

RFC-024's Status has the same shape: *"cancellation is polled once per sheet
pair, not between row chunks or cell batches"* reads as granularity. That
framing is why this sat as a deferred nicety across four releases.

The suggested mitigation in that doc comment — *"also set a `max_cells_read` /
`max_cells_compared` bound"* — is not a substitute. It ends the comparison with
`LimitExceeded` whether or not the user asked for anything.

## Change scope

`src/diff.rs`, `src/options.rs` (the `Cancellation` doc comment),
`tests/integration.rs`, `benches/memory.rs` (the overhead measurement),
`docs/src/maintainers/performance.md`, `rfcs/done/024-large-workbook-memory-strategy.md`
(its Status line), `CHANGELOG.md`.

## Non-change scope

- **Do not change comparison results.** The only observable difference is that a
  cancelled comparison now returns `Err(Cancelled)` where it previously returned
  `Ok`. The fixture corpus must not move; if a golden moves, stop and report.
- Do not change the `Cancellation` trait's shape. It is public API and
  `is_cancelled(&self) -> bool` is sufficient.
- Do not add a progress-reporting mechanism. RFC-012 mentions progress events;
  that is not this unit.
- Do not touch the limit checks.

## Required implementation

1. **Add checkpoints inside the per-sheet work.** Both loops already receive
   `opts`, so neither needs a signature change:
   - `read_sheet_cells`'s row loop (`src/diff.rs:595`)
   - `build_sheet_diff`'s coordinate loop (`src/diff.rs:472`)

   Both matter. Reading a large sheet and comparing it are separate phases and a
   caller can cancel during either.

2. **Poll on an interval, not every iteration**, and **choose the interval by
   measurement.** `is_cancelled()` is a dynamic trait call; per-cell polling on
   a 300,000-cell workbook is 300,000 virtual calls.

   Unit 01 gives you the budget: 300k cells took ~567 ms, so roughly 1.9 µs per
   cell. Pick an interval that bounds worst-case latency to something a GUI user
   experiences as immediate — **state the target latency you are designing for
   and derive the interval from it**, rather than picking a round number.

3. **Measure the overhead of the polling you added**, using unit 01's harness
   and its ladder. A cancellation fix that costs 10% of comparison time is a
   different proposal from one that costs 0.1%, and this milestone's whole
   discipline is not guessing which. Report time cost with cancellation
   configured and with it absent (`None`), since the `None` path should be
   nearly free.

4. **Correct `Cancellation`'s doc comment.** It must say plainly that
   cancellation is observed during a sheet, at what approximate granularity, and
   it must drop the `max_cells_read` workaround advice now that it is
   unnecessary. **Do not simply delete the old limitation text** — a reader
   upgrading needs to know the behaviour changed.

5. **Update RFC-024's Status line** for this deferral only. **Check its other
   clauses before touching it** — M6 found four status lines stale in exactly
   the way that comes from editing one clause and assuming the rest.

## Required tests

1. **A single-sheet workbook observes cancellation.** This is the unit. A
   `Cancellation` that reports cancelled from the first poll onward, against a
   one-sheet workbook large enough to cross at least one interval, must yield
   `Err(SheetsDiffError::Cancelled)`.

   **Demonstrate it fails before the fix** — revert `src/diff.rs`, run the test,
   capture the `Ok`, restore. Same standard as M4 unit 02 and M5 units 01/04.

2. **Cancellation is observed during the read phase as well as the compare
   phase.** If only one loop is instrumented the test above may still pass;
   cover both, or explain why one is unreachable.

3. **A comparison that is never cancelled still succeeds and returns identical
   results.** The corpus is the strongest form of this — it must not move.

4. **`Cancellation: None` costs nothing measurable.** Item 3's overhead numbers
   satisfy this; state it explicitly.

## Acceptance criteria

1. Cancellation is observed during a single sheet's processing, in both the read
   and compare phases.
2. A test proves it, and is demonstrated failing before the fix.
3. The polling interval is derived from a stated target latency, not chosen
   arbitrarily.
4. Polling overhead is measured on unit 01's ladder, with and without a
   `Cancellation` configured, and reported in `performance.md`.
5. Comparison results are unchanged; fixture corpus byte-identical.
6. `Cancellation`'s doc comment describes the new behaviour and records that it
   changed.
7. RFC-024's Status line is corrected for this deferral, with its other clauses
   checked.
8. CHANGELOG records this under `### Fixed`, and states plainly that a
   comparison which previously ran to completion despite a cancellation request
   will now return `Err(Cancelled)`.
9. Gates green, full matrix, including the scoped stdout gate and MSRV doctests.
10. No new dependency; no CI performance threshold.

## Prohibited shortcuts

- **Do not poll every iteration to keep it simple.** Measure and choose.
- Do not implement the checkpoint by consulting a clock. Time-based polling
  makes behaviour depend on machine speed and makes the test flaky.
- **Do not weaken the test to a multi-sheet workbook.** Multi-sheet already
  worked; the single-sheet case is the defect.
- Do not silently change what a cancelled comparison returns beyond
  `Err(Cancelled)` — no partial results.
- Do not delete the doc comment's old limitation without replacing it with what
  is now true.

## Compatibility constraints

**A comparison that previously completed and returned `Ok` despite a
cancellation request will now return `Err(Cancelled)`.** That is the fix, and it
is still an observable change for any caller that set a `Cancellation` and
relied — knowingly or not — on it being ignored mid-sheet.

There is no reasonable code depending on that, but it belongs in the CHANGELOG
as a stated consequence rather than being left for someone to discover. This is
a bug fix, not a compatibility event: it makes a documented feature work as
documented.

## Known risks

- **The `None` path must stay free.** `opts.execution.cancellation` is an
  `Option`; the interval check should short-circuit when it is `None`, and the
  measurement in item 3 is what proves it.
- The coordinate loop already has two `return Err` limit checks. Adding a third
  early exit is consistent, but confirm no accumulator is left in a state a
  later change would misread — unit 02 of M4 established that
  `total_cells_compared` is added before the loop, so an early exit over-counts
  harmlessly *because* the error propagates. Do not disturb that.
- A test that cancels from the first poll may abort before any interval is
  crossed, passing for the wrong reason. Make the workbook large enough that the
  fix is what makes it pass.

## Required evidence

- The diff
- The pre-fix failure transcript
- The interval derivation, with its target latency
- Overhead numbers, with and without `Cancellation`, on unit 01's ladder
- Corpus unchanged
- CI run link

## Review request format

Per development policy §9.2, plus the interval derivation and the overhead
table.
