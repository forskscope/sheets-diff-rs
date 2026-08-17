# Handoff 01 — Make documentation checkable

**Governing requirement.** NF-025 (MUST — document migration from v1/v1.2 to v2)
**Roadmap.** M6
**Sequence.** **First.** Units 02 and 03 depend on the harness this builds.

## Purpose

Make every Rust example in `docs/` compile in CI, and prove it on the 11
examples that exist today and are checked by nothing.

## Background

`docs/src/migration/v1-to-v2.md` contains **11 ```rust code blocks**. Nothing
compiles any of them. There is no `include_str!` anywhere in `src/`, no
`mdbook test`, no `book.toml`, and no docs job in CI.

So NF-025 is met in the sense that a migration document exists, and unmet in the
sense that its code may not work. A migration guide is read by someone changing
working code to code that does not yet compile — an example that is wrong there
costs more than almost anywhere else in the project.

M5 established that a rule nothing checks will eventually be false. This is that
rule applied to documentation, and units 02 and 03 are about to add many more
examples under requirements that name examples specifically.

### A mechanism that works

Verified before this handoff was written, not assumed:

```rust
#[doc = include_str!("../docs/src/migration/v1-to-v2.md")]
#[cfg(doctest)]
pub struct MigrationGuideDoctests;
```

`cargo test --doc` then reports each markdown block as a test, and a
deliberately broken block **fails**. `#[cfg(doctest)]` keeps the item out of
normal builds entirely.

**Take this as a starting point, not a specification.** `mdbook test` is the
obvious alternative and has the advantage of testing the book as published; it
also needs a `book.toml` that does not exist and a CI job that does not exist.
Choose, and justify the choice.

## Change scope

`src/lib.rs` (the harness item only), `docs/src/migration/v1-to-v2.md`,
`.github/workflows/ci.yaml` if your mechanism needs a step, `CHANGELOG.md`.
A `book.toml` if you take the mdbook route.

## Non-change scope

- **No behaviour change.** The harness must not exist in a normal build.
- Do not rewrite the migration guide's prose or restructure it. This unit makes
  its examples true; improving its content is not in scope.
- Do not add a dependency. If your chosen mechanism seems to need one, stop and
  report — the `#[cfg(doctest)]` route needs none.

## Required implementation

1. **A mechanism that compiles `docs/`'s Rust examples in CI**, covering the
   migration guide today and extensible to the pages units 02 and 03 will add.
   Say in the review request how a new page gets included, in one sentence — if
   that answer is long, the mechanism is wrong for this project.
2. **Fix or correctly mark every one of the 11 existing blocks.** Expect three
   outcomes and be explicit about which applies to each:
   - **Compiles as-is** — leave it.
   - **Is v1 "before" code** — it cannot compile against v2. Mark it ` ```text `
     or ` ```rust,ignore `, and say which convention you chose and why. This is
     legitimate and expected; what is not legitimate is an example that silently
     never compiles because nobody noticed.
   - **Is v2 code that is wrong** — fix the example. **If the example is right
     and the library is wrong, stop and report** — that is a defect, not a
     documentation task.
3. **Report the tally.** How many of the 11 compiled unchanged, how many were
   marked non-compiling, how many were broken. That number is the finding of
   this unit, whichever way it comes out.

## Required tests

**Demonstrate the harness catches a broken example.** Introduce a deliberate
error into a compiled block, run the suite, capture the failure, revert. Same
standard as M5 units 01 and 04: a check observed only passing is not yet a
check.

Also confirm the harness is **absent from a normal build** — `cargo build
--all-features` must not compile the doc item.

## Acceptance criteria

1. Every ```rust block in `docs/` either compiles in CI or is explicitly marked
   non-compiling.
2. The mechanism runs in CI on every feature combination that already runs
   `cargo test`, or the review request explains which legs it does not cover.
3. A deliberately broken example fails, demonstrated with a transcript.
4. The harness does not exist in a normal build, demonstrated.
5. The tally from implementation item 3 is stated.
6. The convention for non-compiling blocks is stated and applied consistently.
7. Adding a future page requires one obvious step, stated in one sentence.
8. No behaviour change; no `src/` change beyond the harness item.
9. Corpus byte-identical.
10. CHANGELOG under `### Added`; gates green, full matrix.

## Prohibited shortcuts

- **Do not mark a block `ignore` because it fails to compile.** Find out why
  first. `ignore` is for code that cannot compile by nature (v1 examples), not
  for code that should compile and does not.
- Do not delete an example to avoid fixing it.
- Do not weaken an example into something trivially compilable — an example that
  no longer shows the thing it was showing is worse than one that was never
  checked.
- Do not fix a library defect found this way. Report it.

## Known risks

- Doctests run per-crate, so an example needing a dev-dependency (say, a fixture
  builder) cannot use one. Examples should stand on the public API alone, which
  is what NF-024 wants anyway.
- Examples that open real files will fail in a doctest sandbox. Use
  `no_run` for those — they still get **compiled**, which is the property that
  matters, without executing. Say where you used it.
- The migration guide's blocks may reference v1 APIs that no longer exist in any
  form. That is the expected `text`/`ignore` case, not a problem to solve.

## Required evidence

- The mechanism, and where it runs
- The failure transcript from a deliberately broken example
- Proof the harness is absent from a normal build
- The 11-block tally
- CI run link

## Review request format

Per development policy §9.2, plus the tally and the one-sentence answer to "how
does a new page get included".
