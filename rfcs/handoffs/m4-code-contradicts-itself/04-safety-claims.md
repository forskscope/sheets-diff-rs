# Handoff 04 — The safety claims

**Governing RFCs.** RFC-012 (cancellation and limits), RFC-035 (resource safety
and supply-chain governance), RFC-013 (CLI and exit codes)
**Roadmap.** M4 — closes the milestone
**Sequence.** After unit 02 (uses the accumulator it introduced). Unit 03 is
independent of this one.

## Purpose

Three statements this crate makes about its own safety that are not true: a
resource limit that does not limit the resource it names, a threat model that
tells callers it does, and an error classification that reports a disk failure
as a corrupt file.

This is the same defect class as the rest of M4, in the one area where being
wrong matters most — the claims a security-conscious consumer would rely on.

## Background

### F-A — `max_cells_compared` bounds the wrong thing

`src/diff.rs`, inside the coordinate loop in `build_sheet_diff`:

```rust
if let Some(max) = opts.limits.max_cells_compared {
    let compared_so_far = cell_diffs.len() as u64 + 1;
    if compared_so_far > max {
        return Err(SheetsDiffError::LimitExceeded {
            limit: LimitKind::CellsCompared,
            observed: compared_so_far,
        });
    }
}
```

`cell_diffs` is pushed to only when a coordinate produces a diff — the same
`continue` that made `cells_compared` wrong in unit 02. So `compared_so_far`
counts **diffs found so far**, not coordinates visited. The limit bounds output
size, which `max_diffs_returned` already bounds, and does not bound work at all.

`src/options.rs` says the linear limits exist because "their cost scales
predictably with input size". Bounding cost is the one thing this cannot do. A
workbook with millions of populated cells and few differences passes straight
through. Unit 02's corpus makes the gap concrete: `sparse_range` reads 5200
cells, compares 2, emits 1.

`observed` in the returned error is wrong for the same reason — it reports a
diff count under a `LimitKind::CellsCompared` label.

**Two further inconsistencies to resolve while you are here.** The
`max_diffs_returned` check a few lines below uses `*total_diffs`, which is
cumulative across the whole comparison; this check uses a per-sheet local. They
should agree, and cumulative is the correct reading — a limit on a comparison,
not on each sheet independently. And the check runs *after* the coordinate set
is built, so it fails partway through work it has already begun.

### F-E — the threat model claims the protection

`docs/src/maintainers/threat-model.md`, "The bounds themselves (`Limits`)"
(around lines 146–153), lists `max_cells_compared` among "the four genuinely
linear paths" and states that `Limits::hardened()` "sets a concrete value on
every dimension for callers comparing genuinely untrusted input at scale."

Given F-A that is false. A hardened caller has three effective linear bounds,
not four, and the missing one is coordinates compared — the dimension a
large-workbook denial of service would actually exploit. We documented a
protection we do not provide.

### F-F — a disk error reported as a corrupt file

`src/error.rs`:

```rust
fn classify_read_error(e: &calamine::XlsxError) -> ReadErrorKind {
    match e {
        XlsxError::WorksheetNotFound(_) => ReadErrorKind::SheetNotFound,
        _ => ReadErrorKind::MalformedSheet,
    }
}
```

The catch-all swallows `XlsxError::Io`. A disk or network-filesystem failure
part-way through reading a sheet is classified `MalformedSheet`, and as of unit
03 the CLI exits 3 and tells the user their file is corrupt when nothing is
wrong with the file.

The misclassification predates unit 03 and was invisible while every failure
exited 2. Unit 03 is what makes it user-visible, and both ship in 2.4.0, so it
is fixed here rather than deferred.

`ReadErrorKind::Other` exists and is currently unreachable — nothing produces
it. `main.rs`'s `exit_code_for` maps the whole `ReadSheet` variant to 3, so if
`Other` became reachable it would report "corrupt", contradicting the
conservative default that unit 03 correctly applied to `OpenErrorKind::Other`.

## Change scope

`src/diff.rs`, `src/error.rs`, `src/main.rs` (the `ReadSheet` arm only),
`docs/src/maintainers/threat-model.md`, `tests/integration.rs`, `tests/cli.rs`,
`CHANGELOG.md`.

## Non-change scope

- Do not change `Limits::default()` or `Limits::hardened()` values. The
  enforcement is wrong; the numbers are a separate decision and are not yours.
- Do not change comparison behaviour, diff output, or `DiffMetrics`.
- Do not touch the other limit checks (`max_alignment_product`,
  `max_input_bytes`, `max_sheets`, `max_cells_read`, `max_diffs_returned`)
  beyond what F-A's consistency point requires. **If you find another limit
  enforcing the wrong quantity, stop and report it rather than fixing it here.**
- The fixture corpus must not move. This changes error paths, not diff results.

## Required implementation

1. **F-A: bound coordinates, cumulatively, before the work.** The recommended
   shape is to check at the point unit 02 added `*total_cells_compared +=
   coords.len()`: if the running total plus this sheet's coordinate set would
   exceed the limit, return `LimitExceeded` there. That makes the bound
   cumulative (matching `max_diffs_returned`), counts the right quantity, and
   refuses the work before starting it rather than partway through — which is
   what a resource bound is for. If you implement it differently, say why.
