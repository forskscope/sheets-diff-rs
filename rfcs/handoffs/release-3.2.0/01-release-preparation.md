# Handoff — 3.2.0 release preparation

**Written:** 2026-09-29. **Authorized:** owner, 2026-09-29.
**Sequence.** Last. f131, f132, f133, f134, the alignment follow-ups and M9 unit 01 are all merged;
`main` is at `d89643a`, CI green per job. **Nothing else lands before the cut.**

## Purpose

Assemble 3.2.0 from what is already in `[Unreleased]`, verify it end to end, and stop. **You do not
cut it.** The tag, the push and `cargo publish` are the owner's and mine, as every release has been.

## What 3.2.0 is

**A security and correctness release.** Three defects that produced a crash or a wrong answer from a
file the caller did not write, one performance defect with no ceiling, and alignment becoming
cancellable. **Two of the three correctness defects were present in shipped versions**, which is the
fact the summary must lead with.

| | What | Shipped in |
|---|---|---|
| **f132** | A 512-byte file caused a 9.26 GB allocation and **aborted the process** | every released version |
| **f134** | Under `ExactNameThenIndex`, a sheet **vanished** from `WorkbookDiff::sheets` — one old `X` against two new `X` | 3.1.0 and earlier |
| **f133** | Two old sheets sharing a name made a third sheet's content appear as spurious changes | unreleased work only |
| **f131** | The compared-cell set was O(rows × cells); 37 s where `Positional` took 0.13 s, bounded by nothing | 2.1.0 – 3.1.0 |
| — | Row alignment is now cancellable | new |

