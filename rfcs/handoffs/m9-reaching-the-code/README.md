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
| 01 | [The fuzz corpus cannot reach the sheet reader](./01-the-corpus-cannot-reach-the-reader.md) | D1 | The substantive one |
| 02 | [Deviations from `project-instructions-rust.md`](./02-the-rule-deviations.md) | B1–B4 | |
| 03 | [The remaining record corrections](./03-the-records-disagree.md) | C4, C6–C9, E, F-2 | |
| 04 ✅ | ~~`FormulaUnavailable` is pushed once per cell~~ **→ promoted to and closed as [f135](../f135-a-diagnostic-per-numeric-cell/01-say-it-once-and-mean-the-guard.md)** | O4 | Fixed 2026-09-30: guard renamed `sheet_has_formulas`, computed from whether a formula actually attached, not "the formula pass completed without error." Reports once per sheet per side with a count, not once per cell |
| 05 | [The corpus cannot reach either alignment warning](./05-the-corpus-cannot-reach-its-own-warnings.md) | O-B | D1's shape, in the fixtures **Also owns a mixed formula/plain-numeric scenario** — f135's review found the corpus no longer reaches `formula_unavailable` at all: the two goldens that produced it were doing so by accident, on sheets with no formulas, and the four formula-bearing fixtures are two cells each with formulas on all of them. Second instance of this unit's own shape |
| 06 | [The CLI applies no `Limits`](./06-the-cli-bounds-nothing.md) | F-3 | Measure with 04 |

**Order: 00 first.** The rest in any order; 04 and 06 wanted measuring together.

**Resumed 2026-09-30. All four remaining handoffs are written**, with every item re-verified on the
current tree rather than taken from the audit — several had moved, two were already fixed, and C4 had
got worse. **Recommended order, by value:**

1. **[06](./06-the-cli-bounds-nothing.md)** — the only one with a user-facing edge, and it is a
   measurement before it is a change. 04 is done (as f135), so it can be measured alone now.
2. **[05](./05-the-corpus-cannot-reach-its-own-warnings.md)** — three diagnostics no fixture
   produces. This milestone's own shape, in the fixtures.
3. **[03](./03-the-records-disagree.md)** — the false statements. Three items need a ruling rather
   than a fix, and it says which.
4. **[02](./02-the-rule-deviations.md)** — the largest churn and the least behaviour. Last on
   purpose, and B3 stays out of scope per the ruling at m8-03's review.

**PAUSED 2026-09-29, after unit 01.** Unit 01 did what the milestone was for — it made a check
reach the code it guards — and the check immediately found a 512-byte input that aborts the calling
process, in every released version. That is now ahead of the rest of this queue, and the next
release waits for it (owner, 2026-09-29):
`rfcs/handoffs/f132-a-512-byte-file-aborts-the-process/01-decline-before-delegating.md`.
M9 resumes at unit 02 once it ships.

**Amended 2026-09-30, after the owner asked whether M9's incompletion affects our consumer.** It
does, through one unit. Unit 04 was filed as an internal measurement item and is not one: the
diagnostic it concerns is **on by default** and its volume grows with the cell count, so a consumer
comparing ordinary numeric spreadsheets pays it whether or not they know the option exists. It is
promoted to **f135** and runs before the rest of this queue. Units 02, 03, 05 and 06 are confirmed
internal — code layout, record corrections, fixture coverage, and CLI flags that a library consumer
does not use — and none of them changes what a caller gets.

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
