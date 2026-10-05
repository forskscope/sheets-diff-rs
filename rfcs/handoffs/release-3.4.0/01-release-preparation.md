# Handoff 01 — 3.4.0 release preparation

**Unit:** release-3.4.0 01. **Added 2026-10-06.** **Scoped by:** the architect.
**Release number decided by the owner 2026-10-06.** **The cut itself is the owner's and mine — this
unit prepares it and stops.**

## Purpose

Assemble 3.4.0 from the two reviewed units already in the working tree, and run the pre-release
sweep. No new behaviour, no new docs beyond the summary paragraph.

## What 3.4.0 is

Two units, both reviewed and approved:

- **`the-row-that-vanishes/01`** — under `AlignmentMode::RowSignature { sample_columns: Some(cols) }`,
  a row with no cell in any sampled column had no entry in the signature map, so a real change in
  that row produced **zero cell diffs, zero diagnostics, and an `alignment_summary` claiming
  `Exact`.** f130's defect on the one alignment path f130 did not touch. Rescued as `RowKey`'s
  keyless rows already were, with a new `missing_row_signature` warning and the same confidence
  clamp.
- **`row-space-and-the-anchor/00`** — four published doc claims that were false. The consequential
  one told consumers to *"collapse to one row per address"*, which merges two distinct changes and
  loses one.

## Why minor, not patch

**Because it adds public API:** `DiagnosticKind::MissingRowSignature { old_count, new_count }`, plus
its `code()` arm. New public surface is a minor under RFC-031 §6, even on a `#[non_exhaustive]` enum
where it is non-breaking.

This was labelled a patch (3.3.1) when the units were scoped; the label was wrong and contradicted
the vanishing-row handoff's own acceptance criterion 7. Corrected 2026-10-06. There is no delivery
cost either way — a consumer on `^3.3.0` resolves to 3.4.0 exactly as readily as to 3.3.1.

## The tree you are starting from

**Both units' work is already in the working tree, uncommitted. Leave it that way.** Your four
release files join it uncommitted, and the owner and I commit, tag and publish the whole set.

Already committed and pushed, so **not** part of your diff: `deny.toml`'s advisory exemption
(`cdcefe6`), the handoff and review records, and the release-label corrections.

## Change scope

- `CHANGELOG.md` — stamp `[Unreleased]` as `[3.4.0]`, add the summary paragraph.
- `Cargo.toml` — `3.3.0` → `3.4.0`; both lockfiles follow.
- Nothing else.

## Non-change scope

- **Do not commit, tag, push or publish.**
- Nothing under `src/`, `tests/`, `docs/`, `rfcs/`, `fuzz/`, `.github/`. **If the sweep finds
  something, report it** — do not fix it in the release commit.
- **Do not add or reword an existing CHANGELOG entry.** Both have been through a review, including
  the rewrap. Assembling them and writing the summary is the task.
- **MSRV stays 1.88.0.** Confirm rather than assume.
- **`deny.toml` is not yours.** The `RUSTSEC-2026-0317` exemption is committed with its reachability
  assessment. If `cargo deny` fails, that is a finding — do not touch the file.
- **Three things still wait on `calamine` #694, confirmed still open 2026-10-06, and none is yours:**
  `fuzz/corpus-quarantine/` stays (one seed); `fuzz_self_comparison` stays out of the `fuzz-smoke`
  matrix; `fuzz_open_xlsx_bytes` stays at **`-runs=0`**. **Do not read a green `fuzz-smoke` as
  evidence the reader was fuzzed this cycle — it was replayed.**
- **RFC-028's assurance row must still read `Partially`.** #694 is still open.
- **`benches/memory.rs:382`** is still knowingly left alone, as in the last four cuts.
- `calamine#729` (open) and the ForskScope correspondence are not yours.

## Required implementation

1. **Stamp `[Unreleased]` as `[3.4.0]`** with today's date, in the form `[3.3.0]` uses.

2. **Section order.** `Security`, then `Documentation`, is what is there and matches the house order
   with Documentation last. Leave it.

3. **Write the release summary paragraph.** It must:
   - **lead with the dropped change**, in terms a caller can match against their own configuration:
     `RowSignature` with `sample_columns` **set** could report **no difference where a cell had
     changed**, and `sample_columns: None` was never affected. Say plainly that anyone who used the
     affected shape should **re-check results they trusted**;
   - say that the alignment summary **positively asserted `Exact`** while this happened, because a
     consumer who checked confidence was told the pairing was perfect;
   - say it is **f130's defect on the path f130 did not touch**, and that both rescues now share one
     helper. A reader who remembers the 2.x fix deserves to know why it recurred;
   - say the **documentation corrections changed no behaviour**, and say what the dangerous one was:
     our own `CellDiff` doc told consumers to collapse by address, which merges two distinct changes
     under `RowKey` or `RowSignature`. Also that `ChangeAnchor` is **not** an identifier and that
     `next_after` / `previous_before` can fail to advance — **with no promise of a replacement**,
     which is unit 02's and not in this release;
   - say why it is a **minor**, in the terms above;
   - **credit where this came from.** ForskScope asked whether `RowSignature` could produce the
     keyless-row condition — *"you are better placed to say"* — and reading the code to answer them
     is what found the dropped change. The forty-row regression test is their shape. They also
     checked and reported that the `view.rs` navigation defect reaches none of their code, which is
     why its documentation was pulled forward and its fix was not. Say so, as the 3.3.0 notes did.

