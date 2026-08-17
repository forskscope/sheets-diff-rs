# Handoffs — M7: measure, then change

**Not yet open.** This directory exists because M7's first unit had to be
written before the milestone could be proposed — its whole discipline is that
**the rest of M7's scope cannot honestly be written until something is
measured.**

Governed by RFC-024 (large-workbook memory), RFC-012 (progress, cancellation and
resource bounds), RFC-027 (benchmark and performance governance). All in
`done/`, all carrying named gaps.

## Why this milestone is shaped differently

Every previous milestone knew what it was fixing before it started. M4 knew the
eight statements that were false. M5 knew the three rules nothing checked. M6
knew the two MUSTs.

M7 does not, and pretending otherwise is how a performance milestone turns into
a refactor nobody asked for. Three of its four candidate items have the same
property: **we know a cost exists and not what it is.**

- `compare_bytes` copies its input. The threat model says this doubles peak
  memory. **Nobody has measured it.**
- `cell_map_to_align` clones every `CellValue` in both sheets. That is a real
  allocation and an unknown fraction of the total.
- Cancellation is polled once per sheet pair. RFC-024's status calls this a gap
  against acceptance criteria; RFC-012's own goal says *"cancellation checks at
  major pipeline stages"*, which a sheet pair arguably is. **The disagreement is
  unresolvable without knowing how long a caller actually waits.**

This project has guessed at a cause and been wrong twice — the synthetic-fixture
misdiagnosis, and the "G needs v3" claim. Unit 01 exists so M7 does not make it
a third time.

## Queue

| | Unit | Governing | Status |
|---|---|---|---|
| 01 | [Measure](./01-measure.md) | RFC-024, RFC-012 | **Ready** — gates everything else |
| 02 | v1.2-vs-v2 benchmark comparison | RFC-027 | To be written; separable, gates nothing |
| 03+ | Scope set by unit 01's report | — | **Cannot be written yet** |

Unit 02 moved here from M6 — it is measurement, not documentation. It is
independent of unit 01 and of everything else; its obstacle is building v1.2,
not deciding anything.

**Units 03 onward do not exist and will not be written speculatively.** When
unit 01 reports, its numbers decide which of the candidate items are worth
doing, in what order, and whether any of them is worth doing at all. That is
the milestone's point.

## The candidate items, and what would settle each

| Candidate | Settled by |
|---|---|
| Remove `compare_bytes`'s copy | Whether the copy is a material fraction of peak, and at what workbook size |
| Reduce peak by not holding both `CellMap`s | Attribution — how much of peak is the maps versus the alignment clone versus calamine's own buffers |
| RFC-024 §7's density choice (`Sparse`/`Dense`) | Whether `BTreeMap` overhead is material on dense sheets, measured |
| Finer cancellation polling | How long a caller waits after requesting cancellation on the largest realistic single sheet |
| Shared display address (G) | Not a measurement question — a design one, and additive on `#[non_exhaustive]` types |

## Standing constraints

- **Unit 01 changes no library code.** It measures. If it finds a defect,
  report it — do not fix it.
- **No optimisation before the report.** A change that looks obviously good is
  exactly what measurement is for; this project has been wrong about obvious
  before.
- **Report what the measurement does not capture.** Every method has a boundary
  — allocator-tracked bytes are not RSS, a single process is not a GUI under
  load. The boundary is part of the result.
- **The fixture corpus must not move.**
- Gates as always: fmt, clippy `-D warnings`, the scoped stdout gate, `deny`,
  MSRV 1.88, the full matrix.
