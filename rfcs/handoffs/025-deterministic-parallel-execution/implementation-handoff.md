# Handoff — Remove the `parallel` feature

**Governing RFC.** [RFC-025](../../accepted/025-deterministic-parallel-execution.md) (Accepted)
**Roadmap.** M1, decision D2
**Sequence.** First. RFC-034's feature matrix cannot go green until this lands.

## Purpose

Remove the non-functional `parallel` feature and the claims made for it, so the
published crate stops advertising something that has never built.

## Background

`src/diff.rs:162` compares `opts.execution.mode` against
`crate::options::ExecutionMode::Parallel`. `src/options.rs:253` defines
`ExecutionMode` with a single variant, `Sequential`, and the comment
`// Parallel added by RFC-025.` in place of the variant. Therefore:

```
cargo check --features parallel   →  error[E0599]: no variant … named `Parallel`
```

This has been true since 2.2.0, which documents the feature as shipped and
gives usage instructions. The only test, `parallel_mode_produces_same_result_as_sequential`,
is gated on the same feature and so has never run.

The design is not being abandoned. RFC-025 stays in `accepted/`. It is the
*implementation* that is wrong: it pre-reads every sheet sequentially and
parallelises only the comparison phase, so it targets the cheap phase while
parsing — the actual bottleneck — remains serial.

## Applicable requirements

NF-010 (default diff stays lightweight), NF-013 (avoid unnecessary cloning),
RFC-026 (dependency governance), RFC-031 (stability policy).

## Change scope

- `Cargo.toml` — remove the `parallel` feature and the optional `rayon` dependency
- `src/diff.rs` — remove the `#[cfg(feature = "parallel")]` branch, the
  `use rayon::prelude::*`, `compare_pre_read_pair`, and the `use_parallel` split;
  the sequential loop becomes the only path
- `src/options.rs` — remove the stale `// Parallel added by RFC-025.` comment;
  `ExecutionMode` keeps `Sequential` as its only variant
- `tests/integration.rs` — remove `parallel_mode_produces_same_result_as_sequential`
- `CHANGELOG.md` — correct the 2.2.0 entry and add the removal under Unreleased
- `README.md` — no change needed; the feature table already omits `parallel`

## Non-change scope

Do **not** touch: `ExecutionOptions`, `ExecutionMode` as a type, or
`DiffOptionsBuilder::execution_mode`. They stay so the API shape survives for a
future re-introduction. Do not alter comparison behaviour, ordering, metrics, or
any other feature flag.

## Required implementation

The deletions above, leaving the sequential path untouched in behaviour.

The rationale — why the cut was wrong, the candidate parse-parallel design, the
caller-side answer, and the re-introduction gate — is **already recorded** in
RFC-025's `## Amendment (2026-08-15)` section. Read it before starting; do not
edit the RFC. RFC authorship stays with the architect role, and a handoff must
never silently redefine its governing RFC.

## Required tests

No new tests. Removing the only (never-executed) test is expected. The
verification is that the full matrix builds — which RFC-034 will enforce
permanently and which must be run manually here.

## Required documentation updates

- CHANGELOG 2.2.0 entry annotated: the RFC-025 bullet describes a feature that
  never compiled. Do not delete the historical entry — annotate it in place.
- Unreleased section records the removal and points at RFC-025 for rationale.

## Acceptance criteria

1. `cargo check --features parallel` fails with "unknown feature" rather than a
   compile error inside the crate — i.e. the feature no longer exists.
2. `cargo test` and `cargo test --features serde,chrono,cli` pass unchanged.
3. `rayon` no longer appears in `cargo tree`.
4. Comparison output for every existing fixture is byte-identical to before.
5. RFC-025 carries the amendment and remains in `accepted/`.

## Prohibited shortcuts

- Do **not** "fix" the feature by adding a `Parallel` variant. That decision was
  taken and recorded as D2; reversing it is a change request, not an
  implementation choice.
- Do not delete the 2.2.0 CHANGELOG entry to hide the incorrect claim.
- Do not remove `ExecutionMode` or `execution_mode()` — that is a public API
  break outside this handoff's scope.

## Compatibility constraints

Removing a feature flag that has never compiled cannot break a working build.
No consumer can be relying on it. Treat as non-breaking; it ships in a minor.

## Security constraints

Removing `rayon` shrinks the dependency tree, which is desirable under RFC-026.
Confirm no other optional feature pulls it back in.

## Known risks

Low. The only risk is over-deletion — removing `ExecutionMode` or the builder
method, which would be a public API break. The non-change scope above is the
guard.

## Required evidence

- `cargo tree` before and after, showing `rayon` gone
- Output of every command in the acceptance criteria
- The diff of the CHANGELOG annotation
- Confirmation that fixture comparison output is unchanged

## Review request format

Per the development policy §9.2: implementation summary, addressed
requirements, changed files, important decisions, differences from this handoff,
tests executed with results, build and static-analysis results, unresolved
issues, known limitations, requested review focus.
