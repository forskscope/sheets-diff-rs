# Handoff 03 — The XML-patched scenarios

**Governing RFC.** [RFC-036](../../accepted/036-coverage-obligation-and-the-fixture-matrix.md) §5.2, scenarios 10–11
**Roadmap.** M3, track A
**Sequence.** After unit 01. Independent of unit 02 — either order.

## Purpose

Add the two matrix scenarios `rust_xlsxwriter` cannot express, using the
`patch_xlsx_xml` helper that already exists.

## Background

Unit 01 established that exactly two of the eleven scenarios need raw XML:

- **#10** — a *physically present* empty cell (`<c r="A1"/>` with no value)
  before real content. `rust_xlsxwriter` has no reason to emit an empty cell
  element for a cell nothing was written to. This exercises a real and
  non-obvious calamine behaviour: a `Cell{val: DataRef::Empty}` is filtered out
  of the collected list **before** `Range::from_sparse` computes `start`/`end`,
  so a present-but-empty leading cell does not anchor the origin the way a
  populated one would.
- **#11** — an ISO-typed `t="d"` cell, promoting Handoff 05's hand-built
  reachability test into the corpus so the golden becomes a durable trip-wire
  rather than a single integration test.

`tests/support.rs::patch_xlsx_xml` exists and five tests use it. This unit
extends its use; it should not need new capability.

## Change scope

`examples/gen-fixtures.rs` (or a patched-generation path within it),
`tests/fixtures/generated/*/` (two new scenarios), `tests/integration.rs`,
`tests/fixtures/corpus/README.md`, `CHANGELOG.md`.

## Non-change scope

Do not touch `src/`. Do not modify existing scenarios. Do not attempt the
`<dimension>`-tag case — unit 01 established it is not a hazard.

## Required implementation

1. **Decide where patched generation lives, and say why.** The corpus generator
   is an example that deliberately does *not* depend on `sheets-diff`, so the
   fixture bytes cannot be influenced by the code under test — a property worth
   preserving. `patch_xlsx_xml` currently lives in `tests/support.rs`. Either
   move or duplicate it, consistent with the documented divergence between
   `support.rs` and the generator's own builders, and record the choice.
2. **#10** — generate a workbook, patch in a leading empty `<c>` element,
   confirm calamine's filtering behaviour is what unit 01 read from the source,
   and assert on the resulting `compared_range`. If the observed behaviour
   differs from unit 01's source reading, **that is a finding** — report it.
3. **#11** — promote the ISO scenario into the corpus with a golden, keeping the
   existing hand-built test or replacing it, whichever leaves the clearer
   record. Say which and why.
4. Both scenarios must be **byte-reproducible**: patched output must be stable
   across runs, as the unpatched generator already is. Verify by generating
   twice and comparing.

## Required tests

An assertion per scenario satisfying RFC-036 §5.1 — for #10, on the range
bounds the empty cell does or does not anchor; for #11, on the comparison
result, not merely that parsing succeeded.

## Acceptance criteria

1. Both scenarios exist with fixture pairs, goldens, and assertions.
2. Patched generation is byte-reproducible across two runs.
3. `cargo test` leaves the corpus untouched.
4. The seven pre-existing scenarios are byte-identical.
5. The placement decision from item 1 is recorded with its reasoning.
6. Full matrix, gates, MSRV, CI green.
7. No comparison behaviour changes; a failing new assertion is a **finding**,
   reported, not adjusted away.

## Prohibited shortcuts

- Do not hand-write a whole `.xlsx` by hand. Generate, then patch — the point is
  a minimal, reviewable delta from a known-good file.
- Do not bless a golden without reading it.
- Do not let patched fixtures become non-reproducible. A corpus that changes
  between runs is the defect M1 removed.

## Known risks

- Patching may perturb the zip in ways that affect byte-stability. If it does,
  report what and propose a fix rather than accepting an unstable fixture.
- Unit 01's #10 behaviour claim comes from reading calamine's source, not from
  running it. This unit is where it gets tested. It may be wrong.

## Required evidence

- Two generator runs with matching checksums
- Per scenario: the assertion and its output
- Confirmation that the seven pre-existing scenarios are unchanged
- CI run link
- Whether #10's observed behaviour matched unit 01's source reading

## Review request format

Per development policy §9.2, plus the §1 placement decision and the #10
behaviour confirmation.
