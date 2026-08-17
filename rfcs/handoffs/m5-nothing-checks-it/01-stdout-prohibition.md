# Handoff 01 — The prohibition nothing enforces

**Governing RFCs.** RFC-016 (security, privacy, no side effects), RFC-005
(error diagnostics), RFC-013 §"CLI may write to stdout/stderr; library core
must not"
**Roadmap.** M5
**Sequence.** Any. This is the unit that carries the milestone.

## Purpose

Convert "the library core must not write to stdout or stderr" from a rule
four RFCs state into a property the build enforces.

## Background

Four RFCs in `done/` state the prohibition:

- RFC-016 §"Guarantee no stdout/stderr writes from library core"
- RFC-005 §"Ensure core library code has no stdout/stderr writes"
- RFC-013 line 82 — *"CLI may write to stdout/stderr; library core must not."*
- RFC-032 §"Never print to stdout/stderr or panic for ordinary input failures"

**Nothing checks any of them.** There is no CI step, no test, and no lint
configuration that would object to a `println!` added to `src/normalize.rs`
tomorrow.

RFC-016's own Status line already records this — *"Deferred: no static check
enforces the `println!`/`eprintln!`/`dbg!` prohibition in CI"* — so the gap is
known and has simply never been closed.

**The source is currently clean.** The only writes in `src/` are in `main.rs`
(the CLI, where they are correct and RFC-013 explicitly permits them) and one
inside a `//!` doc-comment example in `lib.rs`. **This unit adds a gate; it does
not fix a violation.** If your check reports anything else in `src/`, that is a
finding — stop and report it rather than fixing it here.

### The record correction

`rfcs/done/1.2/006-regression-fixture-and-ci-hardening.md` line 11 states that
v1.2 delivered *"the CI stdout-hygiene check."*

**It did not.** At tag `1.2.0` the only workflow present is
`release-executable.yaml`, there is no test asserting the prohibition anywhere
in the tree, and `git grep` for `stdout` across the whole `1.2.0` tree returns
documentation only. The check was never built.

This is M4's defect class in an RFC that is filed as `done/`. It is corrected
here rather than in a documentation milestone because **this unit is what makes
the sentence true** — the honest repair is to build the check and annotate the
claim, in the same change.

## Change scope

`.github/workflows/ci.yaml` (or a script it calls),
`rfcs/done/1.2/006-regression-fixture-and-ci-hardening.md` (the annotation),
`rfcs/done/016-security-privacy-and-no-side-effects-policy.md` (its Status
line's deferral note), `CHANGELOG.md`.

Add a test file or a script if your chosen mechanism needs one.

## Non-change scope

- **Do not change any code under `src/`.** Nothing there violates the rule.
- Do not touch `main.rs`'s writes. They are correct and permitted.
- Do not add a runtime mechanism (a logging abstraction, a feature flag to
  silence output). The rule is that the code is not there, not that it can be
  turned off.

## Required implementation

1. **A check that fails when the library core writes to stdout or stderr**,
   covering `println!`, `eprintln!`, `print!`, `eprint!`, `dbg!`, and direct
   `std::io::stdout()` / `std::io::stderr()` use.
2. **It must run in CI** and fail the build, not warn.
3. **`src/main.rs` must be excluded**, and the exclusion must be narrow and
   explicit. RFC-013 permits the CLI to print; it permits nothing else to.
4. **Doc comments and strings must not produce a false positive.** `lib.rs`'s
   `//!` example contains `println!` legitimately. A check that forces us to
   mangle a doc example to stay green is a worse check than none — it trains
   people to work around it.
5. **Annotate `rfcs/done/1.2/006`'s claim** — the check it describes did not
   exist, and now does. Annotate; do not rewrite the historical sentence. This
   project's convention, applied three times in `CHANGELOG.md` already.
6. **Update RFC-016's Status line** to record that this half of its deferral is
   closed. Its other half — source-path privacy — is unit 02, and stays open
   until then. Do not mark the whole line resolved.

**Mechanism is your choice and the choice is the interesting part.** A `grep` in
a CI step is the obvious answer and it is legitimate, but it is a string match
over source text: it cannot see through a macro, and it will trip on the word in
a comment. `clippy` has `disallowed_macros`, configurable in `clippy.toml`,
which understands what is actually a macro invocation and is enforced by a gate
this project already runs with `-D warnings`. There may be others.

Pick one, and **say in the review request what your mechanism cannot catch.**
Every option has a blind spot; the milestone is about knowing what it is rather
than assuming there isn't one.

## Required tests

**The check must be demonstrated to fail on a violation.** Add a `println!` to a
library source file, run the check, capture the failure, remove it. Include the
transcript.

This is not optional and it is not ceremony: an unverified gate is the exact
thing this milestone exists to eliminate, and adding one would be a self-refuting
result. A gate that has only ever been observed passing on a clean tree is
indistinguishable from a gate that does nothing.

## Acceptance criteria

1. A CI check fails the build when library code writes to stdout or stderr.
2. `src/main.rs` is excluded, narrowly and explicitly.
3. The check does not fire on `lib.rs`'s doc-comment example, and no doc example
   was altered to accommodate it.
4. The check is demonstrated failing on a real violation, with a transcript.
5. The review request states what the chosen mechanism cannot catch.
6. `rfcs/done/1.2/006`'s false claim is annotated, not rewritten.
7. RFC-016's Status line records this deferral closed and unit 02's still open.
8. No file under `src/` changed.
9. CHANGELOG records the gate under `### Added` — this is not a fix, nothing
   was broken.
10. Gates green, full matrix, CI green including the new check.

## Prohibited shortcuts

- **Do not add a check that only passes.** See "Required tests".
- Do not exclude directories beyond `main.rs` to make it pass. If something else
  trips it, that is a finding.
- Do not edit a doc example to satisfy the check.
- Do not rewrite RFC 1.2/006's sentence to say the check exists now. It did not
  exist then; the record should show both.

## Known risks

- A naive `grep -r "println!" src/` matches the doc comment in `lib.rs` and the
  legitimate writes in `main.rs`. Handling those two by exclusion is fine;
  handling them by weakening the pattern until nothing matches is not.
- `clippy::disallowed_macros` needs a `clippy.toml`, and the MSRV job runs
  `cargo check`, not clippy — confirm which CI jobs actually evaluate it, so
  the gate is not silently absent from the legs you assume cover it.

## Required evidence

- The check, and where it runs
- The failure transcript from a deliberately introduced violation
- A statement of the mechanism's blind spot
- `git status` showing nothing under `src/` changed
- CI run link

## Review request format

Per development policy §9.2, plus the failure transcript and the blind-spot
statement.
