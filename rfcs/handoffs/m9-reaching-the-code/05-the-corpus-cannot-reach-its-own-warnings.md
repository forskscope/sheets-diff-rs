# Handoff 05 — The corpus cannot reach its own warnings

**Milestone:** M9, unit 05. **Items:** O-B, plus `formula_unavailable` (added by f135's review).
**Written:** 2026-09-30. **This is D1's shape, in the fixtures.**

## Purpose

Three diagnostics this crate emits have **no corpus fixture that produces them**. Two alignment
warnings never had one; the third had one by accident and lost it when the accident was fixed. Add
the scenarios, so the golden corpus covers the behaviour it is supposed to guard.

## Background

**The pattern, stated once because it is the whole milestone:** a check that does not reach the code
it names is not evidence. D1 was that in the fuzz corpus. This is that in the fixture corpus.

**The two alignment warnings (O-B).** `RowKey`/`RowSignature` emit `MissingAlignmentKey` and
`alignment_bound_exceeded`; **no scenario in `tests/fixtures/generated/` produces either.** Verify
this yourself first — it was found before f130, f131 and f133 changed that area, and the first thing
to establish is whether it is still true.

**`formula_unavailable` (f135's review).** Confirmed 2026-09-30, after f135:

- **No golden mentions `formula_unavailable` at all.** Before f135, two did — `chart_sheet` and
  `typed_values` — and both were producing it **on sheets with no formulas**, which was the defect.
- The four formula-bearing fixtures cannot produce it: `error_values`, `formula`,
  `formula_at_first_cell`, `formula_shifted_origin` are **two cells each, with formulas on all of
  them**, so there is no plain numeric non-formula cell to count.

So the diagnostic's genuine case is covered by three unit tests in `tests/diagnostics.rs` and by
nothing in the corpus.

**Why the corpus and not more unit tests.** The goldens are the regression contract for *output* —
they catch a change in what a caller receives, whole-result, across every field at once. A unit test
asserts what someone thought to assert. Both matter; this gap is in the first.

## Change scope

- `tests/fixtures/` — new generated scenarios and their goldens.
- The fixture generator (`examples/gen-fixtures.rs`).
- `docs/src/maintainers/` — the coverage statement, if one claims these are covered.
- `rfcs/accepted/036-*.md` — RFC-036 is the coverage-obligation RFC; see *Required implementation* 4.

## Non-change scope

- **Nothing under `src/`.** If a scenario cannot be made to emit one of these, that is a finding
  about the engine — **report it and stop**, do not adjust the engine to make a fixture work.
- Do not change existing fixtures to produce these. `chart_sheet` and `typed_values` losing
  `formula_unavailable` was correct; adding a formula to them to get it back would be re-creating an
  accident deliberately. **New scenarios, named for what they cover.**
- Do not regenerate unrelated goldens.

## Required implementation

**1. Confirm the gap before filling it**, for all three, on the current tree. `MissingAlignmentKey`
and `alignment_bound_exceeded` were established before three units changed alignment. If either is
now reachable, say so and cover only what is missing.

**2. One scenario per diagnostic, named for the diagnostic.** Each must:
   - produce it through the ordinary corpus path (`compare_bytes` on the pair, the same route
     `generated_fixtures_match_golden` takes) — not through a special harness;
   - be **generated reproducibly** by the fixture generator, per RFC-036's acceptance criterion;
   - be minimal. A scenario that emits the target diagnostic and four others teaches nothing.

   For `formula_unavailable` that means a sheet with **at least one real formula and at least one
   plain numeric cell** — the shape none of the four formula fixtures has.

   `alignment_bound_exceeded` needs `old_rows × new_rows` over the bound. **At the default
   25,000,000 that is a large fixture.** Do not commit a huge workbook: either the scenario sets a
   low `max_alignment_product` through the options the corpus runner already supports, or — if it
   does not support per-scenario options — **stop and tell me**, because that is a gap in the corpus
   machinery and its shape is my decision, not a thing to work around.

**3. Assert the diagnostic is in the golden, not just that the golden exists.** A test that the
scenario's `expected.json` contains the diagnostic code, so a future change that silently stops
emitting it fails here rather than passing quietly with a regenerated golden.

**4. RFC-036 is the record that should have prevented this.** It is the coverage-obligation RFC.
Read §5.1 — the definition I sharpened on 2026-09-27, which requires an assertion to say *which
behaviour* it protects — and check whether RFC-036 claims a coverage obligation these three
diagnostics were already failing. **If it does, annotate it**: the obligation was stated and not
met, which is the same class of finding as the rest of this milestone. If it does not, propose the
one sentence that would have caught this, and let me rule on adding it.

## Required tests

- The three new scenarios, in the corpus, with goldens.
- The per-diagnostic assertions from §3.
- **Failing first:** for each new scenario, demonstrate the golden captures the diagnostic by
  removing the specific emitting statement in `src/` and showing that scenario's test fails — then
  restore `cmp`-identical. Three demonstrations. (This is the only place this unit touches `src/`,
  and only to break it temporarily.)
- `no_corpus_scenario_produces_a_code_outside_the_table` and
  `every_code_in_the_table_is_producible_and_nothing_else_is` must still pass — the second may get
  *easier*, since these codes become corpus-producible.

## Acceptance criteria

1. The gap confirmed or corrected for all three, on the current tree.
2. One minimal scenario per diagnostic, generated reproducibly, named for what it covers.
3. Each diagnostic asserted present in its golden.
4. Three failing-first demonstrations, each restoring `cmp`-identical.
5. No existing golden moves. If one does, that is a finding — **report it**.
6. RFC-036 checked, and annotated or a sentence proposed.
7. Gates as always, plus rule 003.

## Prohibited shortcuts

- Do not add a scenario that produces the diagnostic **by accident**, as a side effect of something
  else it is testing. Name it for the diagnostic or do not add it.
- Do not commit a multi-megabyte fixture to reach the alignment bound. See §2.
- Do not weaken `max_alignment_product`'s default to make a fixture cheap.
- Do not regenerate goldens in bulk to make the suite green.

## Known risks

**1. `alignment_bound_exceeded` may not be reachable cheaply**, and the honest answer might be that
the corpus cannot cover it without either a large fixture or per-scenario options. **Report that
rather than forcing it.** An uncoverable case, written down, is worth more than a fixture that
pretends.

**2. Adding a formula to reach `formula_unavailable` may move the `formula` fixture's neighbours** if
the generator shares code between scenarios. Check before assuming a clean addition.

**3. This unit's own success condition is a corpus that reaches three diagnostics.** It is not "three
new files". If a scenario is added and the diagnostic still does not appear in its golden, nothing
has been achieved — which is why §3 asserts on the golden's contents.

## Required evidence

Under `.git-exclude/review-request/m9-05-the-corpus-cannot-reach-its-own-warnings/evidence/`:

1. The before state: which diagnostics no scenario produces, established on the current tree.
2. Each new scenario's golden, with the diagnostic visible in it.
3. The three failing-first demonstrations and their `cmp` restores.
4. Corpus: which goldens moved (expect only the new ones).
5. What RFC-036 says, quoted, and your reading of whether it was already violated.
6. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/m9-05-the-corpus-cannot-reach-its-own-warnings/README.md`, with:

- Whether all three gaps were still open when you started.
- How you reached `alignment_bound_exceeded` without a large fixture, or why you could not.
- Whether RFC-036 already obliged this coverage, with the text.
- Any other diagnostic in the table that no scenario produces. There are more than three codes; if
  the list is longer than this unit's scope, **report the list** and I will scope the rest.
