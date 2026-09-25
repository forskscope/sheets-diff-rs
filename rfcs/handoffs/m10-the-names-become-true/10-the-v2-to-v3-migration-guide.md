# Handoff 10 — The v2→v3 migration guide

**Governing.** [RFC-037 §5.6 and criterion 2](../../accepted/037-v3-scope.md)
**Roadmap.** M10 — final unit
**Sequence.** **Last.** After units 01–09, all landed, because the guide
describes what shipped rather than what was planned.

## Purpose

Nine units removed or changed public API. A caller on 2.6.0 needs one page that
tells them what broke and what to write instead. **RFC-037 criterion 2 makes it
an acceptance condition: every removal needs a row.**

## Background — the two requirements most likely to be missed

1. **The guide must be in the doctest harness.** `src/lib.rs:192` includes
   `docs/src/migration/v1-to-v2.md` via
   `#[doc = include_str!(…)] #[cfg(doctest)]`, which is why that page's examples
   cannot rot. **A migration guide whose examples are not executed is a
   migration guide nobody checks.** Add the new page the same way.
2. **The guide must be in `docs/src/SUMMARY.md`**, or mdbook does not build it
   into the book. The current entry is `- [Migration](migration/v1-to-v2.md)`, a
   single link; with two guides it needs to become a section with both beneath
   it. Check the rendered book, not just the file.

## The baseline — decide it explicitly and say it on the page

**Recommendation: the guide's baseline is 2.6.0**, stated in the first
paragraph, with a line telling a caller on an older 2.x to read the CHANGELOG up
to 2.6.0 first.

The alternative — covering every 2.x — means describing changes that are already
in the CHANGELOG and were not part of this milestone (the reorder exit code, the
diagnostics counters, `--format json`), and it makes the page longer without
making it truer. 2.6.0 is also the release we told our only known consumer to
adopt.

If you disagree, say so in the review request before writing 300 lines.

## Change scope

- `docs/src/migration/v2-to-v3.md` — new
- `docs/src/SUMMARY.md`
- `src/lib.rs` — the doctest include
- `CHANGELOG.md` — a `### Documentation` line
- Nothing under `src/` except `lib.rs`'s include. **If writing the guide reveals
  a defect, report it; do not fix it here.**

## Non-change scope

- **Do not change any API to make the guide easier to write.** If something is
  awkward to explain, that is a finding.
- Do not rewrite `v1-to-v2.md`. It stays as it is.
- Do not restate the whole CHANGELOG. The guide answers *"what do I write
  instead"*; the CHANGELOG answers *"what changed"*.

## Required content

### Every removal gets a row

Nine of them, each with what to write instead:

| Removed | Replacement |
|---|---|
| `SheetMatchReason::{ExactName, ContentSimilarity, IndexAndContent}` | `SameIndex` / `SoleRemainingPair` — **and the migration is not a rename**: an arm on `IndexAndContent` almost always meant "this is a rename", which is `SheetChange::Renamed {..}` / `RenamedAndMoved {..}` |
| Four `DiagnosticKind` variants **and their code strings** | nothing — they could never occur. **Name the strings**: a caller matching `code()` gets no compile error |
| `SourceKind::Unknown` | nothing — the three entry points always know |
| `DiffOptionsBuilder::number_compare` | `number_compare_policy` |
| `DiffOptionsBuilder::build_with_matching` | `.sheet_matching(…)` and/or `.alignment(…)` — **and say it fixes a silent bug**: the old method replaced the whole struct and discarded an earlier `sheet_matching` |
| `FormulaCompareMode::{NormalizedText, RawAndNormalized}`, `FormatCompareMode`, `ComparisonOptions::format`, `format_compare` | nothing — they returned `InvalidOptions` if selected. Say when they may return (RFC-022) |
| `AlignmentMode::HeaderColumn` | `RowKey { columns: vec![1] }` — exactly equivalent |
| `Severity::Error`, `DiagnosticSummary::errors`, the CLI's `N error(s)` | nothing — no diagnostic was ever fatal |
| `SheetsDiffError::{UnsupportedFormat, Internal}`, `OpenErrorKind::Locked` | `OpenWorkbook { kind: NotXlsx }` for the first; for `Locked`, note a locked file arrives as `PermissionDenied` or `Other` |

### The two that are not removals, and are the most likely to bite

**1. `..Default::default()` no longer compiles from another crate.**

