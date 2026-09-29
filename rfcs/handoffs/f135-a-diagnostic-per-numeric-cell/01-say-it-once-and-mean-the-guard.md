# Handoff f135-01 — One `Info` per numeric cell, on by default, bounded by nothing

**Defect response. Promoted out of M9 unit 04 ahead of the rest of M9.**
**Written:** 2026-09-30. **Found by:** the owner asking whether M9's incompletion affects our
consumer. It does, and this is the item.

## Purpose

A plain numeric workbook with **no formulas in it** produces one `Info` diagnostic per numeric cell,
per side, by default, with nothing bounding the total. Measured: a **666 KiB** workbook compared with
itself yields **400,000 diagnostics** and a **158.9 MiB** JSON result, against **1,870 bytes** with
the option turned off. Make it say the thing once, and make the guard mean what it is named.

## Background

**Measured 2026-09-30 before writing this**, 20,000 rows × 10 columns of plain numbers, no formula
anywhere, `compare_bytes(w, w)`:

| `include_formula_cached_values` | diagnostics | JSON result |
|---|---:|---:|
| `true` (**the default**) | **400,000** | **166,575,526 bytes (158.9 MiB)** |
| `false` | 0 | 1,870 bytes |

Roughly **89,000× the output**, from a 666 KiB input, with no formulas present.

**Two separate faults, and the second is why the first is reachable.**

**1. The diagnostic is per cell.** `src/diff.rs:774`, inside `for (&(row1, col1), cell) in &cells`,
pushes one `Diagnostic` for every numeric cell without formula text. Each carries a cloned sheet
name and a formatted message. **Nothing bounds the diagnostics vector** — `max_diffs_returned`
bounds diffs, and there is no `max_diagnostics`.

**2. The guard does not mean what it is named.** The loop is gated on `has_formulas`, set at `:736`:

```rust
let has_formulas = match wb.reader.worksheet_cells_reader(&sheet.name) {
    Ok(mut reader) => loop {
        match reader.next_formula() {
            Ok(Some(cell)) => { /* ... collect ... */ }
            Ok(None) => break true,        // <- end of stream
            Err(_) => { formulas.clear(); break false }
        }
    },
    Err(_) => false,
};
```

`Ok(None)` is **end of stream**, which every readable sheet reaches. So `has_formulas` means *"the
formula pass completed without error"*, not *"this sheet has formulas"*, and the gate is true for
essentially every sheet.

**The comment immediately above the loop says what was intended:**

> *"Not every numeric cell is a formula; this is expected and not worth more than Info — **don't spam
> warnings on plain data sheets**."*

That is exactly what it does. **This is the fifth instance of the category ForskScope named for us —
a marker credited with more than it measured** — after `ContentSimilarity`, `cells_read`, the
"~15 ms", and the assurance row that cited a mutating fuzz run after it stopped mutating. It is the
first of them whose consequence is a resource cost rather than a wrong description.

**Who this reaches.** Default-on, so every caller who does not know to turn it off. A consumer
comparing ordinary numeric spreadsheets and serialising the result is the worst case, which is
precisely our known consumer's shape.

## Change scope

- `src/diff.rs` — the guard and the diagnostic.
- `src/options.rs` — only if a bound is added (see *Required implementation* 3; **prefer not**).
- `tests/` — regression tests.
- `CHANGELOG.md`, `docs/src/maintainers/threat-model.md` — a resource cost that was not recorded.
- `rfcs/handoffs/m9-reaching-the-code/README.md` — unit 04 is this, done here.

## Non-change scope

- **Do not remove the diagnostic.** It tells a caller something true when the case is real. The
  defect is that it fires when the case is not real, and that it fires per cell.
- **Do not change the default of `include_formula_cached_values`.** Turning a comparison feature off
  to fix a diagnostic's volume would be fixing the wrong thing, and it is a public default.
- Do not rename `DiagnosticKind::FormulaUnavailable`; the kind is right.
- Do not touch the formula pass itself (`:736`–`:759`) beyond what the guard needs. Attaching
  formulas works; only the flag's meaning is wrong.

## Required implementation

**1. Make the guard mean what it says.** The condition should be that the sheet *has* formulas —
`!formulas.is_empty()` before they are drained, or an equivalent that survives the `for (key, text)
in formulas` loop that consumes the vector. **Rename it** to whatever it actually tests
(`formula_pass_succeeded` is what the current value means; the new one wants `sheet_has_formulas` or
similar). The old name is how this survived.

