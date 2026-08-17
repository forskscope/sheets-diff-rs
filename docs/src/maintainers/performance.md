# Performance: measured, not inferred

M7 Handoff 01, updated by Handoff 03. Four questions about memory and
cancellation latency had been reasoned about from reading the code, never
measured. This page reports what measuring them actually found — including
two places the reasoning was wrong, and one place a measurement attempt
itself had to be discarded and redone once the numbers didn't make sense.

**Handoff 01 changed no library code to produce this report** — `src/` was
untouched; finding a defect while measuring was a finding for a later unit,
not something to fix in that unit, and Q4 found exactly such a defect (see
below). **Handoff 03 is that later unit**: it changed `src/diff.rs` to close
the gap Q4 found, and this page's Q4 section and Candidates table are
updated accordingly, with the original (pre-fix) findings kept rather than
silently overwritten.

---

## Method

A `#[global_allocator]` wrapper (`benches/memory.rs`) around `std::alloc::System`
that tracks live bytes via atomics and records a high-water mark. `#![forbid(unsafe_code)]`
is a `src/lib.rs` attribute and does not reach `benches/`, which is why the
wrapper's `unsafe impl GlobalAlloc` lives there. No dependency, no external
profiler, no platform-specific code.

**Accuracy, demonstrated before trusting anything else it reports:** allocate
exactly 8 MiB, read the tracked peak. Observed **8,388,608 bytes** — an exact
match, not an approximation.

**What this measures, and what it does not:**

- Allocator-tracked bytes, not RSS. Excludes the allocator's own bookkeeping
  overhead, memory-mapped regions, and stack. A process's actual resident
  memory will be somewhat higher than every number below.
- `[profile.release]` in this crate's `Cargo.toml` is `opt-level = "z"`
  (size-optimized), the profile `cargo bench` uses. All numbers below are
  from that profile; a speed-optimized build's allocation *pattern* should
  be the same (same code paths, same data structures) but this was not
  independently confirmed under `opt-level = 3`.
