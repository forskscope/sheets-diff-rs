# Confidence that measures counts

**Added 2026-10-06.** **Scoped by:** the architect.
**Found via:** ForskScope's letter of 2026-10-06 §1, which named the condition abstractly as a
future risk. **It is not a future risk. A shipped mode meets it today.**

## What they said

Reviewing `RowPlacement`, they agreed per-row confidence belongs on the sheet, with one condition:

> *"Sheet-level confidence is right while a mode's pairing quality is a property of the **mode and
> the data as a whole**, which is true of the three modes you ship. It would stop being right if a
> future mode paired some rows exactly and others by guesswork **within one sheet** — then a single
> sheet-level value would be averaging two different claims… if you ever add such a mode, the shape
> to re-examine is the sheet field, and this enum will look like the victim rather than the cause."*

Their instinct about **where** to look was right. Their premise — that it is true of the three modes
we ship — is not. `RowKey` and `RowSignature` both pair some rows exactly and others by position
within one sheet, and the sheet-level value does not average the two claims: **it reports the
higher one.**

## What was measured, 2026-10-06, on 3.3.0

**`RowKey`, duplicate keys on both sides, every row paired:**

```
old:  A/x   A/y                 AlignmentMode::RowKey { columns: vec![1] }
new:  A/y   A/x                 (the two rows swapped)
```

```
alignment_summary = AlignmentSummary { inserted_rows: 0, removed_rows: 0,
                                       matched_rows: 2, confidence: Exact }
diag duplicate_alignment_key : duplicate alignment keys detected (1 distinct key(s)
      repeated in old, 1 in new); LCS matching may pair rows ambiguously among duplicates
change at B1 (Modified)
change at B2 (Modified)
```

The sheet reports **`Exact`** while carrying its own warning that the pairing *"may pair rows
ambiguously among duplicates"*, and the two changes it reports are both spurious — the rows were
reordered, and a key-aware pairing would report nothing changed.

**`RowSignature`, two rows with the same sampled signature, swapped:**

```
old:  A/x   A/y                 RowSignature { sample_columns: Some(vec![1]) }
new:  A/y   A/x
```

```
alignment_summary = AlignmentSummary { inserted_rows: 0, removed_rows: 0,
                                       matched_rows: 2, confidence: Exact }
sheet diagnostics = 0
change at B1 (Modified)
change at B2 (Modified)
```

`Exact`, two spurious changes, and **no diagnostic at all.**

## Why

`src/align.rs:361`:

```rust
/// `Exact` when every row on both sides was matched; `High` when matched rows outnumber the rest;
/// `Medium` otherwise.
fn confidence_for(n_matched: usize, n_removed: usize, n_inserted: usize) -> MatchConfidence
```

**It sees three counts and nothing else.** It cannot distinguish a row paired by a unique key from a
row paired because LCS had two identical keys and took them in order. "Every row was matched" is
true in both cases, so both are `Exact`.

The principle was already written down, one clamp away. f130 added this at `src/align.rs:251`, for
keyless rows:

```rust
// `High` says the pairing is reliable apart from a few real insertions and removals.
// Rows placed by their content alone, or not at all, are neither — so a sheet with any
// keyless row is at most `Medium`, however many of them paired.
```

**Rows paired ambiguously among duplicates are placed by their position alone.** The same sentence
covers them. The clamp was never extended, and the signature path has no clamp at all.

## The four findings

| | Finding |
|---|---|
| **A-01** | `confidence` can be `Exact` for a pairing the engine's own warning calls ambiguous, and for one it reports nothing about. It measures counts, not pairing quality. |
| **A-02** | `row_signature_alignment` takes `_diagnostics` and emits none. **The whole signature mode is diagnostic-silent** — no duplicate warning, no ambiguity warning, nothing, ever. |
| **A-03** | `AlignmentSummary.confidence` has no field doc. `MatchConfidence`'s only doc says *"How confident the **sheet-matching** algorithm is about a non-exact pairing"* — a different subject (RFC-009 sheet matching), and no variant carries a doc. Nothing tells a consumer what `Exact` claims about rows. |
| **A-04** | **`Medium` is a catch-all for unrelated claims, so it cannot be documented as it stands.** Measured below. Unit 01 would add a third claim to it. |

