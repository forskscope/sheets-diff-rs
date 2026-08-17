# Handoff 04 — Close the bypass

**Governing RFCs.** RFC-016, RFC-005, RFC-013 — same as unit 01
**Roadmap.** M5
**Sequence.** After unit 01 (merged). Independent of 02 and 03.

## Purpose

Unit 01's gate can be waved through with a one-line attribute, and clippy's own
error message tells you how. Close it, and correct one blind-spot claim that
turned out not to be a blind spot.

## Background

### F-K — the gate is overridable, and advertises it

Unit 01's check fails the build on a library stdout/stderr write. Its failure
message ends:

```
= help: to override `-D warnings` add `#[allow(clippy::disallowed_macros)]`
```

That help text is accurate. Verified during unit 01's review: a library function
carrying that attribute writes to stdout and the scoped gate exits **0**.

So the gate currently stops an accident and does not stop a decision. The next
person to hit it will be told by their own tooling exactly how to get past it,
at the moment they are most motivated to.

This is a more likely bypass than either blind spot unit 01 named. It needs no
knowledge of clippy internals, no macro trickery, and no indirection — it is
copy-paste from the error output.

**This is my scoping failure, not the implementer's.** Unit 01's non-change
scope said *"Do not change any code under `src/`"*, and the fix lives in
`src/lib.rs`. They could not have done it.

### F-J — a stated blind spot that isn't one

Unit 01's review request §3 lists, as a limitation:

> **Indirection through a stored function pointer or trait object** — e.g.
> `let f: fn() -> std::io::Stdout = std::io::stdout; f()` … may not be
> recognized as the same disallowed method.

It is recognised. Verified with that exact example: the gate fails at the column
of the right-hand side, because `disallowed_methods` fires where the function is
**named**, not only where it is called. Naming is unavoidable in any indirection
starting from `std::io::stdout`, so the route is closed.

That section is what a future maintainer reads when deciding how far to trust
the gate. An overstated weakness there invites someone to conclude the check is
weaker than it is.

## Change scope

`src/lib.rs` — the lint attribute and its comment, nothing else — and
`CHANGELOG.md`.

The F-J correction belongs in the CHANGELOG entry. The review-request and review
files under `.git-exclude/` are the historical record of what was submitted and
what I found; **do not edit them.** The claim was reasonable when written and
the correction is recorded here and in the changelog, not by rewriting what was
said.

## Non-change scope

- **No behaviour change.** The attribute is a lint directive; it must not alter
  what the crate does.
- Do not touch `.github/clippy-no-stdout/clippy.toml`'s lint list or the CI
  step. Unit 01's mechanism is correct; this only removes the override.
- Do not add `#[allow]` anywhere to make something pass. That is the thing being
  closed.

## Required implementation

1. **Add to `src/lib.rs`:**

   ```rust
   #![forbid(clippy::disallowed_macros, clippy::disallowed_methods)]
   ```

   `forbid` cannot be overridden by an inner `#[allow]` — the attempt becomes
   `error[E0453]`, a hard compile error rather than a lint that can be demoted.

   **I verified this works before specifying it**, including that it is free
   elsewhere: with no config loaded, the lints have no configured paths, so
   `cargo build --all-features` and the unscoped `cargo clippy --all-targets
   --all-features -- -D warnings` both still exit 0. Confirm that independently
   rather than taking my word for it — if either now fails, stop and report.

2. **Place it deliberately.** `src/lib.rs:1` already carries
   `#![forbid(unsafe_code)]`. These belong together and a short comment should
   say what the new one is for and which unit built the gate it protects.

3. **Record the F-J correction in the CHANGELOG entry** — indirection through a
   named function pointer *is* caught, because the lint fires where the function
   is named. State it as part of what the gate covers, not as an erratum.

## Required tests

**Demonstrate the bypass is closed**, the same way unit 01 demonstrated the gate
works — this milestone does not accept a check whose effect has only been
argued:

1. Add a library function with `#[allow(clippy::disallowed_macros)]` and a
   `println!`. Run the scoped gate. Capture `error[E0453]`. Revert.
2. Confirm `cargo build --all-features` and the unscoped clippy step both still
   pass on the clean tree with the `forbid` in place.

No test file is needed — this is a compile-time property, and a transcript is
the evidence.

## Acceptance criteria

1. `#![forbid(clippy::disallowed_macros, clippy::disallowed_methods)]` is in
   `src/lib.rs`, with a comment naming its purpose.
2. An `#[allow]` attempt on a library stdout write produces `error[E0453]`,
   demonstrated with a transcript.
3. `cargo build --all-features` passes with the `forbid` present.
4. The unscoped `cargo clippy --all-targets --all-features -- -D warnings`
   passes unchanged.
5. The scoped gate still passes on the clean tree.
6. The F-J correction is recorded — indirection through a named function
   pointer is caught, not missed.
7. No behaviour change; no file under `src/` changed except `lib.rs`'s attribute.
8. Fixture corpus byte-identical.
9. CHANGELOG records this under `### Added`, folded into or adjacent to unit
   01's gate entry — a reader should not have to assemble the gate's real
   strength from two separate bullets.
10. Gates green, full matrix, CI green including the scoped step.

## Prohibited shortcuts

- Do not use `deny` instead of `forbid`. `deny` is overridable by `#[allow]`,
  which is the entire defect.
- Do not close this by removing the `help` line from the error output. The
  message is clippy's and it is honest; the fix is to make the advice fail.
- Do not add the attribute to individual modules. Crate-level is the point —
  a per-module attribute is a list someone must remember to extend.

## Known risks

- `forbid` on a tool lint is strict by design: if any dependency's macro
  expansion or generated code inside this crate legitimately trips one of these
  lints, the crate stops compiling and there is no local escape hatch. Nothing
  currently does — the clean tree compiles — but if you find a case, **stop and
  report** rather than downgrading to `deny`. The right answer would be to
  narrow the configured paths, not to make the gate overridable again.
- The attribute must sit with the other crate-level attributes, before any item,
  or it will not parse.

## Required evidence

- The `error[E0453]` transcript
- `cargo build` and unscoped-clippy exit codes with `forbid` present
- Scoped gate green on the clean tree
- `git status` showing only `lib.rs` under `src/`
- CI run link

## Review request format

Per development policy §9.2, plus the `error[E0453]` transcript and the two
"costs nothing elsewhere" confirmations.
