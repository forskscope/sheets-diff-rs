# Handoff 08 — The options tree becomes extensible

**Governing.** [RFC-037 §3.9](../../accepted/037-v3-scope.md); RFC-006
(diff options and the configuration builder); RFC-033 §11
**Roadmap.** M10 — unit 08
**Sequence.** **Last of the code units.** After 06 and 07, because it must see
the final set of option fields. Before the migration guide.

## Purpose

Every option this crate adds is a breaking change. Every result field it adds is
additive. Make the options behave like the results.

## Background

**All eight public model structs are `#[non_exhaustive]`:** `WorkbookDiff`,
`SheetDiff`, `CellDiff`, `DiffSummary`, `DiffMetrics`, `Diagnostic`,
`DiagnosticLocation`, `SheetRef`.

**None of the eight public options structs is:** `DiffOptions`,
`ComparisonOptions`, `ValueCompareOptions`, `MatchingOptions`, `Limits`,
`ExecutionOptions`, `DiagnosticOptions`, `OutputOptions`. Every field is `pub`,
so a downstream caller may write:

```rust
ComparisonOptions { value, formula, include_formula_cached_values }
```

and any new field breaks that code.

**This is not a decision anyone made.** The results got it right and the options
were overlooked. It is invisible in the API surface and it can only be fixed at
a major — and left alone it taxes every future release. RFC-011's column
alignment, RFC-022's formatting, a formula normaliser: each one adds an option,
and each would otherwise wait for 4.0.0 or ship as a break.

**It is also what makes unit 06's removals recoverable.** Reintroducing
`ComparisonOptions::format` when RFC-022 is implemented is additive only after
this lands.

## Change scope

- `src/options.rs` — the eight structs
- **`tests/builder_coverage.rs` → `src/`** — see Required implementation 2
- `rfcs/done/006-*.md`, `rfcs/done/033-*.md` §11
- `docs/src/api-guide.md`, `docs/src/migration/` — construction guidance
- `CHANGELOG.md`

## Non-change scope

- **Do not make any field private.** `#[non_exhaustive]` stops *construction by
  literal* from outside the crate; reading and assigning fields must keep
  working, and the equivalence tests from §3.7 depend on it.
- Do not add, remove or rename an option. Units 06 and 07 settled the set.
- Do not change `Default` for any struct.
- Do not apply `#[non_exhaustive]` to the option **enums**. They already carry
  it where it belongs, and this unit is about the structs.

## Required implementation

1. **`#[non_exhaustive]` on all eight options structs.**
2. **Move the builder-coverage guard into `src/`.** `tests/builder_coverage.rs`
   destructures these structs exhaustively, with no `..`, so that adding a field
   fails to compile until it is listed — verified at review, it produces
   `error[E0027]`. **It is a separate crate, so `#[non_exhaustive]` breaks it.**

   Move it to a `#[cfg(test)] mod` inside `src/options.rs` (or a `src/` module
   it includes), where `#[non_exhaustive]` does not apply. **The exhaustive
   destructure must survive the move** — it is the guard, not the wrapper around
   it. Verify by adding a probe field and seeing `E0027`, then removing it.

   Note this is the one place the project's rule against inline `#[cfg(test)]`
   modules (readiness finding B2) is knowingly ignored, because the guard only
   works from inside the crate. Say so in the module comment.
3. **`Default` and the builder become the documented way to construct.** The
   builder covers every leaf option (§3.7), so the recommended path is complete.
   `..Default::default()` remains available for callers who prefer literals.
4. **RFC-006 and RFC-033 §11** record that the options tree is extensible and
   that new options are additive from 3.0.0 on. **That sentence is the whole
   value of this unit** — write it where a future author planning an option will
   find it.
5. **The migration guide gets its entry** (the guide itself is the next unit):
   struct-literal construction of options → builder or `..Default::default()`,
   with a before/after.

## Required tests

- **The guard, moved and still biting**: add a probe field to each of the eight
  structs in turn and show `E0027`. Doing it for one is not enough — the point
  is that all eight are covered.
- **A downstream caller can still read and assign fields**: an integration test
  (separate crate) that sets options by field assignment and reads them back.
  This proves `#[non_exhaustive]` did not over-restrict.
- **An integration test constructs options with `..Default::default()`** and
  gets the same result as the builder. That is the migration path, tested.

## Acceptance criteria

1. All eight structs are `#[non_exhaustive]`.
2. The coverage guard lives in `src/`, still destructures exhaustively, and
   **still produces `E0027`** — demonstrated for all eight structs.
3. Field read and assignment from another crate still work, tested.
4. `..Default::default()` construction works and equals the builder, tested.
5. **No option added, removed or renamed**; the leaf count matches unit 06's
   final set — state the number.
6. RFC-006 and RFC-033 §11 say new options are additive from 3.0.0.
7. Corpus byte-identical.
8. CHANGELOG `### Changed`, with the struct-literal migration and — this is the
   part worth writing well — **what the crate gains**: every option after this
   is additive.
9. Gates green, including `--all-targets` (benches construct options).

## Prohibited shortcuts

- **Do not delete the coverage guard because the move is awkward.** It is the
  best structural test this milestone produced; it catches the next option added
  without a setter, forever, and moving it is the price of the fix.
- Do not weaken it to a non-exhaustive destructure with `..` so it compiles from
  `tests/`. That silently removes the guarantee and leaves a test that looks
  like a guard.
- Do not add `#[doc(hidden)]` fields or a private marker field as a substitute.
  `#[non_exhaustive]` is the supported mechanism and it says what it means.

## Compatibility constraints

**Breaking for anyone constructing options by struct literal**, which is the
entire point, and the last time it will be.

State the trade plainly in the CHANGELOG: one break now, in exchange for every
future option being additive. A reader should understand they are paying once.

## Known risks

- **`benches/` construct options** and `--all-targets` compiles them; they are
  in-crate for `benches`? **Check** — a bench is a separate crate, so
  `#[non_exhaustive]` applies to it. Unit 03 found two bench call sites a
  `tests/`-only search missed.
- `examples/gen-fixtures` may construct options by literal.
- `docs/src/` executes; any example constructing options by literal will fail
  and must move to the builder — which is the migration working.

## Required evidence

- `E0027` for a probe field in **each** of the eight structs
- The field read/assign test from a separate crate
- The `..Default::default()` equivalence test
- Every construction site you migrated, across `src/ tests/ benches/ examples/
  docs/`, with the search command shown
- Corpus byte-comparison
- Gates including `--all-targets`
- CI run link

## Review request format

Per development policy §9.2. Additionally: state the final leaf-option count and
confirm it matches unit 06's, and list every construction site you had to
migrate — the architect's count for `build_with_matching` was 8 and the answer
was 10.
