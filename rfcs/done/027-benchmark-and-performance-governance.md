# RFC-027 — Benchmark and Performance Governance

**Status.** Implemented (2.0.0–2.4.x) — verified 2026-08-16; the deferral closed 2026-08-17 (M7 Handoff 02). `benches/v1_2_comparison.rs` compares v1.2 and v2 through their path-based entry points across three workbook shapes, with the comparison's four confounds (calamine version, differing capability, v1.2 having no benchmarks of its own, v2's cancellation-polling counter) each handled explicitly; documented in `docs/src/maintainers/performance.md`. The other three acceptance criteria (programmatic fixtures, `cargo bench` covering the core scenarios, a >255-column benchmark) were already met and re-confirmed rather than assumed still true.
**Target:** v2.0 guardrail  
**Related:** RFC-011, RFC-012, RFC-024, RFC-025

## 1. Summary

Introduce benchmark scenarios and performance gates so v2 quality improvements
do not accidentally make ordinary workbook comparisons too slow or memory-heavy.

## 2. Motivation

v2 adds typed values, diagnostics, sheet matching, optional formatting, and
potential alignment. These improve correctness and usability but can increase
runtime and memory use. Without benchmark governance, regressions may go
unnoticed until downstream GUI users report hangs.

## 3. Goals

- Establish representative benchmark fixtures.
- Track runtime and memory-oriented proxy metrics.
- Compare v1.2 baseline, v2 positional mode, and v2 quality modes.
- Prevent expensive optional modes from becoming accidental defaults.

## 4. Non-goals

- Hard real-time guarantees.
- Benchmarking every possible workbook shape.
- Requiring all contributors to have identical hardware.

## 5. Benchmark scenarios

Minimum scenarios:

1. **Small business workbook:** 5 sheets, 100 rows, 20 columns.
2. **Wide workbook:** 1 sheet, 100 rows, 1000+ columns.
3. **Tall workbook:** 1 sheet, 50k rows, 20 columns.
4. **Sparse workbook:** large used range with low populated cell density.
5. **Many sheets:** 200 sheets with small content.
6. **Formula workbook:** formulas and cached values.
7. **Rename workbook:** sheets renamed/moved with minimal cell changes.
8. **Alignment workbook:** single inserted row causing cascade in positional mode.

## 6. Metrics

Use Criterion or a similar benchmark tool for runtime. For memory, start with
proxy counters:

```rust
pub struct DiffMetrics {
    pub sheets_read: u32,
    pub cells_read: u64,
    pub cells_compared: u64,
    pub diffs_emitted: u64,
    pub diagnostics_emitted: u64,
}
```

Actual RSS measurement can be added later in platform-specific scripts.

## 7. Performance policy

- v2 default positional mode should remain suitable for interactive GUI use on
  ordinary workbooks.
- Alignment, formatting, object diffing, and similarity matching may be slower
  but must be opt-in.
- Benchmarks should report both runtime and output size.

## 8. CI policy

Regular CI should compile benchmarks but not rely on fragile wall-clock gates.
A scheduled or manual benchmark workflow can compare against baseline artifacts.

## 9. Regression triage

A performance regression is acceptable only when:

- it fixes correctness;
- it affects an optional mode; or
- it is documented with mitigation and future work.

## 10. Acceptance criteria

- Benchmark fixtures are generated programmatically.
- `cargo bench` or equivalent runs the core scenarios.
- Benchmark docs explain how to compare v1.2 and v2.
- At least one benchmark covers the >255-column bug class.
