# Handoff 00 — CI asserts the standing properties

**Governing.** ROADMAP §6; `.git-exclude/decisions/004-where-verification-should-run.md`
**Roadmap.** M9 — first unit
**Sequence.** First. It makes every later unit cheaper to verify.

## Purpose

Several checks we run by hand on every unit are not in CI at all, and two CI
legs are the same build. CI runs **18 jobs in about four minutes** and has idle
capacity; the local machine has been doing work that need not block a person.

**The line this unit draws:** a check that asserts a **standing property** of the
crate belongs in CI. A check that proves **this change is correct** belongs with
the change, and stays local.

## Background

From the audit (decision 004). Not in the workflow at all:

| Missing | Note |
|---|---|
| `cargo doc -D warnings` | run on every local sweep; **absent from CI** |
| nightly doctests | the thirteen `compile_fail,EXXXX` pins are **not enforced on stable** — the code is decoration there. Verified both ways at M10 unit 10's review |
| `cargo publish --dry-run` | would catch a packaging fault before a cut rather than during one |
| `cargo public-api` diff | used once by hand in M10 unit 10, where it was the strongest oracle of the milestone: it confirmed the nine removals independently **and found a gap in the architect's own table** |
| install check | `cargo install --features cli` + `--format json` populating `iso`; caught a real defect in M8 |

Already present and wrong:

- **F-2** — the `cli` and `serde,chrono,cli` matrix legs are **the same build**
  since `cli` began implying both. One configuration compiled twice, per OS.
- **F-3** — the `msrv` job runs `cargo check --all-features` only. **Nothing in
  CI builds a minimal library at MSRV.**

## A correction to the audit, which changes one item

Decision 004 recommended putting `benches/memory.rs` in CI. **That was wrong and
this handoff supersedes it.**

- **The memory *guard* is already in CI.** `tests/streaming_read.rs` asserts
  `peak < 64 MiB` and runs in the test matrix.
- **`benches/memory.rs` is a report generator by design** — its own header says
  "no assertions on numbers (only on the harness's own accuracy)". Its figures
  are hardware-specific, so a CI run produces *different* numbers and validates
  nothing.

**What is portable is a relationship, not an absolute.** M7 unit 04's result —
after deleting the alignment clone, `RowKey` peak **equals** `Positional` peak,
delta exactly 0 — holds on any machine. It is quoted in `performance.md` and
checked today only by a person reading bench output.

**So: move the portable relationships into `tests/`, where CI already runs
them.** Do not add a bench job.

## Change scope

- `.github/workflows/ci.yaml`
- `tests/` — the memory-relationship test
- `docs/src/maintainers/performance.md` — see item 7
- `CONTRIBUTING.md` — if it lists the CI legs
- `CHANGELOG.md` — `### Documentation`, or nothing if you judge it internal

## Non-change scope

- **Do not weaken an existing job** to make room. If the workflow gets slow, say
  so and stop; four minutes is the budget and we are nowhere near it.
- Do not add a job that cannot fail. A job that only prints is not a gate; if a
  check cannot be made to fail meaningfully, **say so and leave it out**.
- Do not move mutation matrices or A/B build comparisons into CI. They are
  per-change evidence and CI mutating its own source is the wrong shape.
- Do not change MSRV, features, or any crate code beyond the new test.

## Required implementation

1. **`cargo doc -D warnings`** — add to the `lint` job or its own. It is already
   a gate in practice.
2. **Nightly doctests.** `cargo +nightly test --doc --all-features`. The
   workflow already installs nightly for `fuzz-smoke`, so this is one step on a
   toolchain we already provision and whose flakiness we already accept.
   **State in the job's name or a comment that its purpose is enforcing the
   `compile_fail` error codes**, which stable ignores — otherwise it looks like a
   duplicate of the stable doctest run and someone deletes it.
3. **`cargo publish --dry-run`**, on tag pushes. Check the packaged file list
   contains no `.git-exclude/` entry. Fail on either.
