# Handoff 03 — The builder's surface, settled once

**Governing.** [RFC-037 §3.7](../../accepted/037-v3-scope.md); RFC-006 (diff
options and the configuration builder); RFC-033 §11
**Roadmap.** M10 — unit 03
**Sequence.** Any time. Parallel with 02 and 04.

## Purpose

`DiffOptions` documents `builder()` as the construction entry point. Two options
cannot be set through it, two methods should not exist, and the naming is
inconsistent. **Settle all of it in one unit**, because deciding the naming rule
twice is how it got inconsistent in the first place.

## Background

M8 unit 07 added `min_severity` and `alignment` and left the rest. The audit it
ran — by script, over `src/options.rs`, not by grep — found the state this unit
finishes:

- **20 leaf options.** 18 now have a setter of their own.
- **`comparison.value.date` (`DateComparePolicy`) has no builder method at all.**
  A real option with real behaviour (`src/compare.rs`), documented in the README.
- **`limits.max_cells_read` has no method of its own**, while its five `Limits`
  siblings each do. Reachable only by rebuilding the whole struct via `limits(..)`.
- **`number_compare` and `number_compare_policy` are identical** — same argument
  type, same field, two names.
- **`build_with_matching(MatchingOptions)` assigns the whole struct**, so it
  silently discards a `sheet_matching` set earlier in the chain. Demonstrated:
  `.sheet_matching(ExactNameOnly)` then `build_with_matching(..)` ends with the
  default; `.sheet_matching(ExactNameOnly)` then `.alignment(..)` keeps
  `ExactNameOnly`. With unit 07's two setters it has no remaining use a chain
  does not cover.

## Change scope

- `src/options.rs`
- `tests/`
- `rfcs/done/006-*.md`, `rfcs/done/033-*.md` §11
- `docs/src/api-guide.md` — if it shows builder usage
- `CHANGELOG.md`

## Non-change scope

- **Do not change any option's type, default or meaning.** Every option settable
  today stays settable, with the same effect.
- Do not make any field private. Direct field assignment must keep working.
- Do not touch `build()` or `validate()`.
- Do not add setters beyond the two named. If the audit finds a third gap,
  **report it** — that is a finding, not a task.

## Required implementation

1. **Settle the naming rule first, and write it down**, before touching a line.
   The two existing conventions are `<field>_compare` (`formula_compare`,
   `format_compare`) and `<field>_policy` (`numeric_type_policy`,
   `type_mismatch_policy`). The implementer's recorded view is to keep
   `number_compare_policy` because it matches its siblings *within the same
   struct*. **State the rule in the review request as one sentence**, then apply
   it to every method you add or keep.
2. **Add `date_compare` / `date_policy`** — whichever your rule dictates — for
   `comparison.value.date`.
3. **Add `max_cells_read(Option<u64>)`**, matching `max_alignment_product` and
   `max_input_bytes`, which take `Option` because `None` is meaningful.
4. **Remove the duplicate** of `number_compare` / `number_compare_policy`, per
   your rule.
5. **Remove `build_with_matching`.**
6. **`DiffOptions`'s doc comment becomes simply true**: the builder covers every
   option. No "except". **Audit it rather than asserting it** — M8 unit 07's
   criterion told the implementer to assert this and it was false at the time;
   that is why this unit exists.
7. **Remove `#[allow(dead_code)]` from `AlignmentMode` and its `HeaderColumn`
   variant.** Verified redundant: rustc never reports a `pub` variant of a
   public enum as dead, and clippy stays clean without them. Two lines.
8. **RFC-033 §11's builder-coverage paragraph** records the final state and the
   naming rule.

## Required tests

- **The audit, as a test.** Every leaf option is set through the builder and
  read back, so a future option added without a setter is caught by something
  other than a person noticing. If a data-driven form is impractical, one test
  naming all twenty is acceptable — say which you chose and why.
- A comparison configured **entirely through the builder** equals the same
  configuration set by field assignment, including `date` and `max_cells_read`.
  Keep unit 07's non-vacuity controls: the equality must be between two
  *configured* states, not two defaults.
- `max_cells_read(None)` behaves as the default.

Removals demonstrate themselves by failing to compile; additions do not. Show
the new setters failing first by a targeted change, and say plainly which
demonstrations are compile failures.

## Acceptance criteria

1. The naming rule is stated in one sentence and applied consistently.
2. `date` and `max_cells_read` have setters; all **20** leaves are reachable.
3. `number_compare`/`number_compare_policy` is one method.
4. `build_with_matching` is gone.
5. `DiffOptions`'s doc comment is true, **verified by an audit you ran**, with
   the audit reported.
6. Both `#[allow(dead_code)]` attributes removed, clippy clean under
   `--all-features` **and** `--no-default-features`.
7. No option's type, default or behaviour changed; field assignment still works.
8. Corpus byte-identical.
9. CHANGELOG `### Added` and `### Removed`, with the migration for both removed
   methods — including that `build_with_matching`'s callers should chain
   `.sheet_matching()` and `.alignment()`, which is **better** than what they
   had, since it no longer discards.
10. Gates green.

## Prohibited shortcuts

- **Do not add a whole-struct setter** — `diagnostics(DiagnosticOptions)`,
  `matching(MatchingOptions)` — to close the gap. That is `build_with_matching`
  again, and discarding sibling fields silently is the defect being removed.
  `limits(Limits)` survives because `Limits` has six fields and a `hardened()`
  constructor; that is a reason, not a precedent.
- Do not deprecate instead of removing. This is the major.
- Do not deprecate field assignment.
- Do not rename a surviving setter to satisfy the rule unless the rule genuinely
  requires it — each rename is a break and they are not free.

## Compatibility constraints

**Breaking.** Callers of `build_with_matching` and of whichever `number_compare*`
goes will not compile. Both have a mechanical replacement; give it in the
CHANGELOG.

`build_with_matching`'s removal is the one to explain carefully: its replacement
is not merely equivalent, it **fixes a silent bug** in any call that set
`sheet_matching` beforehand. Say that — a caller who reads only the migration
line should learn their old code may have been losing a setting.

## Known risks

- **`build_with_matching` is used by the existing test suite** — unit 07's
  review noted alignment tests going through it (`tests/integration.rs` ×6,
  `tests/diagnostics.rs`, and `docs/src/semantics.md`, which executes). All must
  move to the chain. **Check the count yourself**; mine is from a review, not a
  fresh search.
- `AlignmentMode::HeaderColumn` is also RFC-037 §3.6's subject, which is
  **blocked on the owner**. This unit only removes an attribute; it must not
  pre-empt whether the variant is implemented, renamed or removed.

## Required evidence

- The naming rule, stated
- The audit output, before and after
- The `build_with_matching` call-site migration, with your own count
- Compile-failure transcripts for the removals; targeted failures for the additions
- Corpus byte-comparison
- CI run link

## Review request format

Per development policy §9.2. Additionally: state the naming rule in one
sentence, and report your own count of `build_with_matching` call sites.
