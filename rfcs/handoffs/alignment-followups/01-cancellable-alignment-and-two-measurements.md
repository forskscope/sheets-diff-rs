# Handoff — Cancellable alignment, and two measurements that stopped meaning anything

**Governing.** RFC-011 (row alignment); RFC-025 / M7 Handoff 03 (cancellation);
ROADMAP "Alignment follow-ups"
**Sequence.** Before M9's units 01–06. Not a milestone unit — it discharges a
written commitment and closes two loose ends from f130 and M9 unit 00.
**Release:** the next minor. Item 1 is a behaviour change; 2 and 3 are not.

## Purpose

Three small things in one place, all in the alignment area.

1. **Alignment is not cancellable.** 2.5.1 made the *read* cancellable and left
   this phase. **We told ForskScope on 2026-09-26 that it is scheduled**, so this
   discharges a commitment in writing.
2. **`benches/memory.rs` measures an alignment that does not happen** — the
   finding behind M9 unit 00's correction, now wrong in a second way after f130.
3. **`missing_alignment_key`'s location is not covered** for a renamed or moved
   sheet. A coverage gap, not a defect: the behaviour is correct and I traced it.

## Item 1 — cancellable alignment

`grep -c check_cancel src/align.rs` → **0**. ForskScope measured a cancel
requested 100 ms into a 1.2 s alignment and observed at **1,208 ms**.

**The cost is the LCS table fill** in `lcs_match`:

```rust
for i in (0..m).rev() {
    for j in (0..n).rev() { … }      // n work per outer step
}
```

**The outer loop is the poll point.** One check per table row gives m checks;
at ForskScope's 4,900 rows that is a granularity of about 0.25 ms, and the cost
is one atomic load per `n` cell operations.

### How to thread it

`compute_row_mapping` does **not** receive `&DiffOptions`, and should not start:
the established pattern in that signature is to pass the narrow thing —
`max_alignment_product: Option<u64>` rather than the options tree.

**Pass what cancellation needs and nothing more.** `Cancellation` is already a
public trait in `options.rs`. `check_cancel` is private to `diff.rs`; do not
widen it — decide whether `align.rs` gets its own small helper or the check is
inlined, and say which.

1. **Poll once per outer step of the DP fill**, returning
   `Err(SheetsDiffError::Cancelled)`.
2. **`compute_row_mapping` and `lcs_match` become fallible.** They return
   `Option<RowMapping>` and `RowMapping` today; the call site in `src/diff.rs` is
   already inside a `Result` function.
3. **The other phases: measure, then decide.** `extract_row_keys`,
   `pair_identical_rows` and the backtrack are linear in data already read, so
   they are probably not worth a poll — **say what you measured** rather than
   assuming. If one of them is a material fraction of a large alignment, poll it
   too.
4. **No new public API.** `AlignmentMode`, `RowMapping` and every signature in
   `options.rs` stay as they are.

### Tests

- A cancellation requested mid-alignment is observed **mid-alignment**: with a
  token that trips after a fixed number of checks, `compare_*` returns
  `Err(Cancelled)` and the elapsed time is a small fraction of an uncancelled
  run on the same input. **Not a wall-clock threshold** — a ratio, so it survives
  a CI runner.
- **Cancellation between the read and the alignment still works**, and an
  uncancelled comparison is byte-identical to before.
- **Show it failing by removing the poll** — the M7 lesson: that unit's
  cancellation test passed with its poller deleted, because the mutation reverted
  a whole file and could not discriminate. Remove **the one statement**.

## Item 2 — the bench measures nothing

`benches/memory.rs:382`:

```rust
.alignment(AlignmentMode::RowKey { columns: vec![0] }) // the stable "id_N" column
```

**Key columns are 1-based** (`options.rs:148`) and cells are stored 1-based
(`diff.rs:688`), so column 0 matches nothing. Every row was keyless; the LCS ran
on two empty sequences; the "delta 0.0%" recorded in `performance.md` and the
threat model was the cost of an alignment that did not happen. Both pages are
already corrected — **the bench is not.**

**And f130 changed it again:** with every row keyless, that line now exercises the
keyless path — pairing, a `Warning`, `confidence: Medium` — which is a third
thing, and not what the bench claims to measure.

