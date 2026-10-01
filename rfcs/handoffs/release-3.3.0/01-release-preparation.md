# Handoff — 3.3.0 release preparation

**Written:** 2026-10-01. **Authorized:** owner, 2026-10-01.
**Sequence.** Last. M9 is closed (all seven units) and f135 is merged; `main` is at `d2b134b`,
CI green per job. **Nothing else lands before the cut.**

## Purpose

Assemble 3.3.0 from what is already in `[Unreleased]`, verify it end to end, and stop. **You do not
cut it.** The tag, the push and `cargo publish` are the owner's and mine.

## What 3.3.0 is

**A correctness-and-honesty release.** One defect a caller will notice, one API doc that promised
something false, and a milestone's worth of making the records and the checks say what is true.

| | What |
|---|---|
| **f135** | A plain numeric workbook with **no formulas** emitted one `Info` diagnostic per numeric cell, on by default, bounded by nothing — 400,000 of them and a 158.9 MiB serialised result from a 666 KiB input. Now one per sheet per side, carrying a count |
| **doc** | `include_formula_cached_values` never controlled whether cached formula values are compared, and its doc said it did |
| **doc** | `min_severity` is a `retain` after assembly, so it is not a resource control — now stated |
| **doc** | `Limits::hardened()` claimed to "comfortably accommodate an ordinary office workbook"; `max_cells_read` counts **both sides**, so it refuses a 500,000-row seven-column pair |
| **M9** | The fuzz corpus reaches the sheet reader; the fixture corpus produces every diagnostic the engine emits (25 scenarios); `src/` carries no `#[allow]` **attribute** (one mention survives in a `lib.rs` comment — prose discussing the pattern, not a silencing); the layout rule is followed; RFC-009 §5, RFC-035's lifecycle and the handoff-naming rule corrected |