This is the one to lead the "Changed" half with, because **we documented the
pattern it breaks.** The API guide showed
`Limits { max_sheets: Some(50), ..Limits::default() }` and
`tests/integration.rs::limits_struct_update_syntax_still_compiles` pinned it as
a compatibility promise. Anyone who followed our own guide is in this migration.

Show the error (`E0639`) and both replacements — the builder, and
`Default::default()` plus field assignment — as **executable examples**. Say
plainly that functional update is a struct expression, so `..` does not help.

**2. "New fields are additive" needs a qualification for `Ord`-deriving types.**

`#[non_exhaustive]` makes a future field additive **for compilation**, not
behaviourally neutral. `CellAddress` derives `Ord` over `row, col, a1`, so a
field added later extends that lexicographic order. Narrow today — `a1` is
derived from the other two — but a caller sorting by `CellAddress` should know
the guarantee is about compiling, not about order.

### The rest of the Changed half

`SheetMatchReason`'s meaning; `DiagnosticLocation` now naming the sheet;
`cells_read` and `max_cells_read` counting populated cells (**and that the bound
only loosens** — no workbook accepted by 2.6.0 is rejected now);
`to_json`/`to_json_pretty` returning `String` (**the rare break that makes
calling code shorter — delete the `?`**).

### What did not change

Worth a short section, because a migration guide that only lists breaks reads
as though everything moved: comparison results are unchanged, the serialised
shape is unchanged apart from the fields named above, MSRV is still 1.88, and
the six `compare_*` entry points are untouched.

## Required tests

- **Every example executes.** That is what the `include_str!` gives you; the
  test is `cargo test --doc` passing with the page included.
- At least one example per *Changed* item that a caller has to act on, and one
  per replacement in the removal table where a replacement exists.
- **The `E0639` example is a `compile_fail` doctest**, so the page proves the
  break rather than asserting it.
- `mdbook build` (or the project's doc check) resolves every link. **Relative
  links from `migration/` need `../`** — unit 06 of M8 shipped dead
  `README.md#anchor` links because mdbook renders `README.md` as `index.html`.

## Acceptance criteria

1. `docs/src/migration/v2-to-v3.md` exists and covers **all nine removals**,
   each with a replacement or an explicit "nothing".
2. The four dead **code strings** are named, since matching on them fails
   silently rather than at compile time.
3. `..Default::default()` leads the Changed half, with the `E0639` `compile_fail`
   example and both replacements, and says we documented the broken pattern.
4. The `Ord` qualification is stated.
5. The baseline is stated in the first paragraph.
6. A "what did not change" section exists.
7. **The page is in `src/lib.rs`'s doctest includes** and `cargo test --doc`
   covers it — state the doctest count before and after.
8. **`SUMMARY.md` has both guides**, and the book builds with no dead links.
9. CHANGELOG `### Documentation`.
10. Gates green, including `cargo check --manifest-path fuzz/Cargo.toml --bins`.

## Prohibited shortcuts

- **Do not write an example you have not run.** The whole point of the harness
  is that this page cannot drift; an example that is `text`-fenced to avoid a
  compile error defeats it. If something genuinely cannot be executed, mark it
  `ignore` **and say why in the review request**.
- Do not paste CHANGELOG bullets. They answer a different question.
- Do not omit a removal because its replacement is "nothing" — that row is the
  most useful one to a caller searching for the name.
- Do not soften the `..Default::default()` break. We recommended that pattern.

## Known risks

- **The page is long and every example is compiled**, so a mistake is a build
  failure rather than a silent wrong claim. That is the design working; budget
  for the iteration.
- `compile_fail` doctests pass if the code fails to compile **for any reason**,
  including a typo. Pin the expected error — `compile_fail,E0639` — as unit 08
  did, or the example proves nothing.

## Required evidence

- `cargo test --doc` before and after, with the counts
- The rendered book, or the link check, showing no dead links
- The nine-removal table checked against `CHANGELOG.md`'s `### Removed`, with
  the comparison shown — **that is RFC-037 criterion 2, and it is the one thing
  a reviewer cannot reconstruct cheaply**
- Gates, each with exit status
- CI run link

## Review request format

Per development policy §9.2. Additionally: state whether you agree with the
2.6.0 baseline, list any example you had to mark `ignore` and why, and confirm
the removal table against the CHANGELOG rather than against this handoff — **my
list of nine is a list of mine**, and this project's counts have been wrong
eight times.