4. **Bump `Cargo.toml` to 3.4.0**; update both lockfiles (`cargo build`, then the fuzz check rewrites
   `fuzz/Cargo.lock`). The benchmark's crates.io `sheets-diff 1.2.0` dev-dependency stays untouched —
   it is the subject of the `deny.toml` exemption and must not move.

5. **Run the pre-release sweep** — the same 17-command sweep you ran for both units, each command
   with its exit status and a line of real output, **one scratch target dir for the whole sweep**,
   deleted when the evidence is captured — plus these release-specific checks:

   - **`cargo public-api --all-features diff 3.3.0..HEAD`.** Expect **exactly one addition**,
     `DiagnosticKind::MissingRowSignature`, and **nothing** removed or changed. Anything else is the
     finding. The commit-range form does an in-place `git checkout` and **fails on a dirty tree**,
     and your tree is dirty by design — use a detached worktree, or diff two rustdoc-JSON files, as
     you did for both units.
   - **`cargo publish --dry-run` at 3.4.0.** Check the packaged list for `.git-exclude/` entries
     (there must be none) and for the **four new fixture files** —
     `tests/fixtures/generated/missing_row_signature/{old.xlsx,new.xlsx,expected.json,scenario.toml}`.
     Confirm nothing under `fuzz/corpus*/` is in.
   - **Exactly one other file under `tests/fixtures/` changed: `corpus/README.md`.** That is
     **documentation, not a golden** — no existing golden moved this cycle, and the same file caused
     a miscount in the 3.3.0 handoff. Verify rather than assume.
   - **The documented install command at 3.4.0**, with `--format json` populating `iso`.
   - **A last read of `docs/` and the CHANGELOG against each other.** Eight sweeps have found
     something every time. The likely places this cycle: any remaining claim that one address carries
     one `CellDiff` without naming the alignment mode; any description of `ChangeAnchor` as an
     identifier or as stable; any statement that `alignment_summary` is reserved; and whether
     `missing_row_signature`'s behaviour is described consistently in `src/options.rs`,
     `src/model.rs` and `tests/fixtures/corpus/README.md`. **Use `grep -rn "^\s*#\[allow("`, not
     `grep -rn "#\[allow("`** — the latter hits a comment in `src/lib.rs:4`.

## Required tests

No new tests. The suite is the test: **420 all-features across 30 suites, 386 default across 29** —
measured 2026-10-06 on this tree, up from 3.3.0's 407/373 and 29 suites, the new suite being
`rowsignature_unmapped_rows`. Report the numbers you get; a difference is a finding, not a rounding.

## Acceptance criteria

1. `[3.4.0]` stamped with today's date; **zero non-blank characters** between `[Unreleased]` and it.
2. The summary paragraph covering all seven points of *Required implementation* 3.
3. `Cargo.toml` at 3.4.0; exactly two `version = "3.4.0"` lines across the two lockfiles; the
   `sheets-diff 1.2.0` dev-dependency untouched.
4. MSRV confirmed 1.88.0.
5. `public-api` range diff showing one addition and nothing else.
6. `publish --dry-run` list checked, with the four fixture files in and `.git-exclude/` and
   `fuzz/corpus*/` out.
7. Test totals reported.
8. Full sweep green, rule 003 included, one scratch dir, deleted.
9. **Nothing committed** — the four release files uncommitted alongside both units' work.

## Prohibited shortcuts

- Do not commit, tag, push or publish. The cut is the owner's and mine.
- Do not reword a reviewed CHANGELOG entry.
- Do not fix anything the sweep finds. Report it.
- Do not touch `deny.toml`.
- Do not use `cargo public-api`'s commit-range form against the dirty tree.
- Do not describe a replacement for `next_after` / `previous_before`. There is not one yet.

## Known risks

**1. Two units' work is uncommitted in one tree.** Both were reviewed in isolation; this is the
first time they are assembled. The `public-api` range diff is what catches a bad interaction, and
`src/model.rs` is the one file both touched — unit 01 added the variant, unit 00 rewrote two doc
comments. Read that file's full diff once before you start.

**2. The CHANGELOG's two entries were written before the release number was decided.** They say
`[Unreleased]`, which is correct; neither names a version. **Check that neither says "patch"**, since
both units were scoped as one before the semver correction.

**3. `cargo deny` is green only because of a committed exemption.** If it fails, the exemption's
three stated conditions may have stopped holding — that is a finding and it is mine, not a thing to
work around.

**4. A green `fuzz-smoke` means the corpus was replayed**, not that anything was fuzzed. #694 is
still open.

## Required evidence

Under `.git-exclude/review-request/release-3.4.0-01-release-preparation/evidence/`:

1. The CHANGELOG stamp and summary, as a diff.
2. `Cargo.toml` and both lockfile diffs.
3. The `public-api` range diff, by whichever isolation method you used.
4. `publish --dry-run`'s packaged list, with the fixture files and the absences shown.
5. Test totals, both feature sets.
6. The full sweep, one line of real output per command; scratch dir created and deleted.
7. The `docs/`-versus-CHANGELOG read, and anything it found.

## Review request format

`.git-exclude/review-request/release-3.4.0-01-release-preparation/README.md`, with:

- The summary paragraph quoted in full, so I review the words.
- The test totals, and whether they match the numbers above.
- What the `public-api` range diff printed.
- Anything the `docs/` read found. **Every cut so far has found something; if this one finds
  nothing, say so explicitly** so I know it was looked for rather than skipped.
