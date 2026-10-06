# Formulas move with their rows

**Added 2026-10-06.** **Authorised by the owner 2026-10-06** on the architect's recommendation:
accept, staged, additive, 3.6.0. **Two units.** Reasoning and the measurement:
`.git-exclude/decisions/007-formula-references-and-moved-rows.md`.
**Requested by ForskScope**, 2026-10-06, with the definition already checked on their side.

## The defect

Formulas are compared as **raw text**, and Excel rewrites row references when rows move. So any
alignment that pairs a row across a shift reports that row's formulas as changed. Measured on 3.5.0,
200 rows keyed on column A, one row inserted at row 3, `RowKey`:

| column D | positional cells | aligned cells | of which formula changes |
|---|---|---|---|
| a plain number | 796 | **4** | 0 |
| the row-relative `=C{r}*2` | 598 | **202** | **199** |

The truth is about three changed cells. **Alignment does not remove the cascade on this shape, it
exchanges one for another:** positional compares row 5 with row 5 and gets formulas right and values
wrong; alignment the reverse. A formula column over the data is close to the most ordinary shape a
sheet has, and `AlignmentMode::RowSignature`'s own doc claims it *"reduces cascades after row
insertion/deletion"*.

## The one rule that governs both units

**Annotate. Never suppress.**

Today we **over**-report formula changes: noise, visible, irritating. A comparison that decided two
formulas were equal would **under**-report — a real formula change hidden because we assumed the row
shift explained it. That is the silent-wrong-answer class this project spent a quarter removing, and
it would arrive as the fix for a noise problem.

So the reported set of `cell_diffs` **does not change**. A consumer who wants the cascade gone filters
on the annotation. And the annotation is produced **only when the whole formula is understood and
every reference maps** — otherwise we annotate nothing and behave exactly as today. The failure mode
is "no annotation", which is current behaviour, rather than "wrong annotation".

Note this is the **opposite** of the conservative direction `confidence-that-measures-counts/01`
chose, and right for the same reason: there, claiming too much confidence was the danger; here,
claiming too much understanding is. Pick the direction by which mistake is silent.

## The two units

| Unit | What | Public API |
|---|---|---|
| **01** | the tokenizer and the row-reference mapper, internal, refusal-first | **none** |
| **02** | `FormulaChange` gains the annotation, wired to unit 01 | additive (minor) |

**01 first, and it is the whole risk.** It has no public surface, so it can be proven in isolation and
is contained if it is wrong. 02 is the easy half.

## Why this is a tokenizer and not a regular expression

Two one-line demonstrations, both verified:

```
=LOG10(C5)        /[A-Z]+[0-9]+/ matches ['LOG10', 'C5']
=IF(A1="B2",1,2)  /[A-Z]+[0-9]+/ matches ['A1', 'B2']
```

`LOG10` reads as column `LOG`, row 10 — a naive mapper rewrites a **function name** to `LOG11`. And
`B2` there is inside a **string literal** and must not move. Either mistake produces a wrong mapped
text, which produces a wrong annotation, which is the one outcome the rule above forbids.

## Stage 2, explicitly not in scope

Cross-sheet references (`Sheet2!C5`, `'My Sheet'!C5`) must map through **that** sheet pair's mapping,
which may not exist if that pair aligned positionally. Structured/table references, whole-column and
whole-row forms, R1C1, and locale argument separators are stage 2 as well. **Stage 1 refuses all of
them**, which is the correct behaviour for stage 1 and not a gap.

## Standing rules

Rule 002, rule 003, and **rule 005: both units require a proposal before implementation.** Unit 01's
is the refusal list; unit 02's is the enum. Do not commit; review first.