**One defect stays open and must be said plainly:** a crafted worksheet panics in builds with debug
assertions on (`calamine`, tracked as [#694](https://github.com/tafia/calamine/issues/694)). Release
builds wrap and return an ordinary `Err` on the artifact we have. **Every downstream `cargo test` is
a debug build**, so this is live for anyone running our crate in their test suite.

## Why minor, not patch

`cargo public-api --simplified diff 3.1.0` reports **no difference** — nothing added, nothing
removed, nothing changed. A patch is therefore arguable and it is wrong. `[Unreleased]`'s `###
Changed` entry is a real behaviour change: **a comparison cancelled during alignment now returns
`Err(SheetsDiffError::Cancelled)` where it used to run to completion and return `Ok`.** A caller who
set a token and relied on getting a result cannot be handed that in a patch. **Say this in the
summary**, so the bump does not read as scope creep or as a security-theatre version jump.

## Change scope

- `CHANGELOG.md` — stamp `[Unreleased]` as `[3.2.0]`, add the summary paragraph.
- `Cargo.toml` — `3.1.0` → `3.2.0`; both lockfiles follow.
- Nothing else.

## Non-change scope

- **Do not commit, tag, push or publish.**
- Nothing under `src/`, `tests/`, `docs/`, `rfcs/`, `fuzz/src/`, `.github/`. **If the sweep finds
  something, report it** — do not fix it in the release commit.
- **Do not add or reword an existing CHANGELOG entry.** Assembling what is there, and writing the
  summary, is the task. The entries have each been through a review.
- **MSRV stays 1.88.0.** Confirm rather than assume.
- **`fuzz/corpus-quarantine/` stays, and `fuzz_self_comparison` stays out of the `fuzz-smoke`
  matrix.** Both wait on the still-open `calamine` overflow, not on anything in this release.
  Do not tidy them as part of the cut.
- **`benches/memory.rs:382`** is still knowingly left alone, as in 3.1.0's cut. Scheduled separately.
- **The GHSA, the ForskScope letter and the `calamine#714` patch are not yours and not part of this
  unit.** They follow the cut and are the owner's (rule 004).

## Required implementation

1. **Stamp `[Unreleased]` as `[3.2.0]`** with today's date, in the form `[3.1.0]` uses.
2. **Section order is already correct** — `Security`, `Changed`, `Fixed`, `Documentation`. Leave it.
3. **Write the release summary paragraph.** It must:
   - **lead with the security fix and say a caller's own mitigation did not help**: `max_input_bytes`
     could not see it (512 bytes) and **`Limits::hardened()` did not prevent it**. A reader who
     hardened their limits and believed themselves covered needs that sentence;
   - say **which defects shipped**, per the table above — particularly that a sheet could vanish in
     3.1.0 under `ExactNameThenIndex`. Do not let the reader infer that everything here arrived with
     unreleased work; an earlier draft of one entry said exactly that and was wrong;
   - state the open `calamine` defect and that debug builds are where it bites;
   - say why it is a **minor**, in the terms above;
   - name the observable changes a caller will notice: a cancelled comparison now returns
     `Err(Cancelled)`; aligned comparisons of asymmetric workbooks are dramatically faster
     (37 s → 154 ms on one row against 40,000); sheet matching reports a `Removed` where it
     previously reported a bogus `Moved`, or nothing at all;
   - **credit where the findings came from.** f131 and the alignment work came out of ForskScope's
     reports; f132, f133 and f134 came from our own fuzz corpus reaching the sheet reader for the
     first time, and f134 was found by the dev team while testing the neighbourhood of its own fix.
     Say so; it is true and it is how we would like the next ones to arrive.
4. **Bump `Cargo.toml` to 3.2.0**; update both lockfiles (`cargo build`, then the fuzz check rewrites
   `fuzz/Cargo.lock`). The benchmark's crates.io `sheets-diff 1.2.0` dev-dependency stays untouched.
5. **Run the pre-release sweep**, each command with its exit status and a line of real output,
   **one scratch target dir for the whole sweep** (rule 002 as narrowed), deleted when the evidence
   is captured:
   - fmt; clippy `--all-targets` under `--all-features` and `--no-default-features`; the scoped
     stdout gate; `cargo deny check`;
   - MSRV 1.88: `--all-features`, `--no-default-features`,
     `--no-default-features --features cli`; doctests stable, 1.88 **and nightly**;
   - **`cargo check --manifest-path fuzz/Cargo.toml --bins`** (rule 003);
   - `cargo doc -D warnings`; the four-leg feature matrix;
   - corpus byte-identical to `HEAD`;
   - `cargo publish --dry-run` at 3.2.0, packaged list checked for `.git-exclude/` entries **and for
     the new binary fixtures** — `tests/fixtures/f132/*.bin` and `tests/fixtures/f134/*.xlsx` are new
     this release; confirm they are included and that nothing else unexpected is;
   - the documented install command at 3.2.0, with `--format json` populating `iso`;
   - **a last read of `docs/` and the CHANGELOG against each other.** Six sweeps have now found
     something every time. The likely places here: any sentence claiming `Limits::hardened()` bounds
     what a caller feeds in; any statement that sheet matching never loses a sheet; the
     `max_alignment_product` figures, which moved this cycle; and RFC-028 §7's assurance row, which
     must still read **Partially**, not Yes, because the overflow defect is open.

## A check CI gives you, and what it does not

The `public-api` job fails when a public path is **removed or changed** without a greater major.
**This release adds nothing and changes nothing**, so it should pass trivially. **Confirm locally
before the cut**, because the job runs on the tag, which is after the point of no return:

```
cargo public-api --all-features diff 3.1.0..HEAD
```

Report what it prints. Anything at all is the thing to catch — `--simplified` has shown nothing all
cycle, and the unsimplified form is stricter.

## Required tests

No new tests. The suite as it stands is the test: **399 all-features / 365 default, 29 suites.**
Report the numbers you get; a difference from those is a finding, not a rounding.

## Acceptance criteria

1. `[3.2.0]` stamped with today's date; `[Unreleased]` left empty and in place.
2. The summary covers every point in *Required implementation* 3.
3. `Cargo.toml` at 3.2.0; both lockfiles updated; no other manifest change.
4. Sweep: every command exit 0, evidence captured, one scratch dir, deleted.
5. `cargo publish --dry-run` clean, package contents checked as above.
6. `cargo public-api --all-features diff 3.1.0..HEAD` reported.
7. Matrix matches 399 / 365, or the difference is explained.
8. Corpus byte-identical.
9. `git diff --stat` touches exactly `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`, `fuzz/Cargo.lock`.

## Prohibited shortcuts

- **Do not cut, tag, push or publish.** Prepared, verified, and handed back.
- Do not let a sweep finding be fixed quietly in the release commit. A release commit that also
  fixes something is a release nobody can bisect.
- Do not soften the open-defect sentence. A release that fixes three security-class defects and
  hides a fourth is worse than one that names it.
- Do not claim a defect is fixed in a version it is not. Check each "shipped in" against the table.

## Known risks

**1. The summary is the part that has been wrong before.** Every entry beneath it has been reviewed;
the paragraph on top will be written once, quickly, at the end. Two of this cycle's findings were
exactly that — a correct fact in a report, summarised into something weaker. **Write it first, not
last, and check each claim against the entry it summarises.**

**2. `cargo publish --dry-run` packages binary fixtures.** Three new binary files land in
`tests/fixtures/` this cycle. They are small and they are the regression evidence for two security
fixes, so they belong in the package — but confirm the package size has not jumped and that nothing
under `fuzz/corpus*/` is being swept in.

**3. The version is a judgement, and it is mine.** If the sweep turns up a public-API difference
that `--simplified` hid, **stop** — that changes the version question and it comes back to me before
anything is stamped.

## Required evidence

Under `.git-exclude/review-request/release-3.2.0-01-release-preparation/evidence/`:

1. The full sweep log, one command per line with exit status.
2. `cargo publish --dry-run` output and the packaged file list.
3. `cargo public-api --all-features diff 3.1.0..HEAD`.
4. The matrix counts.
5. The corpus check.
6. The `docs/` ↔ CHANGELOG read: what you compared and what you found, including "nothing" if that
   is the answer — but see Known risk 1.
7. `git diff --stat`.

## Review request format

`.git-exclude/review-request/release-3.2.0-01-release-preparation/README.md`, with:

- The summary paragraph quoted in full, so I review the words rather than a description of them.
- Anything the sweep found, **unfixed**, with your view on whether it blocks the cut.
- The `public-api` output.
- Confirmation that the quarantine and the `fuzz-smoke` matrix are untouched.
- Your view on whether anything in `[Unreleased]` reads wrongly now that it is a release rather than
  a running log. You have written most of it; you are the right person to notice.