2. **`observed` must report the quantity the `LimitKind` names.**
3. **F-F: classify `XlsxError::Io` as `ReadErrorKind::Other`**, leaving
   `MalformedSheet` for what is genuinely malformed. Then map `ReadSheet`'s
   sub-kinds individually in `exit_code_for` instead of wholesale:
   `MalformedSheet` and `SheetNotFound` are 3, `Other` is 2 by the same
   conservative default unit 03 already applies.
4. **F-E: correct the threat model** to state what is bounded after this
   change. It is a current-state document — say what is true now, not what was
   wrong before.
5. **Comment the `SheetNotFound => 3` decision** in `exit_code_for`. It is
   sound only because the CLI has no sheet-selection flag; a future `--sheet`
   would make it caller error and move it to 2. Nothing records that today.

## Required tests

1. **A test that fails under the current enforcement.** This is the whole
   exercise and it is easy to get wrong — a limit low enough to trip *both*
   the old and new code proves nothing.

   The discriminating case is **many coordinates compared, zero diffs**: with
   no diffs, `cell_diffs` stays empty, so the old `compared_so_far` never
   exceeds 1 and never trips at any limit. Comparing a fixture's `old.xlsx`
   against itself gives you exactly that, with no new fixture — a populated
   workbook, many coordinates, zero differences. Set `max_cells_compared` low
   via the builder and assert `LimitExceeded { limit: LimitKind::CellsCompared, .. }`.

   Verify the discrimination rather than assuming it: state in the review
   request what the old code does with your chosen input and limit.
2. **A test that the limit does not fire below the bound** — otherwise a fix
   that errors unconditionally passes test 1.
3. **`observed` carries the coordinate count**, asserted.
4. **F-F: a test pinning `ReadErrorKind::Other` for an I/O failure**, if you
   can construct one without a new dependency. If you cannot reach that path in
   a test, say so plainly rather than leaving the impression it is covered —
   the classification fix and the `exit_code_for` arm still stand on their own.

## Acceptance criteria

1. `max_cells_compared` bounds coordinates compared, cumulatively across the
   comparison.
2. A test fails under the old enforcement and passes under the new, and the
   review request shows why it discriminates.
3. The limit does not fire below the bound.
4. `observed` reports the quantity `LimitKind::CellsCompared` names.
5. `XlsxError::Io` no longer classifies as `MalformedSheet`; `exit_code_for`
   maps `ReadSheet`'s sub-kinds individually, with `Other` conservative at 2.
6. The threat model states what is actually bounded, as current state.
7. The `SheetNotFound => 3` reasoning is recorded in the code.
8. Fixture corpus byte-identical.
9. CHANGELOG records F-A as a behaviour change under `### Changed`, with the
   compatibility consequence in §"Compatibility constraints" stated plainly.
10. Gates green: fmt, clippy `-D warnings` on `--all-features` and
    `--features cli`, deny, MSRV 1.88, full CI matrix.

## Prohibited shortcuts

- Do not make the limit fire on a count that merely *correlates* with
  coordinates because it makes a test pass. Count the coordinates.
- Do not weaken `hardened()`'s value to avoid the compatibility consequence.
  The owner has ruled on that consequence; absorbing it by quietly loosening
  the bound would reverse their decision without saying so.
- Do not delete the `ReadErrorKind::Other` arm because it is unreachable. Unit
  03 established that an unclassified case defaults to the conservative code;
  this makes that real.
- Do not fix the threat model by deleting the sentence. The bounds section
  needs to say what is bounded.

## Compatibility constraints

**The owner has ruled: proceed, and say so plainly.**

`Limits::hardened()` sets `max_cells_compared: Some(5_000_000)`. Today that
value bounds diffs, so a hardened caller comparing a large workbook with few
differences never trips it. After this change they will. **A comparison that
succeeded in 2.3.0 can return `LimitExceeded` in 2.4.0** for callers using
`hardened()` or setting the limit explicitly.

That is the limit doing what it was always documented to do, and it is still a
behaviour change a consumer can be surprised by. It belongs in the CHANGELOG as
a compatibility event in its own right, not folded into F-A's description.
`Limits::default()` leaves the limit unset, so default-configured callers are
unaffected — say that too, since it bounds who is exposed.

## Known risks

- The discriminating test is the part most likely to be got wrong, in the way
  this project has been bitten before: a test that passes both before and after
  looks like coverage and is not. Requirement 1 exists for that reason.
- Checking before the loop changes *when* the error surfaces, not just whether.
  If any test asserts on partial results before a limit error, it will move.
  Nothing should — limit errors abort the whole comparison — but confirm.
- F-F changes a public `#[non_exhaustive]` enum's produced variant for a given
  input. Callers matching on `ReadErrorKind` must already have a catch-all, so
  this compiles everywhere, but it is a behaviour change and belongs in the
  CHANGELOG.

## Required evidence

- The diff
- The discriminating test, with a statement of what the old code does with the
  same input and limit
- Corpus unchanged
- Gates output, both feature legs
- CI run link

## Review request format

Per development policy §9.2, plus an explicit statement of how requirement 1's
test discriminates old from new.
