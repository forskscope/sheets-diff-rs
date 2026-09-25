# Handoffs — M9: reaching the code, and a record that agrees with itself

**OPEN 2026-09-26.** Authorized 2026-09-24; opened now that 3.0.0 has shipped
and M10 is closed.

**One theme:** the checks and corpora that are supposed to guard this crate do
not reach the code they guard. A fuzz corpus that cannot get past the zip
header. A fixture corpus with no case for either alignment warning. Records that
describe a crate we do not have. And, found by the health audit, a CI
configuration that runs one build twice and several checks not at all.

## Queue

| | Unit | Item | Nature |
|---|---|---|---|
| 00 | [CI asserts the standing properties](./00-ci-asserts-the-standing-properties.md) | audit; F-2, F-3 | **First** — makes every later unit cheaper to verify |
| 01 | The fuzz corpus cannot reach the sheet reader | D1 | The substantive one |
| 02 | Deviations from `project-instructions-rust.md` | B1–B4 | |
| 03 | The remaining record corrections | C4, C6–C9, E, F-2 | |
| 04 | `FormulaUnavailable` is pushed once per cell | O4 | **Measure before deciding** |
| 05 | The corpus cannot reach either alignment warning | O-B | D1's shape, in the fixtures |
| 06 | The CLI applies no `Limits` | F-3 | Measure with 04 |

**Order: 00 first.** The rest in any order; 04 and 06 want measuring together.

**Unit 00 is first for a practical reason.** It moves standing checks off the
local machine, and M9's own units will otherwise accumulate scratch the way
M10's did — 154 GB in `target/scratch` before the audit found it.

## What M9 is not

- **Not a release.** Six of its seven units are invisible to a caller and land
  on `main` as they are done. Its one observable unit (O-A, `DiagnosticLocation`)
  moved into M10 and shipped in 3.0.0.
- **Not a place for new API.** Anything that changes the public surface is a v4
  question or a separate RFC.

## Standing constraints

- Gates as always, plus **`cargo check --manifest-path fuzz/Cargo.toml --bins`**
  — `fuzz/` is a separate crate no other gate compiles
  (`.git-exclude/rules/003-where-a-removal-must-be-swept.md`).
- **One scratch target dir per sweep, not one per command**, and delete it when
  the evidence is captured — rule 002 as narrowed 2026-09-26. The evidence is
  the captured output, not the directory.
- Every fix arrives with a test that fails without it, demonstrated by removing
  the specific code under test — never by reverting a file.
