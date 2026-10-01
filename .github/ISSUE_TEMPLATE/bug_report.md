---
name: Bug Report
about: Something isn't working as expected
title: "[Bug] "
labels: bug
assignees: ''

---

**Describe the bug**
A clear and concise description of what happened.

**To Reproduce**
1. The exact command you ran (`sheets-diff old.xlsx new.xlsx --format ...`), or the library call
   and the `DiffOptions` you built.
2. The two input workbooks, or a minimal pair that still reproduces it — please attach them.
   If the real files can't be shared, describe the sheets/cells involved (types, formulas,
   sheet names) closely enough that a similar pair could be built.
3. What came back: the output, the error, or the panic.

**Expected behavior**
What you expected `sheets-diff` to report instead.

**Did it panic, or return an `Err`?**
This crate's design goal is to never panic on malformed or hostile input — only return a
`SheetsDiffError`. If you saw a panic, that is a defect in its own right, independent of whatever
triggered it: please include the full backtrace (run with `RUST_BACKTRACE=1`).

**Environment**
- OS: [e.g. Linux/macOS/Windows]
- `sheets-diff` version: [`sheets-diff --version`, or the version in your `Cargo.lock`]
- Features enabled: [default (none), `serde`, `chrono`, `cli`, or a combination]
- How you invoked it: the CLI command and flags, or the library call and options
- If this is a build/compile problem: `rustc --version`

**Additional context**
Anything else that might matter: relevant log output, whether it reproduces on every run or only
sometimes, or whether it worked in a previous version.
