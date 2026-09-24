# Handoff 01 — A reorder reports no difference, and a rename renders none

**Governing RFCs.** RFC-013 (output formatters, CLI and exit codes), RFC-006
(sheet matching); ROADMAP §6 (a missed difference is an integrity failure)
**Roadmap.** M8 — first unit
**Sequence.** After 2.5.1 ships. Before every other M8 unit.

## Purpose

The engine detects that two workbooks differ and the CLI tells the user they do
not. Close both halves of that.

## Background

Dev-team task 001 found this and it reproduces. **Scoping it found a second,
independent defect**, so treat the finding as two.

### Defect 1 — a pure reorder exits 0

Swap two `<sheet>` entries in `workbook.xml`, touch no cell:

```
sheets : 0 added, 0 removed, 0 renamed, 0 changed
cells  : 0 changed (0 value, 0 formula)
sheet 'Alpha' [moved]: 0 cell(s) changed
sheet 'Beta'  [moved]: 0 cell(s) changed
exit=0
```

`src/main.rs:146-150` tests `cells_changed`, `sheets_added`, `sheets_removed`
and `sheets_renamed`. **`sheets_moved` is absent**, and so is `sheets_changed` —
the latter benign today only because a modified sheet raises `cells_changed`.

Note the summary's own contradiction: the headline says `0 changed` directly
above two `[moved]` lines.

### Defect 2 — a pure rename exits 1 and renders nothing

Rename a sheet, touch no cell:

```
  sheets : 0 added, 0 removed, 1 renamed, 0 changed
  sheet 'AlphaRenamed' [renamed]: 0 cell(s) changed
  exit=1

  --- old.xlsx
  +++ renamed_only.xlsx      ← the entire unified output
  exit=1
```

**The exit code and the renderer contradict each other in one invocation.**
`src/output/text.rs`:

```rust
let is_structural = matches!(sd.change, SheetChange::Added | SheetChange::Removed);
if !has_cell_changes && !is_structural { continue; }
```

Any sheet without cell changes is skipped unless it was added or removed — so
`Moved`, `Renamed` and `RenamedAndMoved` are dropped. The function's own
`[renamed: 'x' → 'y']` branch sits below that guard and **can only execute when
the sheet also has cell changes**, which is a contradiction inside one function.
`Moved` has no branch at all.

### Why this is an integrity matter, not a formatting one

ROADMAP §6 and the threat model both say a silently missed difference is a
data-loss path treated at the same severity as a crash. A consumer piping
`--format unified`, or branching on the exit code, is told two workbooks are
identical when the engine knows they are not.

The engine is right throughout: `SheetChange::Moved` exists,
`DiffSummary::sheets_moved` counts it, and `sheet_reordered` is a corpus
scenario. Only the surfaces are wrong.

## Change scope

`src/main.rs` (the exit condition), `src/output/text.rs` (both renderers),
`tests/cli.rs`, `tests/integration.rs`, `docs/src/migration/v1-to-v2.md`
(the exit-code table), `CHANGELOG.md`.

## Non-change scope

- **Do not change the engine.** `SheetChange`, `DiffSummary` and the matcher are
  correct. If you believe one is wrong, stop and report.
- **Do not change what `render_summary` already prints per sheet.** The
  `[moved]` / `[renamed]` lines are right; the headline counts above them are
  not.
- The fixture corpus must not move. Goldens are the serialised model, and the
  model does not change — **if one moves, stop and report**, because that means
  you changed the engine.

## Required implementation

1. **Exit 1 when the workbooks differ in any way the engine records.** Add
   `sheets_moved` and `sheets_changed` to the condition. Derive it from the
   summary rather than listing fields if you can do so without reaching into the
   engine — a future field should not silently reintroduce this.
2. **`render_unified` must render a sheet whose only change is structural.**
   Fix the guard so `Moved`, `Renamed` and `RenamedAndMoved` survive it, and add
   the missing `Moved` branch. Decide and state the marker text; `[moved]` reads
   consistently with the summary renderer.
3. **`render_summary`'s headline must not contradict its own body.** It prints
   `0 changed` above two `[moved]` lines. Report moved sheets in the counts.
4. **Update the migration guide's exit-code table.** It documents 0/1/2/3;
   what makes a `1` is changing.

## Required tests

1. **A pure reorder exits 1** — CLI subprocess test, `tests/cli.rs`.
2. **A pure reorder renders a moved sheet in unified output**, and **a pure
   rename renders a renamed one.**
3. **A genuinely identical pair still exits 0 and still renders nothing.** This
   is the guard against fixing defect 1 by making everything exit 1.
4. **Each demonstrated failing before the fix**, by removing the specific
   condition or guard — not by reverting the file.

Both fixtures must be built in-test, as the readiness review did by patching
`workbook.xml` in a copy of `sheet_reordered/old.xlsx`. `tests/support.rs` has
`patch_xlsx_xml`. **Do not add corpus scenarios** — these are CLI and renderer
tests, not comparison scenarios.

## Acceptance criteria

1. A pure reorder exits 1; a pure rename exits 1; an identical pair exits 0.
2. `render_unified` renders moved, renamed and renamed-and-moved sheets that
   have no cell changes.
3. `render_summary`'s headline counts agree with the per-sheet lines beneath.
4. Every test above is demonstrated failing before the fix, by removing the
   specific code under test, with transcripts.
5. No engine change; fixture corpus byte-identical.
6. The migration guide's exit-code table matches the new behaviour.
7. CHANGELOG under `### Changed`, stating the contract change plainly: a
   comparison that exited 0 for a reordered workbook now exits 1.
8. Gates green, full matrix, including MSRV doctests.

## Prohibited shortcuts

- **Do not make the exit condition `!= Default::default()` on the whole
  summary.** `diagnostics` is in there, and an `Info` diagnostic is not a
  difference. Be explicit about which fields mean "differs".
- Do not fix `render_unified` by removing its guard entirely — a sheet with no
  change at all should still produce no hunk.
- Do not delete the `[renamed: …]` branch because it is currently unreachable.
  It is unreachable *because of the guard*; the guard is the bug.

## Compatibility constraints

**This is a CLI contract change and the reason M8 is 2.6.0 rather than 2.5.2.**
A script treating exit 0 as "identical" will now see 1 for a workbook whose
sheets were reordered. That is the correct answer and it is still a change; say
so in the CHANGELOG rather than describing it only as a fix.

## Known risks

- **`docs/src/semantics.md`'s examples execute and assert on real output**
  (M6 unit 03). If any asserts on rendered output for a renamed or moved sheet,
  it will move — that is the documentation doing its job. Update it and say so.
- The diagnostics-only case — defined names and sheet visibility differ, surfaced
  as `Info`, exit 0 — is **deliberately out of scope**. It is a genuine question
  (is a diagnostic a difference?) and it is not this unit's. Report it if you
  reach a view; do not act on it.

## Required evidence

- The pre-fix failure transcripts, one per test
- Reorder, rename and identical-pair runs, both formats, with exit codes
- Corpus byte-comparison
- CI run link

## Review request format

Per development policy §9.2, plus the before/after CLI output for all three
cases.