- Every measurement resets the tracked peak to the current live-byte count
  immediately before the operation being measured, so prior state (already-held
  input buffers, the harness's own bookkeeping) is excluded from that
  operation's *own* reported peak — except where a measurement is
  deliberately constructed to include a preceding step (Q1's `bytes_peak_total`,
  which starts before the input buffers exist; see below).
- `std::hint::black_box` wraps every comparison result and file path passed
  in, so the optimizer cannot elide the work being measured (the same
  discipline `benches/workbook_diff.rs` already uses).
- Every allocation number below was confirmed reproducible: byte-identical
  across two runs in the same process. Across separate process invocations
  of the identical binary, every ladder point is also byte-identical
  **except the largest (300k cells, 15,000×20)**, which drifts by exactly
  2 bytes between invocations (134,327,699 vs. 134,327,701 internal peak,
  observed directly, not assumed) — 0.0000015% of that point's peak, far
  below the several-percent threshold that would call the method into
  question, and not large enough to move any conclusion below. Likely
  allocator-internal (arena/alignment) nondeterminism that only becomes
  visible at the largest working-set size measured; not investigated
  further since it does not affect any number that matters here. **Wall-clock
  timing (Q4, and Handoff 03's polling-overhead measurement) is not
  reproducible to the byte** — it varied roughly 1.5% run to run, expected
  for a timed operation and reported as a range or an average of repeated
  runs, not a single number.
- The size ladder spans **1,000 to 300,000 cells** (2.5 orders of magnitude).
  Nothing below is extrapolated beyond that range — a real workbook far
  outside it (Excel's own ceiling is roughly 17 billion cells per sheet)
  is not covered, and no number here should be scaled up to guess at one.

---

## Q1 — does `compare_bytes`'s copy actually double peak memory?

**No, not at the level that matters. The raw input bytes genuinely double;
total process peak barely moves, because raw bytes are a small fraction of
what peak actually consists of.**

| Size | cells | `compare_paths` peak | `compare_bytes` total peak* | extra over `compare_paths` | input size as % of `compare_paths` peak |
|---|---:|---:|---:|---:|---:|
| 1k | 1,000 | 458,593 | 760,007 | +65.7% | 4.70% |
| 10k | 10,000 | 4,422,902 | 4,633,890 | +4.77% | 2.62% |
| 100k | 100,000 | 44,544,319 | 45,699,391 | +2.59% | 2.40% |
| 300k | 300,000 | 134,327,797 | 138,948,373 | +3.44% | 2.39% |

\* `compare_bytes` **total** peak is measured from *before* the caller's
input buffers exist, through the call — the number a real `compare_bytes`
caller's process actually reaches, since they must hold that buffer for the
call's duration. (A second number, `compare_bytes`'s *internal* peak —
excluding the caller's buffer, isolating only what happens inside the
call — tracked within 0.1% of `compare_paths`'s own peak at every size,
confirming the two entry points' internal processing is identical, as it
should be: same normalization, same comparison engine, from that point on.)

**The 1k row is the outlier, and it is a small-N artifact, not the trend.**
At 1,000 cells, fixed overhead (allocator bookkeeping, small collection
capacities rounding up) is a larger fraction of a small peak, inflating the
percentage. From 10k cells up, the extra cost of `compare_bytes` over
`compare_paths` settles to **2.6–4.8%** — consistent with the raw input
bytes (2.4–2.6% of peak) roughly doubling, and nothing more.

**What this means for the threat model's "doubling peak memory" claim:**
technically accurate for the raw bytes specifically, and it significantly
overstates the practical impact once total process peak is what's being
asked about — peak is dominated by the ~450 bytes/cell normalized
representation built after the bytes are read (`CellValue`, `NormalizedCell`,
`CellMap` entries), identical regardless of entry point, and that dominates
total memory by roughly two orders of magnitude over the raw XML bytes at
every size measured. Removing the copy would save single-digit percent of
peak at realistic scale, not half of it.

---

## Q2 — where does peak memory actually go?

Two suspects measured; the third could not be isolated, and a fourth,
unplanned finding came out of trying.

### The `cell_map_to_align` clone (`src/diff.rs:42`)

Isolated cleanly: `Positional` (the default `AlignmentMode`) never calls
`cell_map_to_align` at all; any other mode does. Same fixture (a keyed
sheet, matched on a stable id column), two alignment modes:

| Rows | `Positional` peak | `RowKey` peak | delta | delta as % of `Positional` | bytes/row |
|---|---:|---:|---:|---:|---:|
| 500 | 452,782 | 600,886 | 148,104 | 32.7% | 296.21 |
| 5,000 | 4,366,990 | 5,846,058 | 1,479,068 | 33.9% | 295.81 |

**Confirmed linear, not superlinear** — bytes/row is within 0.1% between a
10x change in row count, and the percentage overhead is stable (~33%). This
is a real, moderate, and *avoidable-by-default* cost: it is paid only by
callers who opt into `RowKey`, `RowSignature`, or `HeaderColumn` alignment.
`Positional` callers — the default — never pay it.

*(An earlier attempt at this measurement keyed `RowKey` on the wrong column
— the value column rather than the stable id column — and produced a
23x-inflated, clearly-wrong delta. Caught by checking whether the ratio held
across two row counts before trusting it; the wrong number would have looked
plausible in isolation. Corrected before this table was written.)*

### Both `CellMap`s resident, vs. calamine's own buffers

**Could not be isolated from each other, or — on the first attempt — from a
third, unrelated variable.** The first attempt compared a large sheet
against an empty workbook, expecting to isolate "only one `CellMap` is
large." It didn't: under `Positional` alignment, the compared-coordinate set
is the union of both sides' populated cells, so a large-vs-empty pair makes
*every* populated cell a diff (nothing on the empty side matches). Peak
there is dominated by `diffs_emitted × sizeof(CellDiff)`, not `CellMap`
residency — confirmed empirically, not assumed: the large-vs-empty peak
(212,685,637 bytes, 300,000 diffs) was **higher** than large-vs-large
(134,327,701 bytes, 1 diff), which only makes sense as a diff-output-size
effect. Discarded as a method for this suspect once the direction of the
result made that clear.

The methodologically sound version — both sides large and **identical**
(zero diffs, so diff-output size cannot confound the reading) — gives
134,327,691 bytes (447.8 B/cell) at 15,000×20 dense. That is "two large
`CellMap`s resident, near-zero diff-output cost" as a combined number.
**There is no way to externally construct "only one large `CellMap`
resident" without also changing how many coordinates get compared**, which
changes diff count under `Positional`, as the discarded attempt showed. That
is a structural property of how this engine compares two sides, not a gap in
this measurement's method — reported as a limit rather than forcing a
three-way split that would not hold up.

---

## Q3 — is `BTreeMap` the wrong structure for dense sheets?

Same populated-cell count (20,000), two shapes: fully dense (1,000×20, no
gaps) versus sparse (same 20,000 cells spread across a used range ten times
larger).

| Shape | populated cells | peak | bytes/populated-cell |
|---|---:|---:|---:|
| Dense | 20,000 | 8,831,044 | 441.55 |
| Sparse | 20,000 | 9,922,038 | 496.10 |

**+12.4% per-populated-cell overhead for the sparse shape.** Real and
reproducible, not negligible — but not dramatic either. RFC-024 §7's
proposed `Sparse`/`Dense` split would plausibly recover some of that 12.4%
on sheets shaped like the sparse case here, at the cost of building and
maintaining a second code path. **Recommendation: record as a real,
measured candidate at moderate priority — not "declined" (the costs are not
equal), not urgent (12.4% is not the kind of gap that justifies a variant
data structure on its own).**

---

## Q4 — how long does a caller actually wait after cancelling?

**~565 ms (562–572 ms across three runs) for the largest single-sheet
workbook this ladder covers (300,000 cells, 15,000×20), and — separately, a
more important finding than the number — cancellation is not merely slow
for a single-sheet workbook, it is structurally unobservable.**

`check_cancel` fires once per sheet pair (`src/diff.rs:226`), before that
pair's processing begins. For a workbook with more than one sheet,
requesting cancellation during a large sheet's processing is observed at
the *next* sheet's checkpoint — timed above by building a two-sheet
workbook (one large, one trivial) and arming cancellation to fire on the
second poll: the elapsed time to `Err(Cancelled)` equals how long the first
(large) sheet alone took to process.

**For a workbook with exactly one sheet, there is no second checkpoint.**
Demonstrated, not reasoned about: the same 15,000×20 workbook, single sheet,
same cancel-after-first-poll policy, returns **`Ok`** — the comparison
completes normally regardless of when cancellation was requested, because
nothing ever checks again after the one call at the very start. This is a
different and more clear-cut gap than "polling could be finer" — it is
"polling does not exist at all, for the single most common shape of
workbook" (one sheet is the common case, not the exception).

**Recommendation (Handoff 01, at the time):** RFC-024's status calls the
current granularity a gap against acceptance criteria specifying row chunks
or cell batches; RFC-012's own goal ("cancellation checks at major pipeline
stages") arguably already covers a sheet pair. That disagreement is close to
moot next to the single-sheet finding above, which neither document
anticipated: **pursue finer-grained cancellation, scoped to at minimum
restoring a checkpoint reachable within a single sheet's processing** — not
primarily because ~565 ms is unacceptable on its own (it is closer to the
"non-issue" end of what would justify this), but because zero observability
for single-sheet workbooks is a materially different and more serious
characterization of the gap than either RFC currently states.

### Fixed in Handoff 03: mid-sheet polling, and its measured cost

**The gap above is closed.** `read_sheet_cells`'s row loop (`src/diff.rs:615`)
and `build_sheet_diff`'s coordinate loop (`src/diff.rs:472`) each now poll
`is_cancelled()` on an interval, not just once per sheet pair. The same
15,000×20 single-sheet workbook, same cancel-after-first-poll policy, now
returns **`Err(Cancelled)`** — re-run, not just asserted:

```
single-sheet workbook (15000x20), same cancel-after-first-poll policy:
  result=Cancelled -- observed mid-sheet, via the new interval checkpoint
```

**Interval, derived from a stated target latency, not chosen arbitrarily:**
targeting 100 ms worst-case latency between a cancellation request and the
next checkpoint (the threshold at which a UI action reads as instantaneous),
against Handoff 01's own measured ~1.9 µs/cell budget (300,000 cells in
~567 ms), gives 100,000 / 1.9 ≈ 52,631 cells — rounded down to a plain
number comfortably under that budget: **`CANCEL_POLL_INTERVAL = 50_000`
cells**, ≈ 95 ms worst case at the measured per-cell rate. Applied
identically to both the read-phase loop (all cells in the used range,
including empty ones) and the compare-phase loop (only the coordinates
actually compared).

**Polling overhead — measured with and without a `Cancellation` configured,
across the same ladder, three runs each:**

| Size | cells | `None`, avg | `Some` (configured, never fires), avg | overhead |
|---|---:|---:|---:|---:|
| 1k | 1,000 | 1.77 ms | 1.72 ms | −2.9% |
| 10k | 10,000 | 16.95 ms | 16.68 ms | −1.6% |
| 100k | 100,000 | 182.63 ms | 184.13 ms | +0.8% |
| 300k | 300,000 | 559.53 ms | 558.79 ms | −0.1% |

**Overhead is not measurable above run-to-run noise at any size** — the
sign flips between sizes, and every delta is smaller than the ~1.5% timing
variance already noted above for wall-clock measurements. This satisfies
Handoff 03's own requirement directly: `Cancellation: None` costs nothing
measurable, and — the stronger, unplanned result — neither does a
`Cancellation` that is configured but never fires. The dynamic dispatch
cost of `is_cancelled()` at a 50,000-cell interval is simply too small a
fraction of the per-cell comparison work to separate from noise.

**The two-sheet timing above (~565 ms) is superseded, not just old:**
re-run against the fixed engine, the same two-sheet workbook now returns
`Err(Cancelled)` in **~267 ms** (three runs: 266–271 ms) — because the same
cancel-after-first-poll policy now trips at the *first* mid-sheet checkpoint
inside sheet 1's read phase, not at sheet 2's boundary after sheet 1
finishes entirely. This is expected and correct, not a discrepancy: it is
the fix working, timed the same way Handoff 01 timed the defect.

---

## Candidates for later M7 units

The units this milestone's remaining scope is drawn from — each with its
measured size and this report's confidence in the number.

| Candidate | Measured size | Confidence | Note |
|---|---|---|---|
| Remove `compare_bytes`'s copy | +2.6–4.8% of peak at realistic scale (10k+ cells) | High | Not "the single biggest win available" — Q1 settles this directly against the threat model's framing. |
| Reduce `cell_map_to_align`'s clone cost | +33% of peak, linear, confirmed at two scales | High | Paid only by non-`Positional` alignment modes; `Positional` (default) unaffected. |
| Reduce peak by not holding both `CellMap`s | Not isolable from calamine's own buffers with external measurement | None — no actionable number | Would need instrumentation inside `src/`, out of this unit's scope. Not recommended as a standalone candidate without a different measurement approach. |
| RFC-024 §7's density choice (`Sparse`/`Dense`) | +12.4% per-populated-cell for sparse vs. dense at equal populated count | High | Real, not urgent. Moderate priority. |
| Finer cancellation polling | **Done (M7 Handoff 03).** Was structurally zero for single-sheet workbooks; now polls every 50,000 cells in both phases, ≈95 ms worst case, overhead not measurable above noise. | High | Was the milestone's top priority — "a feature that does not work," not an optimisation. Closed, not deferred further. |
| Shared display address (G) | N/A | N/A | Not a measurement question — a design one, additive on `#[non_exhaustive]` types. Out of this report's scope entirely. |
