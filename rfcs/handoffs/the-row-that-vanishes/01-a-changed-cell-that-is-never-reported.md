# Handoff 01 — A changed cell that is never reported

**Unit:** the-row-that-vanishes 01. **Added 2026-10-05.** **Scoped by:** the architect.
**Severity: this is the worst class of defect this product can have** — a diff tool reporting no
difference where a cell changed. Take it before every other open unit.
**Semver: minor** — corrected 2026-10-06. The handoff first said "patch", which contradicted its
own acceptance criterion 7 allowing an additive `DiagnosticKind` variant; new public API is a minor
under RFC-031 §6.
**Release: 3.4.0**, decided by the owner 2026-10-06 (was 3.3.1 before the semver correction). It
carries this unit **and** `row-space-and-the-anchor/00` (false doc comments). **Separate reviews, one release** — do not
combine the two into one review request, and do not wait for 00 before submitting this.
**This unit goes first** of everything currently open.

## The defect

`AlignmentMode::RowSignature { sample_columns: Some(cols) }` **silently drops every change in a row
that has no cell in any sampled column.** Not mis-labels, not mis-places — does not report.

Measured on 3.3.0. Two sheets, row 2 has no cell in column A, and its column-B value changes:

```
old:  A="K", B="a"          new:  A="K", B="a"
      (no A),  B="before"          (no A),  B="AFTER"
```

| mode | inserted / removed / **matched** | confidence | diagnostics | cell diffs |
|---|---|---|---|---|
| `RowSignature { sample_columns: Some([1]) }` | 0 / 0 / **1** | **`Exact`** | **0** | **0** |
| `RowSignature { sample_columns: None }` | 1 / 1 / **1** | `Medium` | 0 | 2 — `B2 Removed`, `B2 Added` |
| `RowKey { columns: [1] }` | 1 / 1 / **1** | `Medium` | 1 | 2 — `B2 Removed`, `B2 Added` |

The column order is `AlignmentSummary`'s own field order — `inserted_rows`, `removed_rows`,
`matched_rows` — **not** matched-first. The first version of this table gave the three numbers under
a bare heading of "summary"; the consumer read them matched-first, re-measured, and reported a
discrepancy that does not exist. Three numbers without their names, in a thread about markers
credited with more than they measured. Label the order wherever these appear.

`before` → `AFTER` is a real change in a real cell. The first row of that table reports **nothing**:
no diff, no diagnostic, and an alignment summary claiming `Exact` — a positive assertion that the
pairing was perfect.

**It requires `sample_columns: Some(...)`.** With `None` the filter never runs and the row is
reported, as the second row shows.

## Why

`compute_row_signatures`, `src/align.rs:449`:

```rust
for ((r, c), cell) in cells {
    if let Some(cols) = sample_cols
        && !cols.contains(c)
    {
        continue;
    }
    rows.entry(*r).or_default().push(...);
}
```

The entry for a row is created **only** when one of its cells passes the filter. A row with no cell
in any sampled column therefore never enters the map, so it is not in `lcs_match`'s sequences, so it
is in none of `matched`, `removed` or `inserted`, so its cells are never compared.

**This is f130's defect, on the path f130 did not touch.** The `RowKey` side carries f130's fix and
its comment, `src/align.rs:190`:

> *"A row with no cell in any key column has no key, so it cannot be matched by key. **What it must
> not do is disappear**: `extract_row_keys` never sees it, so without this it would be in none of
> `matched`, `removed` or `inserted`, its cells would never be compared, and the summary would call
> the alignment exact (f130)."*

Every clause of that paragraph describes the signature path today, including "the summary would call
the alignment exact". The rescue — `keyless_rows` plus the pairing rule below it — was written for
one path and the other was left as it was.

## Change scope

- `src/align.rs` — the signature path's row set, and whatever rescue it needs.
- `src/model.rs` — only if a diagnostic variant is the answer; see *Required implementation* 3.
- `tests/integration.rs` and/or `src/align/tests.rs`.
- `examples/gen-fixtures.rs`, `tests/fixtures/generated/` — if a diagnostic lands, RFC-036 §5.4
  requires a scenario asserting it.
