# Handoff 02 — A mode that never warns

**Unit:** confidence-that-measures-counts 02. **Added 2026-10-05.**
**Scoped by:** the architect. **Read `README.md` first, and take unit 01 before this.**
**Semver:** minor — a new `DiagnosticKind` variant is additive on a `#[non_exhaustive]` enum.

## Purpose

`src/align.rs:265`:

```rust
fn row_signature_alignment(
    old_cells: &CellMap,
    new_cells: &CellMap,
    sample_cols: Option<&[u32]>,
    cancellation: Option<&dyn Cancellation>,
    _diagnostics: &mut Vec<Diagnostic>,
) -> Result<RowMapping, SheetsDiffError> {
```

The parameter is underscore-prefixed. **`AlignmentMode::RowSignature` emits no diagnostic, ever.**
Measured on the README's second reproduction: `sheet diagnostics = 0`, on a sheet where two rows
were paired indistinguishably and two spurious changes were reported.

`RowKey` warns about duplicate keys and missing keys. The signature path, which has exactly the same
ambiguity available to it, says nothing.

## Why this matters more than the asymmetry suggests

A consumer has told us their plan: run `Positional`, and when it reports a cascade also run
**`RowSignature`**, keep whichever reports fewer changed cells, and tell the user which alignment was
kept. **`RowSignature` is the mode they are adopting**, and it is the one that cannot tell them
anything is wrong with the pairing it produced.

## Change scope

- `src/align.rs` — the signature path; the underscore comes off or the parameter comes out.
- `src/model.rs` — a new `DiagnosticKind` variant, if that is the answer.
- `examples/gen-fixtures.rs`, `tests/fixtures/generated/` — a scenario, if a new kind lands.
- `rfcs/accepted/036-coverage-obligation-and-the-fixture-matrix.md` §5.4 — **mandatory** if a new
  kind lands: every `DiagnosticKind` needs a scenario whose assertion fires on it.
- `tests/integration.rs`, and the hardcoded corpus-count assertions if a scenario is added.
- `CHANGELOG.md`.

## Non-change scope

- Do not change which rows the signature matcher pairs.
- Do not revisit `confidence`. Unit 01.
- Do not make the signature path emit `duplicate_alignment_key` **if that code's payload or message
  does not fit signatures.** Its message names *"alignment keys"* and LCS over keys. Reusing a code
  whose wording describes something else is how a diagnostic ends up meaning two things.

## Required implementation

**1. Decide whether the parameter is dead or the diagnostics are missing.** Two honest outcomes:

- the signature path has ambiguity worth reporting, and we report it; or
- it genuinely has nothing to say, and the parameter should be **removed** rather than left
  underscore-prefixed as a standing invitation to assume otherwise.

Unit 01 asked you to establish whether colliding signatures are detected anywhere. **Start from that
answer.** Do not guess which outcome is right — the README's measurement shows an indistinguishable
pairing producing two spurious changes with no warning, which is evidence for the first, but
"detectable" and "detected" are different and you have to check.

**2. If a diagnostic is right, decide code reuse versus a new variant, and justify it.** My
inclination is a new variant, because `duplicate_alignment_key`'s message is written about keys and
a signature is not a key — but that is a judgement about wording, and if you can make one code
honest for both, say so. **A new variant brings RFC-036 §5.4's obligation with it**: a fixture whose
assertion fires on the diagnostic's payload, not on its `code()`.

**3. If the parameter is dead, remove it** and say in the review why the mode has nothing to report,
in terms someone can check in a year. A removed parameter on a private function is not an API
change; leaving a misleading `_diagnostics` in place is a trap for the next reader.

**4. Propose before implementing.** One paragraph: which outcome, and if a diagnostic, its code,
payload and message.

## Required tests

1. **The README's second reproduction produces the diagnostic** — asserting its payload fields, not
   just `code()` — or, if the outcome is "nothing to report", a test pinning that the signature path
   produces no diagnostic, with a comment saying that is deliberate.
2. **`RowKey`'s existing diagnostics are unchanged.** Find the existing tests; do not write new ones
   over the top.
3. **`every_code_in_the_table_is_producible_and_nothing_else_is`** still passes. If a variant lands,
   it must be producible by a committed fixture.
4. **Failing-first** for 1.

## Acceptance criteria

1. The outcome decided, proposed, and agreed before implementation.
2. No `_diagnostics` parameter left in the file — either used or gone.
3. If a variant landed: RFC-036 §5.4 updated, a fixture, a payload-level assertion, and the corpus
   count assertions bumped.
4. The tests above, with failing-first where required.
5. Gates green, rule 003, one scratch dir, deleted. Nothing committed.
6. `cargo public-api` showing the additive variant and nothing else — detached worktree.

## Prohibited shortcuts

- Do not leave the underscore. That is the defect: a parameter that looks threaded and is not.
- Do not reuse a diagnostic code whose message describes a different thing.
- Do not add a scenario that produces the code incidentally. §5.4 forbids it in writing.
- Do not improve the signature matcher.

## Known risks

**1. "Nothing to report" is a legitimate outcome and will feel like giving up.** It is not, provided
you can say why and the parameter goes. What is not acceptable is leaving the underscore and
reporting that the mode is fine.

**2. A new diagnostic on a previously silent path is a visible behaviour change.** A consumer who
counts diagnostics, or treats any warning as a failure, will see new output. Note it in the
CHANGELOG in those terms.

**3. The corpus-count assertions.** M9 unit 05 had to bump several in more than one file. Expect it
if you add a scenario, and keep the note that `sheet_match_classification.rs`'s baseline rows
postdating the 2.6.0 binary are documented as not sourced from it.

## Required evidence

Under `.git-exclude/review-request/confidence-02-a-mode-that-never-warns/evidence/`:

1. Whether colliding signatures are detectable, established from the code.
2. The proposal, and my agreement.
3. The diagnostic firing on its payload, or the pinned silence with its reasoning.
4. RFC-036 §5.4 and the fixture, if a variant landed.
5. `cargo public-api`, detached worktree. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/confidence-02-a-mode-that-never-warns/README.md`, with:

- Which outcome, and the evidence for it.
- The diagnostic's code, payload and exact message, if one landed — I review the wording.
- Whether `duplicate_alignment_key` could honestly have covered it, and why you ruled it in or out.
- How long the `_diagnostics` parameter has been unused. `git log -L` on the signature function will
  say. If it was threaded and then stopped being used, that is a different finding from never having
  been used, and I want to know which.