1. **Key it on the column the comment means**, and check by asserting
   `matched_rows > 0` in the bench's own output rather than trusting the number.
2. **Re-record the figures** and reconcile them with the table M9 unit 00 put in
   `performance.md` (`(rows+1)² × 4`). If they disagree, the disagreement is the
   finding.
3. **Add the keyless case as its own measurement** if it is cheap — it is now a
   real path with a real cost — or say why not.
4. **`performance.md`'s "23×-inflated, clearly-wrong delta"**, discarded as a
   wrong-column artefact, is probably the real measurement. Once the bench keys
   correctly you can say whether it is, and the note should stop hedging.

## Item 3 — the location coverage gap

`tests/diagnostic_location.rs` asserts the new-side rule — that a diagnostic
about a renamed or moved sheet names the **new** workbook's sheet — for
`alignment_bound_exceeded` and `duplicate_alignment_key`, and not for
`missing_alignment_key`.

**The behaviour is correct.** `src/diff.rs:418` passes
`new_sheet.as_ref().or(pair.old_sheet.as_ref())` into `compute_row_mapping`, so
all three share it. **Only the test does not reach it.**

Extend the existing assertion to the third warning, on a renamed **and** moved
sheet. **Show it failing** by making `missing_alignment_key` name the old side.

## Change scope

`src/align.rs`, `src/diff.rs`, `benches/memory.rs`,
`docs/src/maintainers/performance.md`, `tests/`, `CHANGELOG.md`.
RFC-011 or RFC-025 only if item 1's reasoning belongs in a record — decide and
say which.

## Non-change scope

- **No public API change.** Not a signature in `options.rs`, not a variant, not
  a field.
- Do not change what alignment *computes* — only when it can be interrupted.
- Do not change `CANCEL_POLL_INTERVAL` or the read's poll.
- Do not implement O((m+n)·D) here. It is RFC-gated with no date, and ForskScope
  has been told so.
- Do not touch the keyless-row pairing f130 shipped.

## Acceptance criteria

1. Alignment is cancellable; a mid-alignment cancel is observed mid-alignment,
   proven by a **ratio** not a wall-clock figure.
2. The poll's placement is justified, and the other phases are **measured** and
   reported before being left unpolled.
3. Removing the poll fails a test, demonstrated by removing one statement.
4. No public API change; an uncancelled comparison is byte-identical.
5. The bench keys a populated column and **asserts the alignment ran**.
6. Re-recorded figures reconciled with `performance.md`'s table; the "23×"
   note resolved or explicitly still open.
7. The location assertion covers all three warnings on a renamed and moved
   sheet, shown failing.
8. Corpus byte-identical.
9. CHANGELOG: `### Changed` for item 1; items 2 and 3 are internal — say so or
   put them under `### Documentation`, your call.
10. Gates green, including `cargo check --manifest-path fuzz/Cargo.toml --bins`.

## Prohibited shortcuts

- **Do not pass `&DiffOptions` into `align.rs`** to get at cancellation. Pass
  what is needed.
- Do not poll inside the **inner** loop — that is one atomic load per cell of a
  quadratic table, for granularity nobody needs.
- Do not demonstrate cancellation by reverting a file. One statement.
- Do not leave the bench's misleading comment in place while changing the code
  under it.

## Known risks

- **`lcs_match` becoming fallible touches its callers**, including `RowSignature`
  and f130's pairing path. All `pub(crate)` or private; if a signature change
  reaches the public API, stop and report.
- A ratio-based timing test can still be flaky on a loaded runner. Pick a margin
  you can defend and state it.
- The re-recorded bench figures will differ from every number currently in
  `performance.md` for that row. That is the point; make sure the page says
  which numbers are which.

## Required evidence

- The mid-alignment cancellation, as a ratio, and the failing-first transcript
- What you measured for the other phases
- The bench before and after, with `matched_rows` shown non-zero
- The reconciliation against `performance.md`'s table
- The location test failing first
- Corpus byte-comparison; gates with exit statuses
- CI run link

## Review request format

Per development policy §9.2. Additionally: state how you threaded cancellation
and why, what you measured for the unpolled phases, and whether the "23×"
measurement was the real one.