4. **`cargo public-api` diff against the last release tag.** This is the item
   with a design decision in it, so decide it and say why:

   A job that merely prints the diff is decoration. **The gate I would propose:
   fail if a public path is removed or changed while `Cargo.toml`'s version is
   unchanged from the last tag** — you cannot break the API without bumping the
   version. Additions pass; an intentional major passes once the version moves.

   `cargo public-api` needs nightly for rustdoc JSON, which we now have.
   **If that gate is impractical, implement the reporting form and say what
   made the gate impractical** — do not silently downgrade.
5. **Install check**: `cargo install --path . --features cli --locked` into a
   scratch root, then `--format json` on a date fixture asserting `iso` is a
   string. Cheap, and it is the check that caught `cargo install` producing no
   binary.
6. **F-2 and F-3.** Collapse the duplicate `cli` / `serde,chrono,cli` legs into
   one — **state which you kept and why** — and add a minimal build at MSRV
   (`cargo +1.88.0 build --no-default-features`), which nothing currently does.
7. **The portable memory relationships become a test.** At minimum M7 unit 04's:
   with a non-`Positional` alignment mode, peak equals `Positional` peak. Use the
   tracking allocator the existing memory tests use. **Assert the relationship,
   never an absolute byte count** — that is what makes it survive a CI runner.
   Then annotate `performance.md` to say which of its figures are now guarded by
   a test and which remain point-in-time measurements from one machine.

## Required tests

- The new memory-relationship test, shown failing by reintroducing the clone it
  guards against (or by a targeted change that breaks the equality).
- **Every new CI job shown failing** on a deliberately broken input — a doc
  warning, a mis-pinned `compile_fail` code, a removed public path without a
  version bump, a packaged `.git-exclude/` file. **A CI job nobody has seen fail
  is a job nobody knows works**; this project has shipped three guards that did
  not guard.

## Acceptance criteria

1. `cargo doc -D warnings`, nightly doctests, publish dry-run (on tags), install
   check, and the public-api job all present and **each demonstrated failing**.
2. The public-api job either gates as described or reports, **with the reason
   stated**.
3. F-2 collapsed, with the kept leg named; F-3's minimal MSRV build added.
4. The memory relationship is a test, asserting a relationship not a number,
   shown failing.
5. `performance.md` distinguishes guarded figures from point-in-time ones.
6. **Total CI wall time reported, before and after.** Four minutes is the
   budget; if the change approaches it, say so.
7. No existing job weakened; no crate code changed beyond the new test.
8. Gates green locally, including `cargo check --manifest-path fuzz/Cargo.toml --bins`.

## Prohibited shortcuts

- **Do not add `continue-on-error` to make a job pass.** That is the
  `compile_fail`-on-stable defect in a new place: a check that cannot fail.
- Do not assert an absolute byte count in the memory test.
- Do not delete the stable doctest run when adding the nightly one — they check
  different things.
- Do not pin nightly to a date to avoid flakiness without saying so; `fuzz-smoke`
  uses floating `@nightly` and we accept that already.

## Known risks

- **Nightly can break.** It already can, via `fuzz-smoke`. If you think a
  floating nightly is too fragile for a *gating* doctest job, that is a real
  argument — make it in the review request rather than pinning silently.
- **`cargo public-api`'s output format is not stable.** A job parsing it may
  need pinning. Report the version you used.
- **`cargo install` in CI is slow** — it builds from scratch. If it pushes the
  run past four minutes, say so; it may belong on tags only, like the dry run.

## Required evidence

- Each new job's first **failing** run, and its passing run
- CI wall time before and after
- The memory test failing by a targeted change
- The public-api decision, with its reasoning
- Which feature leg you kept for F-2, and why

## Review request format

Per development policy §9.2. Additionally: state the CI wall time before and
after, and say plainly which of the new jobs you could **not** make fail on
purpose — that list is more interesting than the rest of the report.
