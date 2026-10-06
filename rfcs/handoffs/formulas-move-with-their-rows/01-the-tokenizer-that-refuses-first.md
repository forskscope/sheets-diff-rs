# Handoff 01 — The tokenizer that refuses first

**Unit:** formulas-move-with-their-rows 01. **Added 2026-10-06.** **Scoped by:** the architect.
**Public API: none.** Internal only; unit 02 wires it up.
**Read `README.md` in this directory first** — it carries the measurement and the annotate-never-
suppress rule that governs both units.
**Rule 005: propose the refusal list before implementing.**

## Purpose

One internal function. Given a formula's raw text and a row mapping, return the text with every row
reference mapped — **or refuse**, returning nothing, when the formula is not fully understood.

```rust
/// `None` means "refused": some part of `raw` was not confidently classified, so no mapped form is
/// offered and the caller must behave as though this function did not exist.
fn map_row_references(raw: &str, mapping: &RowMapping) -> Option<String>
```

The signature is a suggestion; the **contract is not**. Refusal must be representable and must be the
default for anything unrecognised.

## Why refusal is the feature

A wrong mapped text becomes a wrong annotation in unit 02, and a wrong annotation tells a consumer a
formula did not change when it did. **Refusing is always safe**: unit 02 then reports exactly what
3.5.0 reports. So the function's quality is measured by what it declines, not by what it maps.

Two verified hazards, which are the whole argument against a regular expression:

```
=LOG10(C5)        /[A-Z]+[0-9]+/ matches ['LOG10', 'C5']    <- LOG10 would become LOG11
=IF(A1="B2",1,2)  /[A-Z]+[0-9]+/ matches ['A1', 'B2']       <- B2 is inside a string
```

## Required implementation

**1. Propose the refusal list first** (rule 005,
`.git-exclude/proposal/formula-tokenizer-01/README.md`). State, as two explicit lists:

- **what stage 1 maps:** my intent is plain A1 references (`C5`), absolute and mixed (`$C$5`, `C$5`,
  `$C5`), and ranges of those (`C5:C10`);
- **what stage 1 refuses, by name:** my intent is any formula containing a `"`, a `!`, a `[`, a
  whole-column or whole-row form, an R1C1-looking reference, or any identifier token that is not a
  recognised A1 reference. **Refuse the whole formula, not the token** — a formula we only partly
  understand is one we do not understand.

**Argue with either list if you disagree.** In particular: refusing every formula containing a `"` is
blunt, and the alternative is delimiting string literals properly, which is more capable and more
risk. Say which you would build and why. I lean blunt for stage 1.

**2. Tokenize on boundaries, never by substring search.** A reference token must be bounded by
formula syntax — operators, parentheses, separators, start or end — so that `LOG10` is one identifier
token and not `LOG` + `10`. Report which boundary set you used.

