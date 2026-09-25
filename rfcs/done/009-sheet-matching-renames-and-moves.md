# RFC-009: Sheet Matching, Renames, and Moves

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.

**Corrected M10 unit 01 (unreleased):** §6 describes a matcher that, for the one-removed /
one-added case, "compare[s] metadata and optionally sample[s] content" and scores
`index_similarity + dimension_similarity + sampled_content_similarity`. **None of that was
implemented, and the Status above does not say so.** The matcher inspects no cell content and
no sheet dimensions. It pairs on name; then, in the default mode, pairs the single unmatched
sheet on each side (`Medium` confidence if their tab positions agree, `Low` and
`RenamedAndMoved` if not); and under `ExactNameThenIndex` pairs on tab position (`Low`).
`SheetMatchReason` used to claim otherwise and now names those causes. §9's acceptance line
"one renamed sheet with the same content is detected as renamed" passes because *any* sole
leftover pair is a rename, not because content was compared. Content comparison remains an
open proposal, not a shipped behaviour.
**Target:** v2.0.0 conservative, v2.x extensible  
**Created:** 2026-06-11  
**Category:** Sheet comparison  

## 1. Summary

Detect logical sheet pairs beyond exact name matches, with conservative rename/move handling and explicit ambiguity diagnostics.

## 2. Motivation

v1 compares cells only for sheets with the same name. If a user renames a tab, the sheet appears removed and added, and all cell-level diffs are lost. v2 should recover common rename cases without pretending to know ambiguous matches.

## 3. Goals

- Represent added, removed, same-name, renamed, moved, and ambiguous sheet states.
- Preserve cell diffs across conservative rename matches.
- Avoid false confident matches when multiple candidates are plausible.
- Expose match confidence and reason.
- Keep exact-name matching deterministic and cheap.

## 4. Non-goals

- Do not implement expensive global optimal matching by default in v2.0.
- Do not silently pair ambiguous sheets.
- Do not require content reading of every sheet before cheap exact matching.

## 5. External design

Public model:

```rust
pub enum SheetChange {
    Unchanged,
    Added,
    Removed,
    SameName,
    Renamed { from: String, to: String, confidence: MatchConfidence },
    Moved { from_index: usize, to_index: usize },
    RenamedAndMoved { from: String, to: String, from_index: usize, to_index: usize, confidence: MatchConfidence },
    Ambiguous { candidates: Vec<SheetMatchCandidate> },
}

pub enum SheetMatchingMode {
    ExactNameOnly,
    ExactNameThenConservativeRename,
    ExactNameThenIndex,
    CustomKeys, // reserved or future
}
```

## 6. Internal design

> **Corrected M10 unit 01 (3.0.0, unreleased): this section describes an
> approach that was never built.** No scoring function exists; `grep` across
> `src/` finds zero occurrences of `index_similarity`, `dimension_similarity`
> or `sampled_content_similarity`, and `src/matcher.rs` mentions neither
> dimensions nor sampling. Step 3's "compare metadata and optionally sample
> content" does not happen either.
>
> **What the matcher actually does:** exact-name pairing first; then, in the
> default mode, if exactly one unmatched sheet remains on each side it is paired
> by elimination — `Renamed` at `Medium` when the tab positions agree,
> `RenamedAndMoved` at `Low` when they do not. `ExactNameThenIndex` pairs the
> remainder on equal tab position. Nothing else is consulted, and **no cell
> content is read at any point**: `match_sheets` is called at `src/diff.rs:220`
> and the first `read_sheet_cells` at `:359`, so matching decides before any
> sheet has been read.
>
> The last line below — "for v2.0, exact single add/remove should be enough" —
> is what shipped, and it shipped alone.
>
> **This paragraph is the origin of a defect that took three milestones to
> find.** `SheetMatchReason::ContentSimilarity` and `::IndexAndContent` were
> named after this design rather than after the code, so every rename the engine
> reported claimed a content check that never existed. The enum was corrected in
> M10 unit 01; this section is corrected here so the belief has no source left.
>
> Reintroducing content comparison is a **feature**, not a correction, and would
> have to be a new opt-in `SheetMatchingMode` — the default must not start
> reading sheets before the limits that bound reading have been applied.

Matching phases:

1. Exact name match.
2. Detect added and removed names.
3. If exactly one removed and one added sheet remain, compare metadata and optionally sample content.
4. If confidence passes threshold, emit `Renamed` and compare cells.
5. If multiple candidates remain, emit added/removed plus warning or `Ambiguous` based on model decision.

Internal candidate score:

```text
score = index_similarity + dimension_similarity + sampled_content_similarity
```

For v2.0, exact single add/remove should be enough to support the common tab-rename case.

## 7. Data lifecycle

1. Workbook sheet metadata is collected.
2. Exact-name pairs are removed from unmatched sets.
3. Conservative rename/move matcher processes remaining sheets.
4. Matched pairs go to cell comparison.
5. Unmatched old sheets become removed.
6. Unmatched new sheets become added.
7. Ambiguity diagnostics are attached.

## 8. Error, diagnostic, and edge-case behavior

Ambiguous matches must not be hidden. If two removed sheets and two added sheets are similar, v2.0 should either leave them as add/remove or return an ambiguity warning, depending on options.

If a matched sheet later fails to read, sheet-level read diagnostics apply normally.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Exact same-name sheets match.
- One renamed sheet with same content is detected as renamed.
  - **Annotated M10 unit 01 (3.0.0, unreleased): this passes, and it does not
    test what it reads as testing.** The matcher pairs **any** sole remaining
    unmatched sheet on each side as a rename, whether or not the content
    matches — see §6's correction. A fixture whose content *differs* would
    satisfy this criterion equally. It is met; it is not evidence that content
    is compared, because nothing compares content.
- One renamed sheet with cell changes still produces cell diffs.
- Multiple ambiguous rename candidates do not produce arbitrary confident matches.
- Added and removed sheets remain correctly represented.

## 10. Migration and compatibility

v1 consumers saw remove+add for renames. v2 consumers should update UI logic to show `Renamed` and display cell diffs under the logical pair.

## 11. Open questions

- Should `Ambiguous` be a `SheetChange` variant or only a diagnostic?
- How much content sampling is acceptable for default v2.0 matching?
