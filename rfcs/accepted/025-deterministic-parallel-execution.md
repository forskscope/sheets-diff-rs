# RFC-025 — Deterministic Parallel Execution

**Status.** Accepted — design settled; implementation incomplete as of 2.2.3. See ../README.md.
**Target:** v2.x, optional v2.0 if low risk  
**Related:** RFC-012, RFC-014, RFC-024, RFC-027

## 1. Summary

Define how optional parallel processing may be introduced without changing
observable output order, diagnostics order, or reproducibility.

## 2. Motivation

Workbook diffing is naturally parallelizable at sheet level and sometimes row
chunk level. However, nondeterministic ordering is unacceptable for CLI output,
JSON snapshots, golden tests, and GUI updates. Parallelism must be an internal
optimization, not a behavior change.

## 3. Goals

- Allow optional parallel sheet comparison.
- Preserve deterministic result ordering.
- Preserve deterministic diagnostics ordering.
- Make parallelism opt-in or feature-gated until stable.
- Avoid requiring `rayon` or another thread-pool dependency in minimal builds.

## 4. Non-goals

- Parallel workbook parsing if the underlying reader is not thread-safe.
- Exposing thread-pool internals as stable API.
- Making parallelism the default before benchmarks prove benefit.

## 5. Public options

```rust
pub enum ExecutionMode {
    Sequential,
    Parallel,
}

pub struct ExecutionOptions {
    pub mode: ExecutionMode,
    pub max_threads: Option<usize>,
}
```

Default: `Sequential` for v2.0. The crate may switch default later only in a
minor release if output determinism is proven and documented, but conservative
libraries should keep sequential default.

## 6. Internal design

Parallel tasks should produce indexed outputs:

```rust
struct IndexedSheetResult {
    workbook_order: usize,
    sheet_key: SheetPairKey,
    result: Result<SheetDiff, SheetsDiffError>,
    diagnostics: Vec<Diagnostic>,
}
```

Aggregation sorts by `workbook_order` and stable secondary keys before returning
`WorkbookDiff`.

## 7. Diagnostics ordering

Diagnostics must include a stable source location:

```rust
struct DiagnosticLocation {
    sheet_order: Option<usize>,
    sheet_name: Option<String>,
    address: Option<CellAddress>,
    stage: DiffStage,
}
```

The aggregator sorts diagnostics by stage, sheet order, address, and kind.

## 8. Cancellation

Parallel cancellation should use a shared cancellation state. Each task checks
between chunks. Once cancellation is observed, tasks should stop promptly, but
the returned error must be deterministic.

## 9. Feature flags

Parallelism should be gated:

```toml
features = ["parallel"]
```

The feature may enable `rayon` or another dependency. Without the feature,
`ExecutionMode::Parallel` should either be absent or return `UnsupportedOption`.

## 10. Acceptance criteria

- Sequential and parallel runs produce byte-identical JSON for the same input
  and options.
- Golden tests run both modes when the feature is enabled.
- A cancellation test verifies bounded response time for parallel mode.
- Benchmark results justify keeping the feature.

---

## Amendment (2026-08-15)

**Decision.** The implementation is removed in M1 (roadmap decision D2). This
RFC remains **Accepted**: the goal is still sound, the implementation was not.

**What shipped, and why it was wrong.** Code added in 2.2.0 never compiled —
`src/diff.rs` referenced `ExecutionMode::Parallel`, which `src/options.rs` never
defined — yet CHANGELOG 2.2.0 documented the feature as working. Its only test
was gated on the same feature and so never ran.

Independently of that break, the cut was wrong. The implementation pre-read every
sheet **sequentially** (its own comment: "calamine reader is not Sync") and
parallelised only the comparison phase. Comparison is `BTreeMap` lookups and enum
equality over data already in memory; parsing — zip inflation and XML through
`quick-xml` — dominates. Amdahl's law therefore bounds the achievable gain to the
non-dominant phase. Note that §"Acceptance criteria" above already required
"benchmark results justify keeping the feature"; no such measurement was ever
produced, and producing one would have exposed this before any code was written.

**Two further costs specific to a library.** `rayon`'s global pool competes with
the host application's own pool rather than cooperating with it; and parallelism
is a determinism risk in a crate whose stated value is deterministic output. The
removed path already diverged from sequential on limit accounting, progress
events, and metrics.

**The answer for callers today.** `compare_bytes` is a pure function. A consumer
comparing many workbook pairs parallelises across *pairs* — coarser granularity,
near-linear scaling, under its own thread pool. This is strictly better than what
sheet-level parallelism offered and needs no support from this crate. It belongs
in the documentation.

**Candidate design for any future attempt.** Parallelise per-sheet *parsing*
using independent `Xlsx<Cursor<&[u8]>>` readers over the input bytes, which the
crate already owns in full. That attacks the actual bottleneck. It is a different
design, not a repair of the removed one.

**Re-introduction gate.** A measured parse/compare time split on a representative
many-sheet workbook must precede any new implementation. Removal required no
measurement — nothing that has never compiled can regress — but re-introduction
does.
