# Handoff 04 — Resource bounds and `forbid(unsafe_code)`

**Governing RFC.** [RFC-035](../../accepted/035-resource-safety-and-supply-chain-governance.md) §5.1–5.6
**Roadmap.** M2, risk R6
**Sequence.** After unit 02 (merged). Independent of unit 03 — either order.
Unit 05 depends on this one; both touch `align.rs`.

## Purpose

Close the resource-exhaustion paths that ForskScope's dependency audit cannot
see, so that clearing the `quick-xml` chain is a real all-clear rather than a
false one.

## Background

Unit 02 cleared both advisories. Against the consumer's stated threat model —
*"our users open files they did not author"* — the fix is still incomplete,
because this crate has first-party equivalents of the same failure class:

- **`Limits::default()` is entirely unbounded.** All four fields are `None`, so
  every caller today has zero protection and nothing tells them so. Contradicts
  F-063 and NF-019.
- **`align.rs:195` allocates a full `m × n` `u32` table.** The only guard is a
  row-*count* threshold defaulting to 50 000, so a valid workbook can demand
  ~10 GB, and allocation failure **aborts the process** — violating NF-007.
- **`open.rs` buffers whole files with no size bound**, and `compare_bytes`
  copies its input a second time via `to_vec()`.

None of these needs an attacker. A large legitimate workbook reaches all three.

## Applicable requirements

NF-007 (must not abort), NF-013 (avoid unnecessary cloning), NF-019, F-049
(positional comparison always available), F-055 (alignment must be bounded),
F-063 (safe defaults), RFC-012, RFC-024, RFC-035 §5.1–5.6.

## Change scope

`src/options.rs` (`Limits`), `src/align.rs` (the bound and fallback),
`src/open.rs` (input size, double copy), `src/model.rs` (a new
`DiagnosticKind`), `src/lib.rs` (`forbid`), `src/address.rs` (remove the
`unsafe`), `tests/integration.rs`, `docs/`, `CHANGELOG.md`.

## Non-change scope

Do **not** change comparison semantics, sheet matching, or the public result
model beyond the new diagnostic variant. Do **not** fix the alignment
*coordinate-collision* defect — that is unit 05, and keeping the bound separate
from the correctness fix keeps both reviewable.

Do **not** bound the linear limits by default. See §1.

## Required implementation

### 1. Bound superlinear paths only

Per RFC-035 §5.1, the distinction is how cost **grows**, not how big it is.

- **Bound by default:** the alignment product. A caller who doubles their sheet
  does not expect memory to quadruple.
- **Leave `None` by default:** `max_cells_read`, `max_cells_compared`,
  `max_sheets`, `max_diffs_returned`. These scale predictably with input the
  caller chose to open; bounding them would break working code for no safety
  gain the caller could not have anticipated.

Add the alignment bound as a new `Limits` field. Note the existing
`max_cells_compared` is currently passed to `compute_row_mapping` as a *row*
bound — a semantic mismatch. The new field replaces that usage; say in the
review request what you did with the old wiring.

### 2. Alignment degrades, never fails

When the bound is exceeded: fall back to positional comparison, emit a
diagnostic, and continue. Do **not** return an error.

Rationale is in RFC-035 §5.2 and is load-bearing — erroring would let an
optional quality feature fail an otherwise valid comparison, and would make the
bound itself a denial-of-service vector.

Add a dedicated `DiagnosticKind` with a stable `code()`. While you are there:
`align.rs` currently reports duplicate alignment keys using
`UnsupportedCellValue`, whose documented meaning is "a cell value could not be
normalised" — wrong — and its message claims a positional fallback that does not
happen. Correct both. The `code()` table in `model.rs` is a documented stable
surface; update it.

### 3. Choose the default bound with a measurement

The default product value is a judgement call and RFC-035 §9 requires it be
justified by measurement, not intuition. Measure roughly what the `m × n` table
costs at a few sizes, pick a bound that keeps it to a sane ceiling, and **state
the number and the reasoning** in the review request.