### A-04, measured 2026-10-06

`RowKey { columns: vec![1] }`, three sheets:

| sheet | `inserted / removed / matched` | changes | `confidence` |
|---|---|---|---|
| every row keyed, every row matched | 0 / 0 / 2 | 0 | **`Exact`** |
| one keyless spacer row, otherwise identical | 0 / 0 / 2 | 0 | **`Medium`** |
| every row keyed, nothing matched | 2 / 2 / 0 | 8 | **`Medium`** |

Two consequences, both consumer-visible:

- **`confidence` is not a function of the three counts printed beside it.** Rows one and two have
  identical counts and different confidences, so a consumer cannot derive it, check it, or explain
  it from the summary it arrives in.
- **`Medium` means either "some rows were placed by content alone" or "barely anything matched".**
  Rows two and three are the two most dissimilar outcomes in the table and they share a value. A
  consumer branching on `Medium` cannot know which it got.

So ForskScope's request — *"if the fix gives `Medium` a stated meaning rather than a position in an
ordering, that is the part we would read first"* — **cannot be satisfied by writing a doc comment.**
`Medium` has no single meaning to state. That is a design finding, and it changes unit 01's scope:
see its *Required implementation* 5.

### The precedent for the fix is already in the file

Sheet matching ships **two** values: `MatchConfidence`, and `SheetMatchReason` *"set alongside"* it,
whose variants carry actionable prose — *"Nothing positive links the two sheets… treat it as the
weakest kind of rename."* Row alignment ships the confidence alone, undocumented. The asymmetry is
the defect and the fix direction is the sibling.

A-01 is instance **ten** of this quarter's pattern — a marker credited with more than it measured —
and A-03 is why it survived: a value nothing documents is a value nothing can contradict.

## Who this reaches

ForskScope's stated plan is to run `Positional`, and on a cascade also run **`RowSignature`**, keep
whichever reports fewer changed cells, and tell the user which alignment was kept. That is A-02's
mode: the one that never warns. If any part of their user-facing story leans on our diagnostics,
there are none on that path — and the confidence it reports alongside can be `Exact` on a guess.

**Tell them before this ships, not after.** The letter of 2026-10-06 did, and their reply changed
their own design: their tie-breaker was *"keep whichever reports fewer changed cells"*, and they
realised reading our measurement that **the rule is biased toward mis-pairings** — `RowSignature`
pairs by similarity, so a mis-pairing produces a *small* cell diff by construction, and "fewer cells
wins" preferentially selects it. They will now gate the cascade on `confidence`.

So A-01 is not a quality improvement for them. **It is a precondition for their feature being
correct**, and the two signals that could have caught the bias are exactly A-01 and A-02.

## The two units

| Unit | What | Semver |
|---|---|---|
| **01** | `confidence` must not claim more than the pairing supports, and must say what it claims | **minor** — it adds public API |
| **02** | the signature mode emits no diagnostics | minor (a new `DiagnosticKind` is additive) |

**01 first.** 02 depends on deciding what the signature path should warn about, which 01's work
establishes.

**The semver question is settled: minor.** A-04 means unit 01 cannot be a doc fix — `Medium` has no
single meaning to document, so the unit adds a reason value alongside `confidence`, following
`SheetMatchReason`. That is a public API addition, which makes it a minor whatever one concludes
about the behaviour change. No golden moves: the corpus compares under default options and
`alignment_summary` is `None` for `Positional`.

The behaviour change is also now known to be safe for the one consumer who has told us what they
run. They will branch on `confidence`, have no deployed behaviour depending on today's values, and
**want it more conservative** — *"a sheet that stops saying `Exact` is a sheet we would stop
trusting, which is the outcome we need."*

## Standing rules

Rule 002 (one scratch target dir per sweep, deleted after), rule 003 (sweep both manifests —
`./Cargo.toml` and `./fuzz/Cargo.toml`, the latter only via
`cargo check --manifest-path fuzz/Cargo.toml --bins`). Do not commit; review first.
