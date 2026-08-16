# Handoff 03 — Exit code 3, and the CLI test that never existed

**Governing RFC.** RFC-013 (output formatters, CLI and exit codes)
**Roadmap.** M4 — the unit that makes this release a minor rather than a patch
**Sequence.** Any.

## Purpose

Emit the exit code RFC-013 specifies, and test the exit codes at all.

## Background

RFC-013 line 90: *"Invalid/corrupt inputs produce exit code 3."* No exit code 3
exists anywhere in the crate. `src/main.rs` emits:

- `exit(2)` for invalid CLI options
- `exit(1)` when differences are found
- `exit(2)` for every comparison error, corrupt input included

So a caller cannot distinguish "this file is not a workbook" from "you passed a
bad flag". Both are 2.

Separately, **no test invokes the built binary at all** — `tests/` contains no
`Command::new` or equivalent. The exit codes have never been verified by
anything, which is why the gap survived from RFC-013's acceptance to now.

## Change scope

`src/main.rs`, `tests/` (a new CLI test), `Cargo.toml` if a dev-dependency is
needed for subprocess testing, `README.md` and/or `docs/` where exit codes are
documented, `CHANGELOG.md`.

## Non-change scope

Do not change the library. Exit-code mapping belongs in `main.rs`; if it seems
to need a new error variant or a classification helper in `src/`, **stop and
report** — that would be a public API change and this unit is not the place.

Do not change codes 0, 1 or 2's existing meanings beyond narrowing 2 by moving
the corrupt-input subset to 3.

## Required implementation

1. **Decide which errors are "invalid/corrupt input", and justify the line.**
   `SheetsDiffError` distinguishes `OpenWorkbook` (with `OpenErrorKind`:
   `NotFound`, `PermissionDenied`, `NotXlsx`, `Corrupt`, `Locked`, `Other`),
   `ReadSheet`, `UnsupportedFormat`, `EncryptedWorkbook`, `InvalidOptions`,
   `Cancelled`, `LimitExceeded`, `Internal`.

   Not all are input problems. `NotFound` and `PermissionDenied` are arguably
   environment, not corruption; `InvalidOptions` is caller error; `Cancelled`
   and `LimitExceeded` are neither. **State the mapping you choose and why**, in
   the review request and in the documentation. This is the substance of the
   unit — the code is a `match`.
2. **Document the full exit-code contract** where a CLI user will find it. An
   exit code nobody documents is barely better than one nobody emits.
3. **Add the subprocess test.** This closes one of M5's three findings early,
   deliberately: implementing an exit code without testing it would be the same
   defect class this milestone exists to remove. Cover every code the CLI can
   produce, not only the new one.

## Required tests

A test that runs the built binary and asserts its exit status for: no
differences (0), differences found (1), invalid options (2), corrupt input (3),
and whatever else your §1 mapping produces. `tests/fixtures/corrupt/not_a_zip.xlsx`
already exists for the corrupt case.

Prefer `std::process::Command` with `env!("CARGO_BIN_EXE_sheets-diff")` over a
new dev-dependency; if you add one, justify it against the `deny.toml` gate.

## Acceptance criteria

1. Corrupt/invalid input exits 3; the mapping is stated and justified.
2. Every exit code the CLI emits is covered by a subprocess test.
3. The contract is documented where a CLI user will see it.
4. The library is unchanged — `git status` shows nothing under `src/` except
   `main.rs`.
5. CHANGELOG records this as a **behaviour change to the CLI contract**, not a
   bugfix footnote. A script matching `2` for corrupt input will now see `3`.
6. The fixture corpus does not move.
7. Gates green, including the `cli` feature leg.

## Prohibited shortcuts

- Do not map every error to 3 to make the criterion pass. The point is that 2
  and 3 mean different things.
- Do not add the exit code without the test. That is the defect this milestone
  removes, committed freshly.
- Do not reach into `src/` for a classification helper without reporting first.

## Compatibility constraints

**This is why M4 releases as 2.4.0 rather than 2.3.1.** Exit codes are an
interface. A consumer matching `2` for "operational error" will see `3` for the
corrupt-input subset after this change. It is the correct behaviour per RFC-013
and it is still a contract change — say so plainly in the CHANGELOG rather than
burying it.

## Known risks

- The mapping decision is genuinely open and reasonable people would draw it
  differently. That is why §1 asks for justification rather than a rule.
- Subprocess tests can be flaky about binary paths across platforms; the CI
  matrix includes Windows, so verify there rather than assuming.

## Required evidence

- The mapping, with reasoning
- The subprocess test and its output, on both platforms via CI
- `git status` showing only `main.rs` under `src/`
- Corpus hash unchanged
- CI run link

## Review request format

Per development policy §9.2, plus the §1 mapping and its justification.
