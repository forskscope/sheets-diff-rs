# RFC-035 — Resource Safety and Supply-Chain Governance

**Status.** Proposed
**Target:** M2 / 2.3.0
**Created:** 2026-08-16
**Amends:** RFC-012, RFC-016, RFC-024, RFC-026
**Related:** RFC-028, RFC-031, RFC-032

## 1. Summary

Define the safety posture `sheets-diff` commits to for untrusted workbooks:
which resources are bounded by default and which are opt-in, how the
dependency tree is gated, what happens when an advisory lands on a dependency,
and what the threat model must say. Together these are the difference between
"we believe this is safe" and "this is checked on every build."

## 2. Motivation

ForskScope — the only known consumer — disabled `.xlsx` comparison entirely
over `RUSTSEC-2026-0194` / `RUSTSEC-2026-0195` in `quick-xml` 0.39.4, reachable
through `calamine` 0.35. Re-enabling on their side is a dependency-policy
change requiring evidence, not a version bump.

Fixing that chain alone would be a false all-clear. Their stated threat model is
*"our users open files they did not author"*, and against that model this crate
has first-party equivalents of the same failure class that their dependency
audit cannot see:

- **`Limits::default()` is entirely unbounded.** All four fields are `None`, so
  every caller today gets zero resource protection unless they opt in, and
  nothing in the documentation tells them to. This contradicts F-063 ("defaults
  must be safe… not surprisingly expensive") and NF-019 directly.
- **`align.rs` allocates an unbounded `m × n` table.** The only guard is a
  row-*count* threshold defaulting to 50 000, so a valid-but-hostile workbook
  can demand ~10 GB. Allocation failure aborts the process, violating NF-007.
- **`open.rs` buffers whole files with no size bound**, and `compare_bytes`
  copies the input a second time via `to_vec()`.

None of these need an attacker — a large legitimate workbook reaches them.

Separately, the project has **no** `deny.toml`, no `cargo audit` gate, no
threat model, and no written process for what to do when an advisory lands. The
consumer has all four. A library whose supply-chain hygiene is weaker than its
consumer's is the wrong way round, and it is why the advisory sat for two months.

## 3. Goals

- Safe-by-default behaviour for the resource classes that fail
  catastrophically, without silently changing behaviour for the classes that
  fail predictably.
- A dependency tree gated on every build, so "no network access" and "no
  advisories" are machine-checked claims rather than prose.
- A written, followable response when an advisory lands.
- A threat model sufficient to decide whether to accept a given risk.

## 4. Non-goals

- Sandboxing or process isolation.
- Defending against a malicious *caller* — the caller is trusted; the
  *workbook* is not.
- Making the crate safe for arbitrary untrusted input at unlimited scale.
  Bounds are the mechanism; unlimited safety is not offered (NG-010).
- Any change to comparison semantics. Correctness defects are handled as
  defects against their existing RFCs, not here.

## 5. Design

### 5.1 Bounded by default: superlinear only

The distinction that matters is not "how big" but **how the cost grows**.

- **Superlinear paths are bounded by default.** Row alignment's `m × n` table
  is quadratic and surprising: a user who doubles their sheet does not expect a
  quadrupling of memory. A default product bound applies.
- **Linear paths stay opt-in.** `max_cells_read`, `max_cells_compared`,
  `max_sheets` and `max_diffs_returned` scale predictably with input size, and
  a caller who opens a large workbook expects a large comparison. Bounding
  these by default would break working code for no safety gain that the caller
  could not have anticipated.

This split keeps the change non-breaking in practice — no positional diff that
works today starts failing — while removing the failure mode that can take the
host process down.

### 5.2 Alignment degrades, it does not fail

When the alignment bound is exceeded, the comparison **falls back to positional
comparison and emits a diagnostic**. It does not return an error.

Rationale: alignment is a *quality* feature. Positional comparison is the
documented default and always available (F-049). Failing the whole comparison
because an optional quality improvement did not fit is a worse outcome for the
caller than a correct-but-noisier diff plus a diagnostic saying why. Erroring
would also make the bound itself a denial-of-service vector.

A new `DiagnosticKind` is required. Note that `align.rs` currently misuses
`UnsupportedCellValue` for a duplicate-key condition, and that diagnostic
claims a positional fallback that does not happen; both are corrected here.

### 5.3 An explicit hardened preset

`Limits::hardened()` returns a conservative bound on every dimension, so the
safe configuration for untrusted input is one call rather than four decisions.
The documentation states plainly which threat it addresses and that
`Limits::default()` does not address it.

### 5.4 Input size is bounded before it is read

`open_path` currently calls `std::fs::read` with no size check. A maximum input
size is added to `Limits`, checked before the read, and the redundant `to_vec()`
copy in `compare_bytes` is removed.

### 5.5 Supply-chain gating

A `deny.toml` gates every build via CI:

- **advisories** — deny; the `cargo audit`/`cargo deny advisories` gate is what
  would have caught the `quick-xml` chain when it landed rather than two months
  later.
- **bans** — network-capable crates are denied outright. This turns NF-015
  ("the library must not access the network") from a claim into a build-time
  property. The same for any crate that would give the library ambient
  filesystem or process access beyond what it already has.
- **licenses** — allowlist consistent with Apache-2.0 distribution.
- **sources** — crates.io only.

This is deliberately equivalent to the gate the consumer already runs, so a
candidate can be verified against the same standard on both sides.

### 5.6 `#![forbid(unsafe_code)]`

The crate contains exactly one `unsafe` block —
`String::from_utf8_unchecked` in `col_to_label` — which buys nothing over the
safe constructor. Removing it permits the lint, converting a memory-safety
argument into a compiler-enforced guarantee for a one-line change.

### 5.7 Advisory-response policy

Written into the threat model document:

1. The `cargo deny advisories` gate fails the build. That is the trigger; there
   is no manual watch.
2. Assess reachability from untrusted workbook input.
3. If reachable, the response is a dependency bump, an upstream fix, or a
   documented temporary exception with an expiry date — never silence.
4. Notify known consumers, since a consumer's fail-closed posture is more
   costly to them than to us.

### 5.8 Threat model

A document under `docs/src/maintainers/threat-model.md`, mirroring the
consumer's path so the two sit side by side. Sufficient means: assets; the
trust boundary; actors; each attack surface with mitigations and **residual**
risk; explicit non-defences; and a verification map naming which control is
machine-checked and where.

It must record **integrity of the diff result** as a first-class asset. In a
diff/merge workstation a silently missed difference means a user is shown
"identical", accepts a merge, and loses data — a data-loss path reachable from
ordinary input with no attacker involved. False negatives are therefore
integrity failures, not quality defects.

## 6. Compatibility

The alignment default bound is the only behavioural change, and it affects only
callers who have explicitly opted into a non-`Positional` alignment mode on very
large sheets — who today get an abort. Degrading to positional with a diagnostic
is strictly better than that.

`Limits` gains a field; it is not `#[non_exhaustive]`-hostile because callers
construct it via `Default` and field assignment. Confirm during implementation
that adding a field does not break the documented construction pattern; if it
does, that is a finding to report before proceeding.

## 7. Testing and verification

- A fixture proving alignment degrades rather than aborting on a sheet that
  exceeds the product bound, and that the diagnostic is emitted.
- A test proving `Limits::hardened()` actually bounds each dimension.
- A test proving the input-size bound rejects before reading.
- `cargo deny check` green in CI, and demonstrated to fail on an injected
  banned dependency — per M1's standard, a gate never observed failing has not
  been shown to work.
- `#![forbid(unsafe_code)]` compiling.

## 8. Alternatives considered

- **Bound everything by default.** Safest on paper, but it breaks working
  callers on linear paths where the cost was predictable and expected. Rejected
  in favour of the superlinear/linear split.
- **Error instead of degrading on the alignment bound.** Simpler to implement,
  but makes an optional quality feature able to fail an otherwise valid
  comparison, and makes the bound a DoS vector. Rejected.
- **Defer bounds to the caller entirely.** This is the status quo, and it is
  what left every caller unprotected while the documentation said nothing.
  Rejected.

## 9. Risks

- The alignment product bound's default value is a judgement call. Too low and
  a legitimate alignment silently degrades; too high and the protection is
  theoretical. The default must be justified with a measurement, not chosen by
  intuition, and the diagnostic makes degradation visible either way.
- `cargo deny`'s ban list can produce false positives on transitive
  dependencies. Report them rather than widening the allowlist reflexively.

## 10. Acceptance criteria

1. `Limits::default()` bounds the alignment product; linear limits remain `None`.
2. Exceeding the alignment bound degrades to positional comparison and emits a
   dedicated diagnostic — it does not error and does not abort.
3. `Limits::hardened()` exists, is documented against the threat it addresses,
   and is tested.
4. Input size is bounded before `std::fs::read`, and `compare_bytes` no longer
   copies its input twice.
5. `#![forbid(unsafe_code)]` is in force.
6. `deny.toml` exists and is gated in CI, with advisories, bans, licenses and
   sources configured, and the gate demonstrated to fail on an injected
   violation.
7. The threat model document exists and meets §5.8, including the
   advisory-response policy.
