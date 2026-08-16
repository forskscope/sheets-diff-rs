# Handoffs — M2: the 2.3.0 security and integrity release

Execution queue for roadmap milestone **M2**. Most units are governed by
[RFC-035](../../accepted/035-resource-safety-and-supply-chain-governance.md);
the dependency migration and the correctness defects are governed by RFCs
already in `done/`, noted per unit. Handoffs have no lifecycle state of their
own and must not redefine their governing RFC.

## Queue

| | Unit | Governed by | Depends on |
|---|---|---|---|
| 01 | [calamine 0.36 compatibility spike](./01-calamine-036-spike.md) | RFC-026 (investigation only) | — |
| 02 | MSRV 1.88 + calamine 0.36 | RFC-026, RFC-031 | 01 |
| 03 | [Supply-chain gates](./03-supply-chain-gates.md) | RFC-035 §5.5 | 02 |
| 04 | [Resource bounds and `forbid(unsafe_code)`](./04-resource-bounds.md) | RFC-035 §5.1–5.6 | 02 |
| 05 | [Integrity-affecting correctness defects](./05-integrity-defects.md) | RFC-010, RFC-011, RFC-018, RFC-019 | 04 |
| 06 | Threat model, advisory policy, CHANGELOG corrections | RFC-035 §5.7–5.8, RFC-016 | 03, 04, 05 |

## Progress

Units **01 and 02 are complete and merged** (`main` at `059ad6f`, CI 17/17).
D0's contingency is discharged, MSRV is 1.88, and both `quick-xml` advisories
are cleared.

RFC-035 was accepted 2026-08-16, so units 03–06 are live. **03 and 04 are
written and can be taken in either order** — they are independent, and either
may start immediately.

**Units 03 and 04 are approved** (PR #10, 18/18) and **unit 05 is written**. It
carries the highest-severity finding in the original audit: `DateTimeIso` and
`DurationIso` values that always compare equal — proven by execution, not
inferred, with `2024-01-01` reported identical to `2099-12-31`.

**06 remains unwritten deliberately.** It documents what the other units
actually built, and writing it first would document intent rather than fact. It
also owns `src/objects.rs`'s stale "calamine 0.35" strings, which are embedded
verbatim in all seven goldens — deliberately kept out of unit 05 so that if a
golden moves there, it moved because behaviour changed.

## Sequencing constraints

These are forced, not preferences:

1. **01 before 02.** D0 approved MSRV 1.88 *contingent on a build proving 1.88
   is the true effective floor*. That proof is unit 01.
2. **MSRV before calamine, inside 02.** Edition 2024 uses the MSRV-aware
   resolver; bumping calamine while 1.85 is still declared can silently resolve
   back to 0.35 and produce a green build with the advisory chain intact.
3. **02 before 03.** Gating a dependency tree that is about to change wastes the
   work and produces findings against a tree we are replacing.
4. **04 before 05.** Both touch `align.rs`; the bound and the fallback path land
   before the coordinate-collision defect is fixed on top of them.
5. **06 last.** The threat model documents what the other units actually built,
   not what they intended to build.

## Standing constraints for all units

- **No release is cut until every unit is closed.** M2 ships as 2.3.0 in one
  piece; the dependency fix alone would be a false all-clear (roadmap R1).
- **No suppression.** No `#[allow]`, no `continue-on-error`, no narrowing the
  matrix. A red check is a finding to report.
- **Every guard gets demonstrated failing.** M1's standard applies to every new
  gate this milestone adds: a guard never observed firing has not been shown to
  work.
- **The fixture corpus is a regression detector.** If a golden changes,
  that is a finding to report — never bless over it without saying what changed
  and why it is correct.
- Every unit ships with the evidence its acceptance criteria name. "Tests pass"
  without the output is not evidence.
