# Handoff 02 — The rule deviations

**Milestone:** M9, unit 02. **Items:** B1–B4.
**Written:** 2026-09-30, state verified on the current tree. **The largest churn in M9 and the least
behaviour** — which is exactly why it needs the tightest guard.

## Purpose

`.git-exclude/rules/project-instructions-rust.md` prescribes a file layout this crate does not
follow. Follow it, without losing a single test on the way.

## Background — verified 2026-09-30

**B1 — inline `#[cfg(test)] mod tests`, eight files.** The rule (`:13`–`:14`) marks the inline form
❌ and prescribes `src/some_mod.rs` + `src/some_mod/tests.rs`. Current:
`src/{matcher,compare,address,diff,align,normalize,builder_coverage,lib}.rs` all carry inline test
modules. `src/diff.rs` also has a `#[cfg(test)] impl NormalizedCell::for_test`.

**B2 — `src/output/mod.rs`.** The rule prescribes the 2018 style: `src/output.rs` alongside
`src/output/`. Current: `src/output/{mod,json,text,view}.rs`.

**B3 — `tests/integration.rs` is 2,336 lines** (2,054 when audited; it has grown). The rule says to
apply "the same file-splitting logic (based on line counts)" to `tests/` — **and defines no line
count anywhere.** I ruled on this once already, at m8-03's review: *leave it; B3 has no threshold and
inventing one mid-milestone is worse than a long file.* **That ruling stands. B3 is out of scope
here** — it is listed so nobody thinks it was forgotten.

**B4 — two `#[allow]`s in `src/`**, both `#[allow(clippy::too_many_arguments)]`, at `src/diff.rs:346`
and `:395`. Down from eight at audit time; the `dead_code` allows the audit flagged on
`AlignmentMode` are gone. CI's `lint` job comment says "no `#[allow(...)]` silencing", which those
two contradict.

## Change scope

- `src/` — file moves, module declarations, and B4.
- Nothing else. No `tests/`, no `docs/`, no goldens.
- `CHANGELOG.md` only if B4's resolution changes a signature.

## Non-change scope

- **No test may change.** Not its name, not its body, not its assertions. This unit *moves* tests; it
  does not touch them. A moved test that needed editing to compile is a finding — report it.
- **B3 is out of scope** (above).
- **No behaviour change.** `cargo public-api --simplified diff 3.2.0` must show nothing.
- Do not split `src/diff.rs` or any other module because it is long. That is not what this asks.

## Required implementation

**1. Pin the test inventory before touching anything.** Capture the exact name of every test in the
lib suite — `cargo test --lib -- --list`, sorted, committed to the evidence. **This is the guard for
the whole unit**, because the failure mode of moving test modules is a test that silently stops being
compiled: a missing `mod tests;`, a `use super::*` that no longer reaches, a `#[cfg(test)]` helper
left behind. A dropped test does not fail; it disappears.

**2. B1, one module at a time**, re-running the count after each. For each of the eight: move the
inline `mod tests` body to `src/<name>/tests.rs`, add `#[cfg(test)] mod tests;` to the parent, and
fix only what the compiler demands — paths, not content.

   **`src/lib.rs` is the awkward one**: its tests live beside the crate root, so the target is
   `src/tests.rs`. Do it last, and if the rule's intent is unclear for the crate root, **say so and
   leave it** rather than inventing a layout.

   **`src/diff.rs`'s `#[cfg(test)] impl NormalizedCell::for_test`** is a test-only helper on a type
   declared elsewhere. It must remain compiled for tests and invisible otherwise. If moving it is not
   clean, leave it where it is and note why — the rule is about test *modules*, not every
   `#[cfg(test)]` item.

**3. B2** — `src/output/mod.rs` → `src/output.rs`, keeping `src/output/{json,text,view}.rs`. A pure
rename plus the module declaration.

**4. B4 — propose, do not silently remove.** Deleting the two `#[allow]`s makes clippy fail; the real
question is whether those two functions should take fewer arguments. Both are internal
(`read_sheet_cells`, `build_sheet_diff`), so a context struct is possible without touching the public
API. **Count the arguments, state what a struct would group, and say whether it is worth it.** My
lean: worth it if the grouping is meaningful (a "read context" that travels together anyway), not
worth it if it is a bag of unrelated parameters wearing a struct. Either way **I rule, you propose**
— and if the answer is "keep the allow", then CI's lint comment is what should change, and say so.

## Required tests

None new. The guards are:

- **The test inventory is identical**, name for name, before and after. Not the count — the *names*,
  diffed.
- Every gate green under every feature combination, since module layout can differ per feature.
- `cargo public-api --simplified diff 3.2.0`: nothing.

## Acceptance criteria

1. The eight inline test modules moved, or a stated reason for each that was not.
2. `src/output.rs` in the 2018 style.
3. **The lib test inventory diffs to nothing** — same names, same number.
4. B4 proposed with the argument counts and a recommendation.
5. Full matrix green; public API unchanged; gates including rule 003.
6. `git diff --stat` touches only `src/`.

## Prohibited shortcuts

- **Do not report a test count as evidence.** Report the sorted name list, diffed. A count can stay
  the same while one test vanishes and another is added.
- Do not fix a moved test that fails to compile by editing the test. Fix the module path, or report
  it.
- Do not resolve B4 by deleting the `#[allow]` and letting clippy fail, nor by widening the lint
  config.
- Do not touch `tests/integration.rs`.

## Known risks

**1. A silently dropped test is the whole risk of this unit.** Everything else is mechanical. §1's
inventory is not bureaucracy; it is the only thing that would catch it.

**2. Feature-gated tests.** Some test modules may be inside `#[cfg(feature = "serde")]` or similar.
Run the inventory under **each** feature combination, not just `--all-features` — a test that only
exists under `--no-default-features` would otherwise be invisible to the guard.

**3. This unit has the worst churn-to-value ratio in M9**, and that is not a reason to skip it — the
rule is the rule — but it *is* a reason to do it in small steps with the guard re-run each time. If
something resists, leaving one module inline with a written reason is a better outcome than a clever
restructure.

**4. Review cost.** Eight file moves are hard to review as one diff. **Commit locally per module** so
I can read them one at a time; I will squash or keep as I see fit.

## Required evidence

Under `.git-exclude/review-request/m9-02-the-rule-deviations/evidence/`:

1. The test inventory before and after, sorted, and the diff between them (expected: empty).
2. The same inventory under each feature combination.
3. Per-module: what moved, and anything the compiler demanded.
4. B4: argument counts, the proposed grouping, your recommendation.
5. Public API diff; full matrix; gate sweep, one scratch dir, deleted.
6. `git diff --stat`.

## Review request format

`.git-exclude/review-request/m9-02-the-rule-deviations/README.md`, with:

- The inventory diff, stated as empty or explained.
- Any module you left inline, and why.
- Your B4 recommendation and the case against it.
- Whether the rule's intent is clear for the crate root (`src/lib.rs`'s tests). If it is not, say so;
  I would rather change the rule than guess at it.
