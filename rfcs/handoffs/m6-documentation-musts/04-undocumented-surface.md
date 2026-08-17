# Handoff 04 — The undocumented public surface

**Governing.** F-D (M4 unit 02's review), F-G (M5 unit 04's review); NF-024's
spirit
**Roadmap.** M6
**Sequence.** Any. Independent of everything.

## Purpose

Two public types where the documented fields are outnumbered by the
undocumented ones, and where the undocumented ones are the misleading ones.

## Background

### F-D — `DiffMetrics` has one documented field of five

M4 unit 02 added a doc comment to `cells_compared`. The other four —
`sheets_read`, `cells_read`, `diffs_emitted`, `diagnostics_emitted` — have none.
The struct has a doc comment; the fields do not.

**`cells_read` is the one that matters.** On the `sparse_range` fixture it
reports **5200** against `cells_compared`'s **2**, because it counts every
physically visited cell including empty ones inside the used range. A consumer
reading the name would not predict a 2600× gap, and nothing tells them.

`cells_compared`'s new comment makes the silence beside it conspicuous.

### F-G — `ReadErrorKind`'s variants are undocumented, and one cannot occur

```rust
pub enum ReadErrorKind {
    SheetNotFound,
    MalformedSheet,
    Other,
}
```

No variant carries a doc comment. After M5 unit 03, `Other` is a public variant
that **nothing can produce** — the workbook reader is `Xlsx<Cursor<Vec<u8>>>`,
so sheet reads touch no I/O and the `XlsxError::Io` that maps to it cannot arise
at that stage — and it has a dedicated exit-code arm in `main.rs`.

That is M4 unit 01's defect class exactly: an unreachable public variant that
says nothing about being unreachable. Unit 01 established the wording, and the
milestone that closed it should not have quietly created a new one.

## Change scope

`src/model.rs` (`DiffMetrics` field docs), `src/error.rs` (`ReadErrorKind`
variant docs), `CHANGELOG.md`.

## Non-change scope

- **Doc comments only.** No behaviour change, no signature change, no variant
  added or removed.
- **Do not remove `ReadErrorKind::Other`.** M5 unit 04 established it as a
  conservative default and `exit_code_for` maps it deliberately. Unreachable is
  not unused.
- Do not change what any metric counts. If a field's name is wrong for what it
  counts, **stop and report** — that is M4's business, not a doc fix.

## Required implementation

1. **A doc comment on every `DiffMetrics` field.** Each must say what is counted
   and, where it could mislead, what is *not*. `cells_read` must state that it
   includes empty cells within the used range, and should give the concrete
   contrast — 5200 read against 2 compared on `sparse_range` — because the
   number is what makes it land.
2. **State the relationships that hold**, since they are what a consumer would
   otherwise infer wrongly: `cells_read >= cells_compared >= diffs_emitted`.
   This holds across all 19 corpus fixtures and was verified in M4 unit 02's
   review. Do not assert it without re-checking it yourself.
3. **A doc comment on every `ReadErrorKind` variant.** `SheetNotFound` and
   `MalformedSheet` describe when they occur. `Other` must state plainly that it
   cannot currently occur, why (the reader is cursor-backed, so no I/O happens
   at read time), and that it is retained as a conservative default rather than
   as a live case — reusing M4 unit 01's established wording rather than
   inventing new phrasing.
4. **Note `SheetNotFound`'s contingency where it is decided.** M4 unit 03's
   review established that mapping it to exit 3 is sound *only because the CLI
   has no sheet-selection flag*. M5's work left a comment on the match arm; the
   variant's own doc should carry the same fact, since a caller matching on the
   variant is not reading `main.rs`.

## Required tests

None beyond the gates — these are doc comments.

**But confirm claim 2 rather than copying it.** Re-derive
`cells_read >= cells_compared >= diffs_emitted` from the current corpus and say
so in the review request. A documented invariant asserted from a prior review's
summary rather than from the data is the thing this project keeps getting
wrong.

If any fixture violates it, that is a defect — stop and report.

## Acceptance criteria

1. All five `DiffMetrics` fields carry doc comments.
2. `cells_read`'s comment states that empty cells within the used range are
   counted, with the concrete contrast.
3. The `cells_read >= cells_compared >= diffs_emitted` relationship is
   documented, and the review request shows it re-derived from the corpus.
4. All three `ReadErrorKind` variants carry doc comments.
5. `Other`'s comment states it cannot occur, why, and why it is retained.
6. `SheetNotFound`'s comment records the no-sheet-flag contingency.
7. No behaviour change; doc comments only.
8. `cargo doc` produces no new warnings.
9. Corpus byte-identical.
10. CHANGELOG under `### Documentation`; gates green, full matrix.

## Prohibited shortcuts

- Do not write "the number of cells read" for `cells_read`. That is the field
  name in a sentence, and it is the misreading the comment exists to prevent.
- Do not hedge `Other` with "may occur in future versions" without saying it
  cannot occur now. M4 removed comments that named futures.
- Do not document the invariant without checking it.

## Known risks

- The stdout gate from M5 forbids `println!` in library code; a doc example
  inside these comments would be a doctest and is fine, but keep them short —
  field docs with worked examples usually want to be on the struct instead.

## Required evidence

- The diff
- The re-derived invariant across the corpus
- `cargo doc` output showing no new warnings
- CI run link

## Review request format

Per development policy §9.2, plus the invariant re-derivation.
