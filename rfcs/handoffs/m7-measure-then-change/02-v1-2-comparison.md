# Handoff 02 — The v1.2-vs-v2 comparison

**Governing RFC.** RFC-027 (benchmark and performance governance)
**Roadmap.** M7
**Sequence.** **After unit 03**, which is merged. Gates nothing itself.
*(Corrected 2026-08-17: this originally read "independent of unit 01 and of
everything else", which was true of blocking and wrong about ordering — unit 03
modifies the loops this unit benchmarks. See the amendment at the end.)*

## Purpose

Close RFC-027's named gap — *"No v1.2-vs-v2 benchmark comparison documented"* —
with a comparison that is honest about what it is comparing.

## Background

RFC-027 has carried this gap since 2.0.0. It moved here from M6 because it is
measurement, not documentation.

**A naive version of this unit would produce a misleading number**, and that is
the whole difficulty. Three confounds were known when this was written, and the
amendment at the end adds a fourth that arrived with unit 03. All four are real:

### Confound 1 — the dependency differs

`1.2.0`'s manifest pins `calamine = "0"` and its committed lockfile resolved to
**0.35.0**. 2.4.1 uses **0.36.1**, which pulls a different `quick-xml` and a
different `zip`. **Any measured difference includes calamine's own change**, and
XML parsing is a large fraction of the work in both versions.

A number attributed to "v2's engine" that is partly upstream's is precisely the
kind of claim this project has spent four milestones removing.

### Confound 2 — they do not do the same work

v1.2's `Diff` compares cells as **strings**. v2 normalises into typed
`CellValue`s, runs row alignment, produces diagnostics, tracks metrics, and
distinguishes value from formula changes.

**v2 being slower would not be a defect; it would be arithmetic.** A comparison
that reports "v2 is N× slower" without saying what the extra time buys is worse
than no comparison, because it invites an optimisation effort aimed at removing
capability.

### Confound 3 — v1.2 has no benchmarks

`git ls-tree 1.2.0` shows no `benches/`. The harness has to be built, and it
must drive both versions through equivalent entry points:

- v1.2 — `sheets_diff::core::diff::Diff::try_new(old_path, new_path)`
- v2 — `compare_paths(old, new)`

Both are path-based, which makes the path entry point the fair comparison.
`compare_bytes` has no v1.2 equivalent worth matching.

## Change scope

`benches/`, `Cargo.toml` (a `[[bench]]` entry and possibly a dev-dependency —
see below), a report section in `docs/src/maintainers/performance.md` (created
by unit 01; if unit 01 has not landed, create the file and say so),
`docs/src/SUMMARY.md`, `CHANGELOG.md`.

## Non-change scope

- **Nothing under `src/`.** If the comparison suggests an optimisation, record
  it as a candidate; do not implement it.
- Do not modify anything at the `1.2.0` tag. It is released history.
- Do not present a single aggregate ratio as *the* answer. See below.

## Required implementation

1. **Get v1.2 buildable alongside v2.** The obvious route is a dev-dependency on
   the published `sheets-diff = "1.2"` from crates.io under a rename, so both
   versions coexist in one bench binary. **This is a new dev-dependency and
   needs the usual argument** (RFC-026, `deny.toml`) — including that it pulls
   `calamine` 0.35 into the dev graph alongside 0.36, which `cargo deny`'s
   `multiple-versions` policy may object to. Check before assuming.

   If that route is blocked, say why and propose another. Two separate binaries
   compared by an external script is legitimate and worse; prefer it only if the
   first route genuinely fails.

2. **Control confound 1, or bound it.** Preferred: measure calamine 0.35 versus
   0.36 on the same parsing work directly, so the upstream share can be
   subtracted or at least stated. If that proves impractical, **state the
   confound as unremoved and do not attribute the whole difference to our
   engine.** Either is acceptable; silently attributing it is not.

3. **State what v2 does that v1.2 does not**, concretely, next to the numbers —
   typed normalisation, alignment, diagnostics, metrics. A reader must be able
   to see which part of any gap is capability rather than inefficiency.

4. **Report per-scenario, not one aggregate.** Reuse `benches/workbook_diff.rs`'s
   existing generators. At minimum: a small dense workbook, a tall one, a sparse
   one. A single ratio hides that the two versions may differ in *shape* — if v2
   is faster on sparse and slower on tall, that is the finding, and an average
   destroys it.

5. **Measure peak allocation too, if unit 01 has landed**, reusing its harness.
   The memory comparison is the one ForskScope would care about more than time,
   and it is nearly free once unit 01's harness exists. If unit 01 has not
   landed, time alone is acceptable — say which.

## Required tests

No assertions, and **no performance threshold in CI** — same reasoning as unit
01.

What must be demonstrated:

