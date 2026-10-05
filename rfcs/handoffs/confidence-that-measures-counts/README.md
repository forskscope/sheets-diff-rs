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

## The three findings

| | Finding |
|---|---|
| **A-01** | `confidence` can be `Exact` for a pairing the engine's own warning calls ambiguous, and for one it reports nothing about. It measures counts, not pairing quality. |
| **A-02** | `row_signature_alignment` takes `_diagnostics` and emits none. **The whole signature mode is diagnostic-silent** — no duplicate warning, no ambiguity warning, nothing, ever. |
| **A-03** | `AlignmentSummary.confidence` has no field doc. `MatchConfidence`'s only doc says *"How confident the **sheet-matching** algorithm is about a non-exact pairing"* — a different subject (RFC-009 sheet matching), and no variant carries a doc. Nothing tells a consumer what `Exact` claims about rows. |

A-01 is instance **ten** of this quarter's pattern — a marker credited with more than it measured —
and A-03 is why it survived: a value nothing documents is a value nothing can contradict.

## Who this reaches

ForskScope's stated plan is to run `Positional`, and on a cascade also run **`RowSignature`**, keep
whichever reports fewer changed cells, and tell the user which alignment was kept. That is A-02's
mode: the one that never warns. If any part of their user-facing story leans on our diagnostics,
there are none on that path — and the confidence it reports alongside can be `Exact` on a guess.

**Tell them before this ships, not after.** The letter of 2026-10-06 does.

## The two units

| Unit | What | Semver |
|---|---|---|
| **01** | `confidence` must not claim more than the pairing supports | behaviour change — see below |
| **02** | the signature mode emits no diagnostics | minor (a new `DiagnosticKind` is additive) |

**01 first.** 02 depends on deciding what the signature path should warn about, which 01's work
establishes.

**The semver call on 01 is the owner's.** It is not an API change — `MatchConfidence` is
`#[non_exhaustive]` and no signature moves — but it makes a reported value more conservative, and a
consumer may be branching on `Exact`. No golden moves: the corpus compares under default options,
and `alignment_summary` is `None` for `Positional`. My read is **minor**, not patch, because a
consumer who tests for `Exact` will see different behaviour; stated in the unit, decided by the
owner.

## Standing rules

Rule 002 (one scratch target dir per sweep, deleted after), rule 003 (sweep both manifests —
`./Cargo.toml` and `./fuzz/Cargo.toml`, the latter only via
`cargo check --manifest-path fuzz/Cargo.toml --bins`). Do not commit; review first.
