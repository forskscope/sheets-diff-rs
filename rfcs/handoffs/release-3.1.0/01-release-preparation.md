# Handoff — 3.1.0 release preparation

**Governing.** ROADMAP §6; Keep a Changelog
**Sequence.** After f130 (merged, `b49dfd0`). Before the cut.

## Purpose

Prepare 3.1.0 for cutting. **Preparation only — the cut is the owner's and the
architect's.**

## What 3.1.0 is

**One defect fix, and the public diagnostic that had to come with it.**

`AlignmentMode::RowKey` never compared rows that had no cell in a key column.
They entered neither `matched`, `removed` nor `inserted`; their cells were never
looked at; and `alignment_summary` reported **`confidence: Exact` with no
diagnostic** — a positive claim of exact matching made about rows that were
skipped. Present in **twelve releases, 2.1.0 through 3.0.0**, established by
running the reproduction against each tag's own source.

Reported by ForskScope, whose shape is the ordinary one: a unique `ID` blank in
about 5% of rows — subtotals, spacers, notes.

**It is a minor and not a patch** because the fix needs a new
`DiagnosticKind::MissingAlignmentKey`, and `code()`'s own documentation promises
callers that new variants arrive in a minor release. Breaking that in a patch
would be the defect class 3.0.0 existed to remove.

## Change scope

`CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`, **`fuzz/Cargo.lock`**.

**Four files, not three.** `fuzz/Cargo.lock` is tracked and pins `sheets-diff`
by version, so the mandatory fuzz check rewrites it. 3.0.0's prep discovered
this the hard way; it is in scope from the start this time.

## Non-change scope

- **Do not commit, tag, push or publish.**
- Nothing under `src/`, `tests/`, `docs/`, `rfcs/`, `fuzz/src/` or
  `.github/`. If the sweep finds something, **report it**.
- Do not add or reword a CHANGELOG entry. Assembling what is there is the task.
- **MSRV stays 1.88.0.** Confirm rather than assume.
- **`benches/memory.rs:382` is knowingly left alone.** It keys `RowKey` on
  `columns: vec![0]`, which is 0-based where the option is 1-based — the finding
  behind M9 unit 00's correction. Post-f130 that column makes every row keyless,
  so the bench line's meaning has changed again. It is a maintainer tool, it
  ships no behaviour, `performance.md` already annotates what it measured, and
  it is scheduled separately. **Do not fix it here.**

## Required implementation

1. **Stamp `[Unreleased]` as `[3.1.0]`** with today's date, in the form
   `[3.0.0]` uses.
2. **Section order** per the file's own convention. `[Unreleased]` has `### Added`
   and `### Fixed` only.
3. **Write the release summary paragraph.** Shorter than 3.0.0's — this release
   is one fix. It must:
   - say the defect in a sentence a user can match against their own data: *a
     row with a blank key was never compared*;
   - say **twelve releases, 2.1.0 through 3.0.0**;
   - say why it is a minor (the new diagnostic), so nobody reads the version bump
     as scope creep;
   - name the behaviour change a caller will actually notice: **`RowKey` now
     reports more** — a changed keyless row appears as a removal plus an
     insertion, and `confidence` is capped at `Medium` whenever any row was
     keyless, including when every keyless row paired;
   - **credit the report.** ForskScope found it by measuring each mode's failure
     modes rather than its success cases. Say so; it is true and it is how we
     would like the next one to arrive.
4. **Bump `Cargo.toml` to 3.1.0**; update both lockfiles (`cargo build`, then
   the fuzz check rewrites `fuzz/Cargo.lock`). The benchmark's crates.io
   `sheets-diff 1.2.0` dev-dependency stays untouched.
5. **Run the pre-release sweep**, each command with its exit status and a line of
   real output, **one scratch target dir for the whole sweep** (rule 002 as
   narrowed), deleted when the evidence is captured:
   - fmt; clippy `--all-targets` under `--all-features` and
     `--no-default-features`; the scoped stdout gate; `cargo deny check`;
   - MSRV 1.88: `--all-features`, `--no-default-features`,
     `--no-default-features --features cli`; doctests stable, 1.88 **and
     nightly**;
   - **`cargo check --manifest-path fuzz/Cargo.toml --bins`**;
   - `cargo doc -D warnings`; the four-leg feature matrix;
   - corpus byte-identical to `HEAD`;
   - `cargo publish --dry-run` at 3.1.0, packaged list checked for
     `.git-exclude/` entries;
   - the documented install command at 3.1.0, with `--format json` populating
     `iso`;
   - **a last read of `docs/` and the CHANGELOG against each other.** Five sweeps
     have now found something every time — including two false statements in
     3.0.0's own notes. The likely places here: any sentence about what `RowKey`
     does with a missing key, and the `confidence` values named in RFC-011 and
     the API guide.

## A check the new CI gives you, and what it does not

M9 unit 00 added a `public-api` job that fails when a public path is **removed
or changed** without a greater major. **This release only adds one** — the new
variant — so it should pass. **Confirm that locally before the cut**, because
the job runs on the tag, which is after the point of no return:

```
cargo public-api --all-features diff <last tag>..HEAD
```

Report what it prints. An unexpected *change* line is the thing to catch.

## Required tests

None new. The sweep is the test.

## Acceptance criteria

1. `[3.1.0]` stamped and dated; section order the file's own.
2. The summary names the defect, the twelve affected releases, why it is a
   minor, the behaviour a caller will notice, and the reporter.
3. `Cargo.toml` 3.1.0; **both** lockfiles updated; benchmark dependency
   untouched; MSRV confirmed 1.88.0.
4. Every sweep item with exit status and real output; **one** scratch dir,
   deleted after.
5. `cargo publish --dry-run` clean; packaged list checked.
6. Install at 3.1.0 verified.
7. The `public-api` diff reported, showing additions only.
8. Corpus byte-identical.
9. Only the four scoped files modified; nothing committed, tagged, pushed or
   published.

## Prohibited shortcuts

- **Do not `cargo publish`.**
- Do not rewrite a shipped entry; `[3.0.0]` and older are annotate-only.
- Do not tidy f130's bullets — they were reviewed as written.
- Do not fix anything the sweep finds. Report it.
- Do not create a scratch dir per command. One per sweep, deleted after.

## Known risks

- **`[Unreleased]` is small this time** (46 lines, two sections), so the
  contradiction risk is lower than 3.0.0's — but the last five sweeps each found
  something, so read it against the docs rather than assuming a short section is
  a safe one.
- `cargo publish --dry-run` may warn about included files. Report, do not act.
- The `public-api` tool floats with nightly. If it will not run locally, say so
  and note that the tag's CI run will be the first real execution.

## Required evidence

- The stamped CHANGELOG section
- Each sweep command, exit status, real output
- The `publish --dry-run` output and file list
- The install run at 3.1.0
- The `cargo public-api` diff
- Corpus byte-comparison
- The docs-vs-CHANGELOG read, **including "nothing", with what you compared**

## Review request format

Per development policy §9.2. Additionally: confirm MSRV 1.88.0 by running, and
report the `public-api` diff verbatim.