**3. Map through the mapping, not by arithmetic.** For each reference's row, look it up in the
alignment's `matched` map. **Do not compute an offset.** A reference to a row *above* the edit does
not move, and one below it does; only the mapping knows which. A reference to a row that is **not**
in `matched` — an unpaired row — has nowhere to go: **refuse the formula.** (Excel makes those
`#REF!`; unit 02's doc will say that reporting such a cell as changed is correct.)

**4. Preserve everything else byte for byte.** Whitespace, case, `$` placement, argument separators.
The mapped text is compared for equality against the new formula's raw text, so any normalisation you
perform silently changes the comparison. **Map rows; touch nothing else.**

## Non-change scope

- **No public API. No `pub` item.** If you need one, stop and say so.
- Do not change `compare_formulas`, `FormulaChange`, or anything a caller can see. Unit 02 does that.
- Do not normalise formulas. The removed `FormulaCompareMode::NormalizedText` is not being
  resurrected; this is alignment-aware reference mapping, which is a different operation.
- Do not implement cross-sheet references, structured references, whole-column forms or R1C1.
- Do not make the function infallible. Refusal is the contract.

## Required tests

Unit tests beside the code. **The refusals matter more than the mappings**, so write them first.

**Refusals (each must return `None`):**
1. `=LOG10(C5)` — the function-name hazard. **If this maps, the unit has failed.**
2. `=IF(A1="B2",1,2)` — a reference-shaped string literal.
3. `=Sheet2!C5` and `='My Sheet'!C5` — stage 2.
4. `=SUM(C:C)` and a whole-row form — stage 2.
5. `=Table1[Amount]` — structured reference.
6. `=MyName*2` — a defined name.
7. A reference to a row absent from `matched`.
8. **A formula that is half-understood** — one mappable reference and one refused construct, for
   example `=C5+Sheet2!D5`. Refusal must win.

**Mappings (each must produce the exact expected text):**
9. `=C5*2` with old row 5 → new row 6 gives `=C6*2`.
10. A reference **above** the edit, which does not move: same text out, and **not** refused.
11. `$C$5`, `C$5`, `$C5` — the `$` is preserved and the row still maps. State in a comment that `$`
    does not prevent movement on insertion; it governs copy and fill.
12. `=SUM(C5:C10)` — both endpoints map.
13. Multi-digit and multi-letter: `=AB123+AB7`.
14. Whitespace and case preserved exactly: `= c5 * 2` out unchanged but for the row.

**Properties:**
15. **Mapping is a no-op when the mapping is identity.** For a mapping where every row maps to
    itself, every accepted formula comes back byte-identical. A violation means normalisation crept
    in.
16. A property test over generated formulas built from the accepted grammar: every output, re-mapped
    through the inverse mapping, returns the input. Report whether you could build the inverse
    cleanly; if not, say so rather than forcing it.

## Acceptance criteria

1. The refusal list and the mapped list, proposed and agreed before implementation.
2. Refusal is the default: an unrecognised construct anywhere refuses the whole formula.
3. All sixteen tests, refusals first.
4. Boundary-based tokenization, with the boundary set reported. **A substring regex is a fail.**
5. Mapping via the `matched` map, never by offset arithmetic.
6. Byte-for-byte preservation of everything but the mapped rows, pinned by tests 14 and 15.
7. `cargo public-api` diff **empty**. No `pub` item added.
8. Gates green, rule 003, one scratch dir, deleted. Nothing committed.

## Prohibited shortcuts

- **No substring regex over the formula text.** Tests 1 and 2 exist to catch it, and they are the two
  cases a regex gets wrong.
- Do not map a token you are not sure of. Refuse.
- Do not compute row offsets.
- Do not normalise, trim, re-case or reformat anything.
- Do not widen to a stage-2 construct because it "looks easy". Each needs its own fixtures and a
  decision about whose mapping applies.

## Known risks

**1. The accepted grammar is the whole design, and it is small on purpose.** The temptation is to
handle one more construct. Resist it: every construct you accept is one you must be right about
forever, and unit 02 cannot tell a wrong mapping from a right one.

**2. A defined name can look exactly like a reference.** `=LOG10(C5)` is the demonstrable case;
`=TAX2*C5` is another, where `TAX2` is a name. Stage 1 refuses any identifier that is not a
syntactically valid A1 reference — but **`LOG` and `TAX` are valid column letters**, so "valid A1
reference" is not sufficient on its own and the boundary analysis is what saves you. Say in the
review how you distinguish `C5` from `TAX2`, because the answer is not obvious and I do not have it
either. If the honest answer is "I cannot, so I refuse any identifier immediately followed by `(`
and accept the rest", say that and name what it still gets wrong.

**3. `matched` maps old rows to new rows.** The formula being mapped is the **old** cell's, so the
direction is old → new. Getting the direction backwards passes test 9 only if the mapping is
symmetric; test 10's above-the-edit case is what catches it.

## Required evidence

Under `.git-exclude/review-request/formula-tokenizer-01/evidence/`:

1. The proposal and my reply.
2. The two lists as implemented, and the boundary set.
3. All sixteen tests with their output.
4. Your answer to *Known risks* 2 — how `C5` is told from `TAX2`, and what the rule still gets wrong.
5. `cargo public-api` showing an empty diff.
6. Gate sweep, rule 003, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/formula-tokenizer-01/README.md`, with:

- The refusal list as shipped, and anything you added to it while implementing.
- How you distinguish a reference from a same-shaped identifier, stated plainly.
- Which of the sixteen tests you found hardest to make pass, and why.
- **Anything you mapped that you are not certain about.** A list of "accepted but I am not sure" is
  more useful to me than a clean report, because unit 02 turns each of those into a claim to a
  consumer.