- `CHANGELOG.md`.

## Non-change scope

- **Do not change `confidence`.** `Exact` here is wrong, and it stops being wrong on its own once
  the row is no longer missing: the rescued rows become matched, removed or inserted and the counts
  change. `confidence-that-measures-counts/01` owns the claim itself. **If `Exact` survives your
  fix on this input, stop and report it** — that would mean two independent defects, not one.
- Do not change `RowKey`'s behaviour. It is correct here and is the control.
- Do not take on "does `RowSignature` ever warn about anything" — that is
  `confidence-that-measures-counts/02`. This unit covers the diagnostic for **this** case only,
  because silence is part of this defect. Say in the review what you did, so unit 02 does not
  collide with it.
- Do not touch `CellDiff`, `RowPlacement` or `output::view`.

## Required implementation

**1. Reproduce it first**, as a test, failing on current `main`. Assert the absence specifically:
`cell_diffs` is empty on that input and the change is therefore unreported. A test asserting only
the summary would pass once the counts move and would not pin the thing that matters.

**2. Make no change go unreported.** f130's rescue is the precedent and the shape: find the rows the
signature map never saw and give them somewhere to go. Read `src/align.rs:190`–`:210` before writing
anything — the RowKey rescue also decides *how* such rows pair (identical rows pair with one another
in row order, so an unchanged spacer does not become a removal plus an insertion), and whatever you
do here should be defensible against the same reasoning rather than merely non-lossy.

**Whether to share the mechanism with `keyless_rows` or write the signature analogue beside it is
yours**, but say which and why. If it can be shared, this stops being two near-identical rescues
that can drift apart.

**3. Decide whether this case should also warn, and justify it.** The `RowKey` path emits
`missing_alignment_key` for its keyless rows. This case is the exact analogue and currently emits
nothing. My inclination is that it should warn, because a consumer choosing `sample_columns` has
made a choice that silently excluded rows and would want to know — but the code's message names
*"alignment keys"*, and a signature is not a key, so reusing it may be dishonest. **Propose the code,
payload and message before implementing**, and remember a new `DiagnosticKind` brings RFC-036 §5.4's
obligation with it: a fixture whose assertion fires on the payload, not on `code()`.

**4. Sweep for the same shape elsewhere.** Two paths built the same row set and one was fixed. **Look
for any other place a row or cell can be filtered out of a collection that later decides what gets
compared** — a `continue` inside a loop that builds a key set, a `filter` before a map, an
`entry().or_default()` reached conditionally. Report the list even if this was the only one. The list
is the finding; f130 plus this unit is twice, and twice is a pattern.

## Required tests

1. **The reproduction**: on `RowSignature { sample_columns: Some([1]) }` with the table's input, the
   `before` → `AFTER` change **is reported**. Assert the cell address and both values, not a count.
2. **`sample_columns: None` is unchanged** — it was already correct and must stay so.
3. **`RowKey` is unchanged** — f130's behaviour, byte for byte. Find its existing test; do not write
   a second.
4. **An unchanged row with no sampled cells does not become a removal plus an insertion**, if that
   is what your rescue decides — the RowKey path's reasoning applied here. Assert whatever you chose
   and put the choice in a comment.
4b. **A forty-row sheet, sampled on two columns, with one spacer row populated only in a third,
   whose value changes.** The consumer built this to check the defect was not an artefact of a
   two-row sheet; it is not. Pre-fix it reports `matched: 39`, `Exact`, and **zero** cell diffs.

   *(Corrected 2026-10-06. This first said `matched: 40`, which was a mis-transcription: the
   consumer's sheet was **41** rows — forty sampled plus the spacer — giving 40 matched. On a
   literal forty-row sheet the spacer is one of the forty, so 39 match. The dev team measured 39,
   flagged the inconsistency between my two numbers, and deduced the 41-row case correctly without
   measuring it. The property tested is identical either way, so the delivered test stands.)*
   **This is the better regression test and it should be the one that carries the unit**, because
   `matched: 40` with `Exact` is a summary a consumer believes, where a two-row sheet can be
   dismissed as degenerate. Assert the change is reported, the count, and that `Exact` is gone.

