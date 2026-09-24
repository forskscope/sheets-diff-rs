# Handoff 07 — The builder advertises itself and omits two options

**Governing RFCs.** RFC-033 (public model lexicon, §11 options tree)
**Roadmap.** M8 — unit 07
**Sequence.** After unit 02 (landed 2026-09-25). Parallel with 04, 06.

## Purpose

`DiffOptions`'s own doc comment says *"Construct via `DiffOptions::default()` or
`DiffOptions::builder()`."* The builder has 19 setters and cannot set two of the
options it implies it covers. Close the gap.

## Background

Found by the implementer as O-D while implementing `min_severity` in unit 02,
and confirmed larger than reported during that review.

`DiffOptionsBuilder` has setters for comparison, matching, limits and execution
— 19 in all. It has none for:

| Option | Why it matters |
|---|---|
| `diagnostics.min_severity` | Unit 02 just gave it behaviour. A caller who reads the CHANGELOG, goes to the builder, and cannot find it will conclude it does not exist. |
| `matching.alignment` | **The one that reaches the alignment machinery** whose warnings unit 02 just made visible. `matching.sheet_matching` has a setter; `matching.alignment`, its sibling in the same struct, does not. |

A caller must reach past the advertised entry point and assign the field:

```rust
let mut opts = DiffOptions::default();
opts.diagnostics.min_severity = Some(Severity::Warning);
opts.matching.alignment = AlignmentMode::RowKey { columns: vec![1] };
```

That works — the fields are `pub` — but it means the builder is not what its own
documentation says it is. **Someone reads `DiffOptions::builder()` and believes
it can configure the options.** That is this milestone's sentence.

## Change scope

- `src/options.rs` — two builder methods and the `DiffOptions` doc comment
- `tests/` — coverage for both
- `rfcs/done/033-*.md` §11 — the options tree record
- `docs/src/api-guide.md` — if it shows builder usage
- `CHANGELOG.md`

## Non-change scope

- **Do not change any option's type, default or meaning.** This unit adds two
  ways to set things that are already settable.
- Do not make any field private. They are `pub` and stay `pub`; direct
  assignment must keep working.
- Do not add setters for anything else, or "while I was here" convenience
  methods. Two options are missing; add two.
- Do not touch `build()` or `validate()`.

## Required implementation

1. **`min_severity(self, severity: Option<Severity>) -> Self`** — taking
   `Option`, because `None` is the documented default and a caller must be able
   to say it. Follow `max_alignment_product`/`max_input_bytes`, which already
   take `Option` for the same reason, rather than the plain-value setters.
2. **`alignment(self, mode: AlignmentMode) -> Self`**, named to match the field,
   and placed beside `sheet_matching` since they are siblings in
   `MatchingOptions`.
3. **Doc comments on both**, each with a compiled doctest. `min_severity`'s says
   it is a collection filter and points at the field's own documentation for the
   counters rule — do not restate that rule in two places where it can drift.
4. **`DiffOptions`'s doc comment becomes true.** Either the builder covers every
   option, or the comment says which options are field-assignment only. After
   this unit the first is the case; say so, and **audit it rather than assume
   it** — I asserted "two" from a grep, and the standing instruction in this
   milestone is not to trust my counts.
5. **RFC-033 §11's options tree** records that both are builder-reachable.

## Required tests

- A comparison configured **entirely through the builder**, setting
  `min_severity` and `alignment`, produces the same `WorkbookDiff` as the same
  configuration set by field assignment. One test, two paths, equal results —
  that is the property, not "the setter sets the field".
- `min_severity(None)` is accepted and behaves as the default.
- The `alignment` setter reaches the alignment machinery: a fixture whose result
  differs between `Positional` and `RowKey`, configured via the builder.

Each shown failing before the fix — which, for a new method, means the test does
not compile. **Say so plainly rather than dressing it up as a runtime failure**;
a compile failure is a legitimate pre-fix demonstration for an additive API, and
pretending otherwise would be the kind of theatre this project does not do.

## Acceptance criteria

1. Both setters exist, named for their fields, with doctests.
2. `min_severity` takes `Option`.
3. The builder-vs-field equivalence test passes.
4. The `alignment` setter demonstrably reaches alignment.
5. `DiffOptions`'s doc comment is true, **verified by an audit of the options
   tree against the builder's methods**, with that audit reported.
6. No existing setter, field, default or behaviour changed.
7. Fixture corpus byte-identical.
8. CHANGELOG under `### Added`; gates green.

## Prohibited shortcuts

- Do not add a generic `diagnostics(DiagnosticOptions)` or
  `matching(MatchingOptions)` escape hatch instead. `limits(Limits)` exists
  because `Limits` has six fields and a `hardened()` constructor; these two
  structs do not need one, and a whole-struct setter silently discards the
  other fields a caller may have set.
- Do not deprecate field assignment.
- Do not write the counters rule into the builder's doc comment. Link to the
  field.

## Compatibility constraints

**Purely additive.** Two new methods on a builder. Nothing existing changes.

## Known risks

- `DiffOptionsBuilder` has two near-duplicate setters already
  (`number_compare` and `number_compare_policy`, both taking
  `NumberComparePolicy`). **Do not fix that here** — it is a real observation,
  possibly a deprecation for v3, and it is not this unit's. Report it if you
  form a view.
- `AlignmentMode::HeaderColumn` carries `#[allow(dead_code)]`. Making the mode
  builder-reachable may make that attribute wrong. If clippy or the compiler
  objects, that is a finding worth reporting, not a reason to widen scope.

## Required evidence

- The options-tree audit (item 4 / criterion 5)
- The equivalence test, both paths
- Corpus byte-comparison
- CI run link

## Review request format

Per development policy §9.2. Additionally: report your own count of
builder-unreachable options. Mine is two and it is a count of mine.