With that alone, the measured case above emits **zero** diagnostics, because the sheet has no
formulas — which is the whole of the comment's intent.

**2. Say it once per sheet, not once per cell.** Even on a sheet that genuinely has formulas, a
caller does not want one `Info` per numeric cell. **Measure the genuine case first** — a real
formula-bearing sheet, a realistic ratio of formula to plain numeric cells — and report the count
before deciding the shape. My expectation, to be confirmed or contradicted by your measurement: one
diagnostic per sheet carrying a **count**, with `address: None`, is the right answer, and the
per-cell addresses are not worth 400,000 entries.

**If your measurement says per-cell addresses are genuinely useful** at realistic scale, say so and
propose the alternative — a bounded number of per-cell entries plus a summary. **Do not implement
both shapes; pick one, argue it, and let me rule** if you are not certain.

**3. Do not add a `max_diagnostics` limit.** A cap on the symptom is not the fix, and it is public
API forever. If after 1 and 2 you still think an overall bound on diagnostics is wanted for its own
sake, that is a separate proposal, not this unit.

## Required tests

- **The measured case, as a test**: a plain numeric workbook with no formulas emits **zero**
  `FormulaUnavailable` diagnostics. This is the regression.
- **A sheet that genuinely has formulas still reports**, in whatever shape 2 settles on — the
  diagnostic must not become unreachable. A guard fix that silences the true case too is a worse
  defect than the one being fixed.
- **The count is bounded by sheets, not cells**: assert that a sheet with N numeric cells produces a
  number of diagnostics that does not grow with N.
- **Failing first**, by restoring the `Ok(None) => break true` semantics (one expression), with a
  `cmp`-verified restore.
- Goldens: **expect some to move**, and check each before regenerating — see Known risk 1.

## Acceptance criteria

1. A no-formula workbook produces zero `FormulaUnavailable` diagnostics; demonstrated failing-first.
2. A formula-bearing workbook still produces the diagnostic.
3. Diagnostic count does not grow with the number of numeric cells.
4. The JSON result for the measured 20,000 × 10 case is within a small factor of the
   option-off size (1,870 bytes), not 158.9 MiB. Report the figure.
5. The flag is renamed to what it tests.
6. No new public API. `cargo public-api --simplified diff 3.2.0` shows nothing unexpected.
7. Goldens: each moved golden accounted for individually, in the review request.
8. Gates as always, plus rule 003.

## Prohibited shortcuts

- **Do not fix this by flipping `include_formula_cached_values` to `false`.** That changes what a
  comparison *computes* to quiet a diagnostic.
- Do not cap with a magic number in place of fixing the guard.
- Do not delete the diagnostic because it is noisy. It is noisy because it is wrong.
- Do not regenerate goldens in bulk. Each one that moves is a statement about what we used to emit.

## Known risks

**1. Goldens will move, and that is the finding, not the obstacle.** Any fixture with numeric cells
has been emitting these. Every moved golden should lose `FormulaUnavailable` entries and change in no
other way. **If a golden changes in any other respect, stop and report it** — and if a fixture's
expected output *keeps* a `FormulaUnavailable` that your fix removes, check whether that sheet really
has formulas before assuming the fix is wrong.

**2. `has_formulas` is also used to decide something else.** Read its other uses before changing its
meaning; if the same flag gates formula *attachment* as well as the diagnostic, splitting it into two
values is the fix, not repurposing one.

**3. This is a behaviour change a caller can see.** Fewer diagnostics is what they want, but a caller
counting them, or asserting on them, will notice. It belongs in `### Changed` as well as `### Fixed`,
and the CHANGELOG should say a caller relying on per-cell `FormulaUnavailable` entries will get a
different shape.

## Required evidence

Under `.git-exclude/review-request/f135-01-say-it-once/evidence/`:

1. The before/after table for the measured case: diagnostics count and JSON size, both settings.
2. The genuine-formula-sheet measurement behind your choice in *Required implementation* 2.
3. The failing-first demonstration and `cmp` restore.
4. Every golden that moved, with the diff for each.
5. `has_formulas`'s other uses, quoted.
6. Public API diff; gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/f135-01-say-it-once/README.md`, with:

- Your measurement of the genuine case and the shape you chose, argued.
- Whether `has_formulas` gated anything besides the diagnostic.
- Each moved golden, accounted for.
- Your view on whether anything else in `src/` pushes a diagnostic inside a per-cell loop. This is
  the fifth marker we have found that did not measure what it was named for; if there is a sixth in
  this file, I would rather hear it now.
