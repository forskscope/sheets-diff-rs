# Handoffs — M2: the 2.3.0 security and integrity release

Execution queue for roadmap milestone **M2**. Most units are governed by
[RFC-035](../../proposed/035-resource-safety-and-supply-chain-governance.md);
the dependency migration and the correctness defects are governed by RFCs
already in `done/`, noted per unit. Handoffs have no lifecycle state of their
own and must not redefine their governing RFC.

## Queue

| | Unit | Governed by | Depends on |
|---|---|---|---|
| 01 | [calamine 0.36 compatibility spike](./01-calamine-036-spike.md) | RFC-026 (investigation only) | — |
| 02 | MSRV 1.88 + calamine 0.36 | RFC-026, RFC-031 | 01 |
| 03 | Supply-chain gates (`deny.toml`, audit, path assertions) | RFC-035 §5.5 | 02 |
| 04 | Resource bounds and `forbid(unsafe_code)` | RFC-035 §5.1–5.6 | 02 |
| 05 | Integrity-affecting correctness defects | RFC-010, RFC-011, RFC-018, RFC-019 | 04 |
| 06 | Threat model, advisory policy, CHANGELOG corrections | RFC-035 §5.7–5.8, RFC-016 | 03, 04, 05 |

## Why only unit 01 is written yet

Unit 02's shape genuinely depends on what unit 01 finds. If calamine 0.36
compiles against our usage unchanged, 02 is a version bump plus the eight
`clippy::collapsible_if` fixes. If the API delta is larger — or if the effective
MSRV floor across `zip` 8.6 and `quick-xml` 0.41 turns out to be above 1.88 —
then 02 is a different unit, and roadmap decision D0's approved number needs
revisiting with the owner before anything lands.

Writing 02 now would mean guessing. Units 03–06 are sketched above and will be
written as the queue advances; their scope is already fixed by RFC-035's
acceptance criteria.

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