### 4. Input size bound, and stop copying twice

Add a maximum input size to `Limits`, checked **before** `std::fs::read` in
`open_path` — checking after defeats the purpose. Remove the redundant
`to_vec()` in the `compare_bytes` path.

This one is a linear path, so per §1 it might seem it should stay unbounded.
It is different: unbounded *here* means the allocation happens before any
comparison logic can observe or report it, so the caller has no way to intervene.
A default is appropriate; choose one generous enough not to surprise anyone
opening an ordinary workbook, and say what you chose.

### 5. `Limits::hardened()`

A conservative bound on every dimension, documented against the threat it
addresses, and stating plainly that `Limits::default()` does not address it.

### 6. `#![forbid(unsafe_code)]`

`src/address.rs:129` uses `String::from_utf8_unchecked`, which buys nothing over
the safe constructor. Replace it and add the lint to `src/lib.rs`. One line each
way, and it converts a memory-safety argument into a compiler-enforced property.

## Required tests

- Alignment exceeding the bound **degrades and does not error**: assert the
  comparison succeeds, the diagnostic is present with the expected `code()`, and
  the result matches positional comparison.
- `Limits::hardened()` actually bounds each dimension.
- The input-size bound rejects **before** reading — prove it, e.g. by observing
  that no read occurs, not merely that an error is returned.
- The duplicate-key diagnostic uses its new code.
- Existing tests still pass; the golden corpus is untouched.

## Acceptance criteria

1. `Limits::default()` bounds the alignment product; the four linear limits stay
   `None`.
2. Exceeding the bound degrades to positional with a diagnostic — no error, no
   abort — and this is tested.
3. The default bound is justified by a stated measurement.
4. Input size is bounded before `fs::read`; `compare_bytes` no longer double-copies.
5. `Limits::hardened()` exists, is documented, and is tested.
6. `#![forbid(unsafe_code)]` is in force and the crate compiles.
7. The fixture corpus and all seven goldens are byte-identical. **If a golden
   moves, stop and report** — nothing here should change comparison output.
8. CI green across all jobs.

## Prohibited shortcuts

- Do not make the alignment bound an error "for now".
- Do not bound the linear limits by default to be safe. That is a behaviour
  change RFC-035 explicitly rejected.
- Do not suppress the `unsafe` with an `#[allow]` instead of removing it.
- Do not bless a golden. Nothing in this unit should move one.
- Do not pick the default bound by intuition and describe it as measured.

## Compatibility constraints

The alignment default bound is the only behavioural change, and it affects only
callers who opted into a non-`Positional` alignment mode on very large sheets —
who today get an abort. Degrading with a diagnostic is strictly better.

`Limits` gains fields. Confirm this does not break the documented construction
pattern (`Default` plus field assignment); if it does, **stop and report** — that
is a public API question, not an implementation detail.

## Security constraints

This unit is the reason M2 is not already finished. Its absence is what makes
the dependency fix a false all-clear (roadmap R1), so the bounds must actually
bound — a default that no realistic workbook reaches protects nobody.

## Known risks

- Choosing the bound too low silently degrades legitimate alignment; too high
  and the protection is theoretical. The diagnostic makes degradation visible
  either way, which is why it is mandatory rather than optional.
- Removing the `unsafe` is trivial, but `forbid` is crate-wide and may surface
  `unsafe` in a place nobody expects. If it does, report rather than carve out
  an exception.

## Required evidence

- The measurement behind the default bound
- Test output showing degradation, not failure, past the bound
- Proof the input-size check precedes the read
- `cargo test` across all five feature combinations
- The corpus hash, unchanged from `c056b0fc…`
- CI run link, all jobs green

## Review request format

Per development policy §9.2, plus the chosen bound values with their reasoning
and a statement of what happened to the old `max_cells_compared` alignment
wiring.
