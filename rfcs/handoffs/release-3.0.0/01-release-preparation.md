# Handoff — 3.0.0 release preparation

**Governing.** ROADMAP §6 (release cycles); [RFC-037](../../accepted/037-v3-scope.md);
Keep a Changelog
**Roadmap.** M10 closes with this release
**Sequence.** After M10 units 01–10 (all merged). Before the cut.

## Purpose

Prepare 3.0.0 for cutting. **Preparation only — the cut is the owner's and the
architect's**, as with 2.5.1 and 2.6.0.

## What 3.0.0 is

**A correction release. Nothing was added except two builder setters that close
gaps, and everything else removed or renamed described behaviour the engine does
not have.** RFC-037 §4 forbade features and none crept in.

Ten units. The summary must make the *character* of the release clear before it
lists anything: a caller upgrading is not getting new capability, they are
getting an API that stops claiming things that were never true.

**The two changes that require action from every caller who is affected:**

1. **Options structs are `#[non_exhaustive]`** — struct-literal construction no
   longer compiles, including `..Default::default()`. **This is the one we
   documented**: the API guide showed the pattern and a test pinned it as a
   compatibility promise.
2. **`to_json` / `to_json_pretty` return `String`** — delete the `?`.

**The rest are removals**, nine of them, each with a row in the new migration
guide.

## Change scope

`CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`.

## Non-change scope

- **Do not commit, tag, push or publish.** Leave everything uncommitted.
- **Nothing under `src/`, `tests/`, `docs/`, `rfcs/` or `fuzz/`.** If the sweep
  finds something wrong, **report it**.
- Do not add CHANGELOG entries. Assembling what is there is the task.
- **Do not hold the release for the nightly-doctest gap** (see below). It is a
  CI improvement on M9's list, not a defect in this release.
- **MSRV stays 1.88.0.** Nothing in M10 required a newer compiler; confirm
  rather than assume.

## Required implementation

1. **Stamp `[Unreleased]` as `[3.0.0]`** with today's date. Check `[2.6.0]`'s
   heading for the exact separator.
2. **Order the sections per Keep a Changelog**, after checking what order the
   file's existing entries use. `[Unreleased]` currently has four sections and
   **`### Removed` is the substantial one** — nine bullets. That is unusual for
   this file and it is the point of the release.
3. **Write the release summary paragraph.** Requirements, because this one is
   harder than 2.6.0's:
   - **Say what the release is** — a correction release, no new features — in
     the first sentence.
   - **Then the two actions**, in the order above.
   - **Then what it buys**: every option added after 3.0.0 is additive, which is
     the thing a caller gets in exchange for the one break.
   - **Do not lead with a list of removed names.** A reader wants to know
     whether they must act before they want the inventory.
   - **Do not overstate.** Every defect in this release was found by us, in
     readiness reviews and unit scoping; **no user reported any of them.** Say
     that once, plainly, or not at all.
4. **Bump the version** in `Cargo.toml` to `3.0.0` and update `Cargo.lock`
   (a `cargo build` does it; the benchmark's `sheets-diff 1.2.0` dev-dependency
   from crates.io is untouched).
5. **Run the pre-release verification sweep** and report each result **with its
   exit status and a line of real output**:
   - fmt; clippy `--all-targets` under `--all-features` **and**
     `--no-default-features`, `-D warnings`; the scoped stdout gate;
     `cargo deny check`;
   - MSRV 1.88: `build --all-features`, `--no-default-features`,
     `--no-default-features --features cli`; doctests on stable and 1.88;
   - **`cargo check --manifest-path fuzz/Cargo.toml --bins`** — rule 003. This
     is the check whose absence turned `main` red during unit 06; it is not
     optional and it is not covered by anything else;
   - the five-feature test matrix;
   - `cargo doc -D warnings`;
   - fixture corpus byte-identical to `HEAD`;
   - **`cargo publish --dry-run` packaging cleanly at 3.0.0**, with the packaged
     file list checked: nothing under `.git-exclude/`, and the new migration
     guide **is** included;
   - **the documented install command at 3.0.0**, from this tree into a scratch
     `--root`, with `--format json` producing a populated `iso`;
   - **a last read of `docs/` and the CHANGELOG against each other.** Four
     sweeps have now found something every time. The highest-yield places this
     release: any sentence naming a removed value, the summary line's shape, and
     the migration guide's own claims.

## A note on the nightly-doctest gap

`compile_fail,EXXXX` does not enforce the error code on stable rustdoc, so the
crate's thirteen such blocks pass if the example fails to compile for any
reason. **All thirteen were verified correct on nightly at unit 10's review.**

**Ship.** The guards are right; only their enforcement is weak, and the fix is
one CI step on a toolchain `fuzz-smoke` already installs. It is on M9's list.

**If you want belt and braces for the cut**, run
`cargo +nightly test --doc --all-features` once as part of the sweep and report
it. That is a reasonable addition and I would accept it; it is not required.

## Required tests

None new. The sweep is the test.

## Acceptance criteria

1. `[3.0.0]` stamped, dated, sections in the file's own order.
2. The summary says what the release is before what it removes, names the two
   actions, says what the break buys, and does not overstate how the defects
   were found.
3. `Cargo.toml` at 3.0.0; `Cargo.lock` updated; benchmark dependency untouched;
   **MSRV confirmed still 1.88.0**.
4. Every sweep item reported **with exit status and real output**.
5. **The `fuzz/` check is in the sweep** and passed.
6. `cargo publish --dry-run` clean; packaged list checked, and the migration
   guide is in it.
7. The install command run at 3.0.0.
8. Corpus byte-identical.
9. Nothing outside the three files modified; nothing committed, tagged, pushed
   or published.

## Prohibited shortcuts

- **Do not `cargo publish`.** Not with `--dry-run` removed, not by accident.
- Do not rewrite a shipped CHANGELOG entry. `[2.6.0]` and older are
  annotate-only.
- Do not tidy a bullet from M10's units. They were reviewed as written.
- Do not fix anything the sweep finds. Report it.
- **Do not soften the struct-literal break** in the summary. We recommended that
  pattern in our own guide.

## Known risks

- **`[Unreleased]` is 260 lines across four sections**, written by ten units over
  one day, several of which corrected sentences written by earlier units in the
  same section. **Read it end to end for contradictions before stamping.** That
  is the most likely place for one, and the 2.6.0 sweep found two there.
- `cargo publish --dry-run` may warn about included files. Report, do not act.
- **This is the first major.** `cargo publish` for a `3.0.0` behaves no
  differently, but the version appears in the migration guide, the README and
  the RFC records — check they say 3.0.0 and not "the next major".

## Required evidence

- The stamped CHANGELOG section
- Each sweep command, its exit status, and real output
- The `publish --dry-run` output including the file list
- The install run at 3.0.0
- Corpus byte-comparison
- The docs-vs-CHANGELOG read, **including "nothing", with what you compared**

## Review request format

Per development policy §9.2. Additionally: state whether the MSRV is still
correct at 1.88.0 having checked rather than assumed, and report anything the
docs-vs-CHANGELOG read turned up — the last four sweeps each found something.
