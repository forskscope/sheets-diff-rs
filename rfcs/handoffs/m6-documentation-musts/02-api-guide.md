# Handoff 02 — The API guide

**Governing requirement.** NF-024 (**MUST**) — *"Document the v2 API with
examples for path, reader, bytes, options, and formatter usage."*
**Roadmap.** M6
**Sequence.** After unit 01 — its examples must be compiled by unit 01's harness.

## Purpose

Meet NF-024. It has been unmet since v2.0.0 and is a MUST.

## Background

There is no API guide. `docs/` has an index, a migration guide, and a threat
model. The only API examples in the project are three doctests in `src/`.

NF-024 names **five** example categories, and the enumeration is the
requirement — prose describing the API does not satisfy it:

| | What it means here |
|---|---|
| path | `compare_paths` — the common case |
| reader | `compare_readers` — any `Read + Seek` |
| bytes | `compare_bytes` — the case ForskScope's adapter uses |
| options | `DiffOptions::builder()` — comparison modes, alignment, limits |
| formatter | `render_summary` / `render_unified`, and the `serde` JSON path |

## Change scope

A new page under `docs/src/`, `docs/src/SUMMARY.md`, `docs/src/README.md`'s
contents list, the harness inclusion from unit 01, `CHANGELOG.md`.

## Non-change scope

- **Nothing under `src/`** except the one-line harness inclusion.
- Do not document behaviour you have not verified. Run the examples.
- Do not describe planned or partial capability as though it works. If a
  formatter or option is limited, say what it does today.

## Required implementation

1. **An example for each of the five categories**, each compiling under unit
   01's harness. `no_run` is fine where a real file is needed; the example is
   still compiled, which is what NF-024's "with examples" requires.
2. **The options example must show a limit being set**, not only comparison
   modes. `Limits::default()` leaves the linear bounds unset and
   `Limits::hardened()` exists for untrusted input — a consumer reading an
   options example and not learning that limits exist is the gap that let
   ForskScope run unbounded before M2. Reference the threat model rather than
   restating it.
3. **The formatter section must cover the `serde` JSON path**, gated behind the
   `serde` feature, since that is a documented output surface with no example
   anywhere. Note the feature gate explicitly.
4. **Say what each entry point costs.** `compare_bytes` currently copies its
   input (`to_vec()`), doubling peak memory — ForskScope flagged this as a real
   cost, it is recorded in the threat model, and a reader choosing between
   `compare_paths` and `compare_bytes` should learn it here rather than from a
   maintainer document. One sentence with a link.
5. **Error handling must appear at least once.** Every entry point returns
   `Result<_, SheetsDiffError>`; an example that `unwrap()`s throughout teaches
   the wrong thing to someone embedding this in a GUI. Show `code()` at least
   once — ForskScope's adapter matches on it, so it is load-bearing public
   surface.

## Required tests

The examples **are** the tests, once unit 01's harness includes the page. What
must be demonstrated:

1. Every example on the page is picked up by the harness — state the count, and
   confirm it matches the number of compiled blocks on the page.
2. At least one example is shown failing when deliberately broken, confirming
   the new page is genuinely covered and not merely adjacent to a harness that
   covers a different file.

## Acceptance criteria

1. A page exists documenting the v2 API, linked from `SUMMARY.md` and the docs
   index.
2. All five NF-024 categories have a compiled example.
3. The options example sets a limit and points at the threat model.
4. The formatter section covers the `serde` JSON path with its feature gate.
5. Each entry point's cost is stated, including `compare_bytes`'s copy.
6. At least one example handles the error rather than unwrapping, and `code()`
   appears.
7. The page's examples are covered by unit 01's harness, with the count stated
   and one demonstrated failing.
8. No behaviour change; no `src/` change beyond the harness inclusion.
9. Corpus byte-identical.
10. CHANGELOG under `### Added`, naming NF-024 as met; gates green, full matrix.

## Prohibited shortcuts

- Do not satisfy a category with a one-line signature. NF-024 says examples.
- Do not write an example you have not run.
- Do not omit the awkward parts. If an option combination behaves unintuitively,
  the example is where a reader finds out.
- Do not claim NF-024 met in the CHANGELOG unless all five categories are
  covered by compiled examples. Partial is partial, and this project has been
  bitten repeatedly by a requirement recorded as met on less evidence than that.

## Known risks

- Doctests cannot use dev-dependencies, so examples cannot build a workbook to
  compare. `no_run` with a plausible path is the expected shape; do not
  contort the API to make an example runnable.
- The `serde` example only compiles on a feature-enabled leg. Confirm which CI
  legs run it and say so — an example compiled by no leg is exactly the problem
  this milestone exists to fix.

## Required evidence

- The page
- The harness count for it, and one deliberate-failure transcript
- Which CI legs compile the `serde` example
- CI run link

## Review request format

Per development policy §9.2, plus the per-category mapping showing which example
satisfies which of NF-024's five.