5. **A sheet where every row lacks a sampled cell.** The degenerate case: no row has a signature at
   all. Assert it does something sane rather than panicking, dividing by zero or reporting `Exact`
   over an empty mapping.
6. **The diagnostic**, on its payload, if one lands.
7. **Failing-first for 1 and 5.**

## Acceptance criteria

1. No change goes unreported on any `sample_columns` value.
2. Tests 1–5, with failing-first where required; 6 if a diagnostic landed.
3. `confidence` on the reproduction is no longer `Exact` — as a consequence of the counts moving,
   not because you touched it. If it is still `Exact`, reported and stopped.
4. The sweep from *Required implementation* 4, as a list.
5. Whether the rescue is shared with `keyless_rows`, stated with the reasoning.
6. No golden moves. The corpus compares under default options; if one moves, stop and report.
7. `cargo public-api` shows nothing, or only an additive `DiagnosticKind` variant — detached
   worktree, not a stash. (Unit 00's criterion of an *empty* `public-api` diff is that unit's alone;
   it is not a property of the release.)
8. Gates green, rule 003, one scratch dir, deleted. Nothing committed.
9. A `CHANGELOG.md` entry under the **3.4.0** heading that says a changed cell could go unreported, in
   those words. Unit 00 adds its own entries under the same heading; leave room for them rather than
   restructuring the section. **Not
   "improved row signature alignment".** Anyone who used this mode with `sample_columns` needs to
   read that entry and know whether it affected them.

## Prohibited shortcuts

- Do not fix it by removing the `sample_columns` filter. The option is documented and useful; the
  defect is that filtering a row out of the *signature* also filtered it out of the *comparison*.
- Do not fix it by giving such rows an empty signature and letting LCS pair them. Every row with no
  sampled cell would then have an identical signature and pair arbitrarily among themselves — which
  is the ambiguity problem, reintroduced as the fix for the loss problem. If you believe that is
  nonetheless right, argue it first.
- Do not adjust `confidence` to compensate.
- Do not report this as a coverage gap. It is a defect; the coverage gap is why it survived.

## Known risks

**1. The fix changes output for anyone using this mode with `sample_columns`.** That is the point —
they are currently missing changes. The CHANGELOG has to let them work out whether they were
affected, which is why criterion 9 is worded as it is.

**2. The degenerate case may be worse than the reported one.** A sheet where *no* row has a sampled
cell produces an empty signature map on both sides. Check what `lcs_match` does with two empty
sequences before you decide the rescue's shape, and report it — if that path is also broken, it is
part of this unit.

**3. `sample_columns` may accept a column no row populates**, which makes the degenerate case
reachable by a plausible mistake rather than a contrived sheet — a user names a column that is empty
in both files. `DiffOptions` validation may or may not reject it; **find out and report**. If it is
accepted, the degenerate case is a realistic input and test 5 is the important one.

**4. This mode has had no consumer until now**, so the practical blast radius is probably nil — and
is about to stop being nil. ForskScope is adopting `RowSignature` this quarter. Do not let the low
historical impact set the care level.

## Required evidence

Under `.git-exclude/review-request/vanishing-row-01-a-change-never-reported/evidence/`:

1. The reproduction failing on `main`, with its output.
2. The rescue's behaviour on all five test shapes.
3. What `lcs_match` does with two empty sequences.
4. Whether `DiffOptions` accepts a `sample_columns` entry no row populates.
5. The sweep from *Required implementation* 4.
6. The diagnostic proposal and my reply, if one landed.
7. `cargo public-api`, detached worktree. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/vanishing-row-01-a-change-never-reported/README.md`, with:

- The rescue, and why it pairs rows the way it does.
- Shared with `keyless_rows` or not, and why.
- The sweep list.
- The degenerate case's behaviour, and whether `sample_columns` validation can reach it.
- **How long this has been shippable.** `git log -L` on `compute_row_signatures` will say, and f130's
  commit will say when the other path was fixed. If f130 fixed one path and left this one in the
  same change, I want that stated plainly rather than discovered later.