**One defect stays open and must be said plainly:** a crafted worksheet panics in builds with debug
assertions on (`calamine`, [#694](https://github.com/tafia/calamine/issues/694)). Release builds
return an ordinary `Err`. **Every downstream `cargo test` is a debug build.**

## Why minor, not patch

`cargo public-api` reports **nothing** against 3.2.0 — simplified and unsimplified, all three
sections `(none)`. A patch is therefore arguable and it is wrong twice over:

1. **`formula_unavailable` changes shape.** A caller matching on it got one diagnostic per numeric
   cell with an `address`; they now get one per sheet with `address: None` and a count in the
   message. Anyone counting or locating those entries sees different data.
2. **A caller who set `include_formula_cached_values = false`** on our own recommendation — given
   while the flood was unfixed — needs to read that it cost them nothing. That is a correction to
   advice we published, not a silent doc tidy.

**Say both in the summary.** The first is the behaviour change; the second is why the release notes
matter to someone who already acted on the old documentation.

## Change scope

- `CHANGELOG.md` — stamp `[Unreleased]` as `[3.3.0]`, add the summary paragraph.
- `Cargo.toml` — `3.2.0` → `3.3.0`; both lockfiles follow.
- Nothing else.

## Non-change scope

- **Do not commit, tag, push or publish.**
- Nothing under `src/`, `tests/`, `docs/`, `rfcs/`, `fuzz/`, `.github/`. **If the sweep finds
  something, report it** — do not fix it in the release commit.
- **Do not add or reword an existing CHANGELOG entry.** Each has been through a review. Assembling
  them, and writing the summary, is the task.
- **MSRV stays 1.88.0.** Confirm rather than assume.
- **Three things still wait on `calamine` #694 and none is yours to tidy at the cut:**
  `fuzz/corpus-quarantine/` stays (one seed); `fuzz_self_comparison` stays out of the `fuzz-smoke`
  matrix; `fuzz_open_xlsx_bytes` stays at **`-runs=0`**. They close together when #694 is fixed and
  the dependency bumped. **Do not read a green `fuzz-smoke` as evidence the reader was fuzzed this
  cycle — it was replayed.**
- **RFC-028's assurance row must still read `Partially`**, not Yes. One of its two inputs is fixed;
  #694 is not.
- **`benches/memory.rs:382`** is still knowingly left alone, as in the last three cuts.
- The ForskScope letter, the `calamine#714` patch (open as
  [#729](https://github.com/tafia/calamine/pull/729)) and the RustSec PR
  ([#3297](https://github.com/rustsec/advisory-db/pull/3297)) are not yours and not part of this
  unit.

## Required implementation

1. **Stamp `[Unreleased]` as `[3.3.0]`** with today's date, in the form `[3.2.0]` uses.
2. **Section order:** `Security`, `Changed`, `Documentation` is what is there and it matches the
   house order with Documentation last. Leave it.
3. **Write the release summary paragraph.** It must:
   - **lead with f135**, in terms a caller can match against their own data: *a workbook with no
     formulas produced one diagnostic per numeric cell*, with the 666 KiB → 158.9 MiB figure, and
     that it was **on by default**;
   - say the **shape change** plainly (per sheet, `address: None`, count in the message) so anyone
     matching on `FormulaUnavailable` knows before they upgrade;
   - say that **setting `include_formula_cached_values = false` never cost a comparison**, because
     we recommended it and the doc said otherwise;
   - say `Limits::hardened()`'s boundary in concrete terms — it is the correction most likely to
     change what a reader does;
   - state the open `calamine` defect and that debug builds are where it bites;
   - say why it is a **minor**, in the terms above;
   - **credit where the corrections came from.** ForskScope found the
     `include_formula_cached_values` claim by reading our code after we gave them bad advice, and
     the `hardened()` boundary came out of M9 unit 06's measurement. Say so.
4. **Bump `Cargo.toml` to 3.3.0**; update both lockfiles (`cargo build`, then the fuzz check
   rewrites `fuzz/Cargo.lock`). The benchmark's crates.io `sheets-diff 1.2.0` dev-dependency stays
   untouched.
5. **Run the pre-release sweep**, each command with its exit status and a line of real output,
   **one scratch target dir for the whole sweep**, deleted when the evidence is captured:
   - fmt; clippy `--all-targets` under `--all-features` and `--no-default-features`; the scoped
     stdout gate; `cargo deny check`;
   - MSRV 1.88: `--all-features`, `--no-default-features`,
     `--no-default-features --features cli`; doctests stable, 1.88 **and nightly**;
   - **`cargo check --manifest-path fuzz/Cargo.toml --bins`** (rule 003);
   - `cargo doc -D warnings`; the four-leg feature matrix;
   - corpus byte-identical to `HEAD`;
   - `cargo publish --dry-run` at 3.3.0, packaged list checked for `.git-exclude/` entries **and for
     the 24 new fixture files** — six new scenarios × four files (`old.xlsx`, `new.xlsx`,
     `expected.json`, `scenario.toml`), verified by `git diff --name-status 3.2.0..HEAD`, plus
     three modified goldens; confirm they are in and nothing under `fuzz/corpus*/` is;
   - the documented install command at 3.3.0, with `--format json` populating `iso`;
   - **a last read of `docs/` and the CHANGELOG against each other.** Seven sweeps have found
     something every time. The likely places this cycle: any remaining claim that
     `include_formula_cached_values` controls a comparison; any statement that `hardened()`
     accommodates ordinary workbooks; the `formula_unavailable` shape described anywhere as
     per-cell; and whether `src/` really carries no `#[allow]` (M9 unit 02's claim). **Use
     `grep -rn "^\\s*#\\[allow("`, not `grep -rn "#\\[allow("`** — the latter hits a comment in
     `src/lib.rs:4` that merely discusses the pattern. This handoff's first draft got that wrong.

## A check CI gives you, and what it does not

The `public-api` job fails when a public path is **removed or changed** without a greater major.
**This release adds nothing and changes nothing**, so it should pass trivially. **Confirm locally
before the cut**, because the job runs on the tag:

```
cargo public-api --all-features diff 3.2.0..HEAD
```

Report what it prints. Anything at all is the thing to catch.

## Required tests

No new tests. The suite is the test: **407 all-features / 373 default, 29 suites.** Report the
numbers you get; a difference is a finding, not a rounding.

## Acceptance criteria

1. `[3.3.0]` stamped with today's date; `[Unreleased]` left empty and in place.
2. The summary covers every point in *Required implementation* 3.
3. `Cargo.toml` at 3.3.0; both lockfiles updated; no other manifest change.
4. Sweep: every command exit 0, evidence captured, one scratch dir, deleted.
5. `cargo publish --dry-run` clean, package contents checked as above.
6. `cargo public-api --all-features diff 3.2.0..HEAD` reported.
7. Matrix matches 407 / 373, or the difference is explained.
8. Corpus byte-identical.
9. `git diff --stat` touches exactly `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`, `fuzz/Cargo.lock`.

## Prohibited shortcuts

- **Do not cut, tag, push or publish.**
- Do not fix a sweep finding in the release commit. A release commit that also fixes something
  cannot be bisected.
- Do not soften the open-defect sentence, and do not let the quarantine/`-runs=0` state go
  unmentioned if a reader could mistake `fuzz-smoke` green for a fuzzing campaign.
- Do not claim a defect is fixed in a version it is not.

## Known risks

**1. The summary is the part that has been wrong before.** Every entry beneath it has been
reviewed; the paragraph on top gets written once, at the end. Two of 3.2.0's findings were exactly
that — a correct fact in a report, summarised into something weaker. **Write it first, not last, and
check each claim against the entry it summarises.**

**2. This release's subject is "we corrected what we claimed."** A summary that overclaims would be
self-refuting. Prefer the concrete number to the adjective everywhere.

**3. Nine new fixture files in the package.** Small, and they are the regression evidence for seven
diagnostics, so they belong — but confirm the package size has not jumped unexpectedly and that
`fuzz/corpus*/` is still excluded.

## Required evidence

Under `.git-exclude/review-request/release-3.3.0-01-release-preparation/evidence/`:

1. The full sweep log, one command per line with exit status.
2. `cargo publish --dry-run` output and the packaged file list.
3. `cargo public-api --all-features diff 3.2.0..HEAD`.
4. The matrix counts.
5. The corpus check.
6. The `docs/` ↔ CHANGELOG read: what you compared and what you found, including "nothing" — but
   see Known risk 1.
7. `git diff --stat`.

## Review request format

`.git-exclude/review-request/release-3.3.0-01-release-preparation/README.md`, with:

- The summary paragraph **quoted in full**, so I review the words rather than a description.
- Anything the sweep found, **unfixed**, with your view on whether it blocks the cut.
- The `public-api` output.
- Confirmation that the quarantine, the `fuzz-smoke` matrix and `-runs=0` are untouched, and that
  RFC-028's assurance row still reads `Partially`.
- Your view on whether anything in `[Unreleased]` reads wrongly now that it is a release rather than
  a running log. You wrote most of it.
