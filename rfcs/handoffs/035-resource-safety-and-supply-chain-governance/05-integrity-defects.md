# Handoff 05 — Integrity-affecting correctness defects

**Governing RFCs.** RFC-010 (cell comparison), RFC-011 (alignment),
RFC-018 (formula semantics), RFC-019 (numeric/date policies). These are
**defects against decisions those RFCs already made**, not new design.
**Roadmap.** M2, §6 (integrity as a security property)
**Sequence.** After unit 04 (approved, merged into PR #10). Both touch `align.rs`.

## Purpose

Fix the cases where this crate reports "identical" for cells that differ.

In a diff/**merge** workstation a false negative is not a quality defect. The
user is shown "no change", accepts the merge, and loses data — a data-loss path
reachable from ordinary input with no attacker involved. That is why these ship
inside the security release rather than after it.

## Provenance of every claim below

Each defect was re-derived from the code at `de9eae8` for this handoff, not
carried over from audit notes. D-01 was additionally **proven by execution**:
a temporary probe in `src/compare.rs`'s test module asserted that
`compare_values` returns `None` for the pairs shown, and it passed. Line numbers
are as of `de9eae8`.

---

## D-01 — ISO date/time and duration values always compare equal

**Severity: high.** This is the most serious finding in the original audit.

`src/normalize.rs` maps `Data::DateTimeIso(s)` to
`CellDateTime { serial: 0.0, is_1904: false, kind: DateTime, iso: Some(s) }`,
and `Data::DurationIso(s)` to `CellDuration { serial: 0.0, iso: Some(s) }`.

`src/compare.rs` then compares:

```rust
(DateTime(a), DateTime(b)) => {
    if a.serial == b.serial && a.is_1904 == b.is_1904 && a.kind == b.kind {
        return None;          // <- iso is never examined
    }
(Duration(a), Duration(b)) => {
    if a.serial == b.serial { return None; }   // <- iso is never examined
```

Every `DateTimeIso` value carries `serial: 0.0`, `is_1904: false`,
`kind: DateTime`. So **any two of them compare equal**. Proven:

- `2024-01-01T00:00:00` vs `2099-12-31T23:59:59` → reported identical
- `PT1H` vs `PT99H` → reported identical

**Reachability.** calamine emits `DateTimeIso`/`DurationIso` for ISO-8601-typed
cells (`t="d"` in the sheet XML), not for the ordinary serial-based dates Excel
usually writes — those take the `Data::DateTime` path, which compares correctly.
So this is not every workbook, but it is a real and silent path when it occurs.
Establishing how reachable it is in practice is part of this unit: see
**required tests**.

**Fix direction, not prescription.** The comparison must consider `iso` when the
serial is not meaningful. Deciding *how* is within RFC-019's remit and yours to
propose: compare `iso` when both sides have one; or parse ISO to a serial at
normalisation time so a single representation is compared; or treat a
`0.0`-serial-with-`iso` as its own case. Each has consequences for
`DateComparePolicy` and for mixed `DateTime`-vs-`DateTimeIso` comparisons, which
today fall to the cross-type arm. **State which you chose and why**, and say what
happens when one side is serial-based and the other ISO — that case must not
silently become "equal" either.

## D-02 — `is_1904` is hardcoded, so a documented policy is dead code

`src/normalize.rs:64` and `:74` set `is_1904: false` unconditionally. The
comment beside them says the flag is private in calamine — **that comment is now
stale**: unit 01's spike confirmed `Xlsx::has_1904_epoch()` is public, and
compiled a probe proving it is reachable from `read_sheet_cells` while holding
`&mut OpenedWorkbook`.

Consequence: `DateComparePolicy::NormalizeEquivalentDateTimes` — a documented,
public option whose entire purpose is reconciling the 1900 and 1904 date systems
— can never do anything, because both sides are always `false`. A caller
selecting it gets silence, not an error.

The spike's plumbing sketch: call `has_1904_epoch()` once in `open_bytes_inner`,
store it on `OpenedWorkbook`, thread it into normalisation. Workbook-level, not
per-cell. Verify that sketch rather than assuming it; report if it does not hold.

Update the stale comment to say what is now true.

## D-03 — Alignment can merge two distinct cells into one coordinate

`src/diff.rs`, the coordinate-set construction when `align_mapping` is present:

```rust
for (old_row, new_row) in &mapping.matched {   // inserts (old_row, c)
for r in &mapping.removed {                    // inserts (r, c)   — old-side rows
for r in &mapping.inserted {                   // inserts (r, c)   — NEW-side rows
```

Matched and removed contribute **old-side** row numbers; inserted contributes
**new-side** row numbers, into the same `BTreeSet`. Nothing distinguishes them.
If an inserted new-side row number collides with a matched or removed old-side
row number — which needs no unusual workbook, only overlapping numbering — the
set silently dedupes two distinct logical cells into one coordinate.

The lookup then compounds it:

```rust
let new_lookup_row = align_mapping.as_ref()
    .and_then(|m| m.matched.get(&row)).copied().unwrap_or(row);
```

For a coordinate that arrived from an *inserted* row, if that number is also a
matched old row, the mapping wins and the comparison reads the wrong new-side
row. The inserted row's content is compared against the wrong cell, or not at
all.

This affects only non-`Positional` alignment modes, which is why the fixture
corpus has never caught it.

**Fix direction:** the coordinate set needs to carry which side a row number
came from, rather than relying on numeric identity across two coordinate spaces.
Propose a shape; do not paper over it by renumbering.

## D-04 — Formula text can attach to the wrong cell

`src/diff.rs:496` takes the **value** range's origin:

```rust
let origin = range.start().unwrap_or((0, 0));
...
let row1 = origin.0 + row_idx as u32 + 1;      // absolute, from the value range
let formula = formula_range.as_ref()
    .and_then(|fr| fr.get((row_idx, col_idx)))  // relative indices into the FORMULA range
```

`row_idx`/`col_idx` are relative to the value range. They are applied unchanged
to the formula range, which is a separate `Range` with its own `start()`. The
code assumes both origins coincide. If they differ, every formula is offset by
the difference and attaches to the wrong cell — silently, since a formula
present at the wrong address looks like an ordinary formula.

**Establish whether the origins can differ** before deciding the fix. If
calamine guarantees they coincide, the correct change may be an assertion plus a
comment recording the guarantee. If they can differ, translate through absolute
coordinates. Report which you found and on what evidence.

---

## Applicable requirements

NF-004 (comparison semantics documented), NF-005 (empty/blank/missing tested),
F-030 (preserve date/time and duration values), F-034 (formula text separate
from cached value), RFC-010, RFC-011, RFC-018, RFC-019, roadmap §6.

## Change scope

`src/normalize.rs`, `src/compare.rs`, `src/diff.rs`, `src/open.rs`,
`src/model.rs` if the value model needs it, `tests/integration.rs`,
`tests/fixtures/` if new scenarios are added, `CHANGELOG.md`, `docs/`.

## Non-change scope

- Do **not** touch `src/objects.rs`'s "calamine 0.35" strings. They are embedded
  in all seven goldens, and unit 06 owns them. Keeping them out of this unit
  means that if a golden moves here, it moved because **behaviour** changed —
  which is the signal this unit needs to stay readable.
- Do not implement newly-available calamine capabilities (hyperlinks, merged
  regions, tables, pivot tables). Later scope.
- Do not change the public API beyond what a defect fix requires. If one does
  require it, stop and report before proceeding.

## Required tests

Every defect needs a test that **fails before the fix and passes after**. Write
the test first and observe it fail; a regression test never seen red proves
nothing.

- **D-01:** unit-level coverage of the comparison for ISO-vs-ISO on both
  `DateTime` and `Duration`, plus the mixed serial-vs-ISO case.
- **D-01 reachability:** attempt an end-to-end fixture with a real
  ISO-typed cell. `rust_xlsxwriter` may not emit `t="d"`; if you cannot produce
  one, **say so explicitly and say what you tried** — an honest "not reproducible
  with our fixture tooling" is a useful result and must not be dressed up as
  coverage.
- **D-02:** a test that `NormalizeEquivalentDateTimes` actually reconciles a
  1900-system and a 1904-system value, which is impossible today.
- **D-03:** a case where an inserted new-side row number collides with a matched
  old-side row number, asserting both cells are compared.
- **D-04:** whatever your investigation shows is testable.

## Acceptance criteria

1. Each of D-01…D-04 is fixed, or — for D-04 specifically — shown not to be a
   defect, with the evidence that shows it.
2. Every fix has a test observed failing before it and passing after.
3. The design decision for D-01 is stated with its reasoning, including the
   mixed serial-vs-ISO case.
4. `DateComparePolicy::NormalizeEquivalentDateTimes` demonstrably works.
5. Full feature matrix green; `fmt`, `clippy -D warnings`, `deny`, MSRV clean.
6. **Goldens:** if any `expected.json` changes, each change is explained and
   justified in the review request as an intended behaviour correction. Blessing
   is permitted in this unit — it is the first where it is — but only with that
   explanation. Silent re-blessing is not.
7. CI green.

## Prohibited shortcuts

- Do not fix D-01 by making all `DateTime` comparisons fall back to display
  strings. That would paper over the typed model this crate exists to provide.
- Do not bless a golden without explaining what changed and why it is correct.
- Do not weaken a test to accommodate a fix.
- Do not resolve D-04 by asserting the origins coincide without evidence.
- If a fix appears to need a public API change, stop and report rather than
  taking it.

## Compatibility constraints

These fixes change comparison output — that is their purpose. Cells previously
reported identical will now be reported as different. This is a **behaviour
change in a patch-level sense but a correctness fix in substance**; the CHANGELOG
must say so plainly, because a consumer's stored diffs may change. Flag anything
you think warrants more than a CHANGELOG note.

## Security constraints

These are integrity fixes, and integrity is a security property here (roadmap
§6). A remaining false negative is a data-loss path. If you find a further one
while working, report it — do not fold it in silently, and do not leave it
unmentioned because it is out of scope.

## Known risks

- D-01's fix may alter comparison results for workbooks that currently pass
  silently. That is the point, and criterion 6 exists to make each such change
  visible.
- D-03's fix touches coordinate construction, which unit 04 also modified. Read
  the current code rather than the pre-unit-04 shape.
- D-04 may turn out not to be a defect. That is an acceptable outcome if it is
  evidenced.

## Required evidence

- For each defect: the test failing before, passing after
- The D-01 design decision and reasoning
- The D-02 1900/1904 reconciliation demonstrated
- The D-04 investigation result and what it rests on
- Full matrix, gates, MSRV, CI run link
- Corpus hash, with every golden change explained if any moved

## Review request format

Per development policy §9.2, plus the D-01 design decision and an explicit
statement of whether any golden moved and why.