1. **Both versions produce comparable results on the same input** — not
   identical output (the models differ), but the same count of changed cells on
   a fixture where that is unambiguous. A benchmark of two functions that
   disagree about the answer is not a comparison.
2. **Run-to-run variance**, reported, as in unit 01.

## Acceptance criteria

1. Both versions are driven through path entry points in one harness.
2. Any new dev-dependency is argued against RFC-026 and `deny.toml`, and
   `cargo deny` passes.
3. Confound 1 is either controlled with a measured upstream share, or stated
   plainly as unremoved.
4. Confound 2 is addressed: what v2 does that v1.2 does not is stated beside the
   numbers.
5. Results are per-scenario across at least three workbook shapes, not a single
   ratio.
6. Both versions are shown to agree on changed-cell count for at least one
   fixture.
7. Run-to-run variance is reported.
8. The comparison is documented in `docs/src/maintainers/performance.md`,
   linked from `SUMMARY.md`.
9. Nothing under `src/` changed; corpus byte-identical; no CI threshold added.
10. RFC-027's Status line is updated to record this gap closed — **and only
    this one**; check whether its other clauses still hold before touching it.
11. CHANGELOG under `### Documentation`; gates green, full matrix.
12. **(amendment)** The report names the commit measured, at or after `db88706`.

## Prohibited shortcuts

- **Do not report a single "v2 is N× slower/faster" headline.** If the report
  has one number in it, it is wrong.
- Do not drop a scenario because it is unflattering.
- Do not attribute the calamine 0.35→0.36 difference to our engine.
- Do not "fix" a slow path you find. Record it as a candidate with its measured
  size, like unit 01.
- Do not update RFC-027's whole Status line on the assumption its other clauses
  are stale. M6 found four status lines wrong in exactly that way; check.

## Known risks

- **`cargo deny`'s `multiple-versions` check may reject two calamine versions in
  the dev graph.** `multiple-versions-include-dev` defaults to `false`, so it
  may pass — confirm rather than assume, and if it fails, that is a finding to
  report, not a config to loosen.
- v1.2 pins `calamine = "0"`, so a fresh resolve may pick something newer than
  the 0.35.0 in its lockfile, changing what you are measuring. Pin explicitly
  and say what you pinned.
- v1.2's API may panic where v2 returns `Err` — RFC-005 records that v1
  consumers used `catch_unwind`. Do not let a panic in the v1.2 path be
  misread as a benchmark result.

## Required evidence

- The harness
- The dependency argument and `cargo deny` output
- Per-scenario results, both runs
- The agreement check from required test 1
- The report section
- CI run link

## Review request format

Per development policy §9.2, plus an explicit statement of how each of the
**four** confounds was handled — controlled, bounded, or stated — and the commit
measured (criterion 12).

---

## Amendment — 2026-08-17 (after units 01 and 03 merged)

This handoff was written when M7 opened, before either had landed. Three things
are now settled that it left conditional or did not know.

**1. Unit 01 has landed — its conditionals resolve.**

`docs/src/maintainers/performance.md` **exists**; add a section, do not create
the file. `benches/memory.rs` **exists** with a working peak-allocation harness;
required-implementation item 5's *"if unit 01 has landed"* is now
unconditional — **measure peak allocation as well as time.** The memory
comparison is the one ForskScope would care about more, and the harness is
already there.

**2. Name the commit you measured. This is now an acceptance criterion (12).**

Unit 03 added cancellation polling inside both per-sheet loops — `src/diff.rs`'s
coordinate loop and row loop — which are exactly the hot paths this unit
benchmarks. **"v2" is no longer one thing across this milestone**, and a
comparison that does not say which v2 it measured stops being interpretable the
next time units 04+ touch those loops.

Measure at or after `db88706` (unit 03's merge) and state the commit in the
report section. This is why the sequencing answer put unit 03 first.

**3. A fourth confound, and it is ours.**

The handoff names three. There is now a fourth: **v2's compare loop carries a
polling counter that v1.2's has no equivalent of.** Unit 03 measured its cost as
below run-to-run noise at every ladder size, so it should not distort your
numbers — but you are comparing against a v1.2 that does no such work at all,
and the honest framing of any v2-slower result includes it.

Do not attempt to subtract it. Unit 03's overhead table
(`performance.md`, Q5) is the reference; cite it and move on. Manufacturing a
correction smaller than the measurement noise would be false precision.

**4. On `cargo deny` and the two calamine versions.**

Still the risk the handoff names, and now worth stating more sharply: if
`multiple-versions` rejects calamine 0.35 alongside 0.36 in the dev graph,
**that is a finding to report, not a config to loosen.** `deny.toml` is a
resource-safety control this project built deliberately in M2. Weakening it to
publish a benchmark would be a poor trade, and there are other routes — two
binaries compared by a script is worse engineering and does not touch the
dependency policy.
