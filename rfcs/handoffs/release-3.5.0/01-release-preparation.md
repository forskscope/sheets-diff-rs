# Handoff 01 — 3.5.0 release preparation

**Unit:** release-3.5.0 01. **Added 2026-10-06.** **Scoped by:** the architect.
**The cut is the owner's and mine — this unit prepares it and stops.**

## Purpose

Assemble 3.5.0 from the five reviewed units in the working tree, and run the pre-release sweep. No
new behaviour, no new docs beyond the summary paragraph.

## What 3.5.0 is

Five units, all reviewed and approved:

| Unit | What |
|---|---|
| `row-space-and-the-anchor/01` | `CellDiff::row_placement` — the row numbers a change is numbered in, on each side. `Hash` on `CellAddress` so the documented identity is keyable |
| `row-space-and-the-anchor/02` | `ChangeKey` and a navigation route that cannot hang; `next_after` / `previous_before` deprecated with their behaviour untouched |
| `confidence-that-measures-counts/01` | `confidence` stops claiming `Exact` for an ambiguous pairing; `ConfidenceReason`, `AlignmentSummary::reasons` and `is_ambiguous()` say why |
| `confidence-that-measures-counts/02` | `duplicate_row_signature` — the signature path's remaining silence |
| `every-row-in-exactly-one-bucket/01` | the debug-build invariant assertion. **No public API, no release behaviour** |

**Two behaviour changes reach consumers**, and they are the reason the summary leads where it does:

1. A sheet with a repeated key (`RowKey`) or repeated signature (`RowSignature`) that reported
   `Exact` now reports `Medium`. A consumer gating on `Exact` sees different behaviour.
2. `RowSignature` emits a new `duplicate_row_signature` warning. A consumer counting diagnostics, or
   treating any warning as a failure, sees new output.

## Why minor

Additive public API throughout — `RowPlacement`, `CellDiff::row_placement`, `Hash` on `CellAddress`,
`ChangeKey` and the navigation methods, `ConfidenceReason`, `AlignmentSummary::reasons` and
`is_ambiguous()`, `DiagnosticKind::DuplicateRowSignature` — and nothing removed or re-signed. The two
behaviour changes are defect fixes that make a reported value more conservative and a silent path
speak. Two deprecations, whose behaviour is unchanged; nothing is removed, per RFC-031 §7.

## The tree you are starting from

**All five units' work is in the working tree, uncommitted. Leave it that way.** Your four release
files join it uncommitted, and the owner and I commit, tag and publish the whole set.

Already committed and pushed, so **not** part of your diff: RFC-031 §8's amendment, the quarantine
README's note, the version-gate handoff, and all the review records.

## Change scope

- `CHANGELOG.md` — stamp `[Unreleased]` as `[3.5.0]`, add the summary paragraph.
- `Cargo.toml` — `3.4.0` → `3.5.0`; both lockfiles follow.
- Nothing else.

## Non-change scope

- **Do not commit, tag, push or publish.**
- Nothing under `src/`, `tests/`, `docs/`, `rfcs/`, `fuzz/`, `.github/`. **If the sweep finds
  something, report it** — do not fix it in the release commit.
- **Do not add or reword an existing CHANGELOG entry.** All of them have been through a review.
  Assembling them and writing the summary is the task.
- **MSRV stays 1.88.0.** Confirm rather than assume. Note that two units need it exactly: the
  invariant's call site uses a let-chain and the navigation tests use `#[expect]`.
- **`deny.toml` is not yours.** The `RUSTSEC-2026-0317` exemption is committed with its reachability
  assessment and its three stated conditions. If `cargo deny` fails, that is a finding.
- **Three things still wait on `calamine` #694, confirmed open 2026-10-06, and none is yours:**
  `fuzz/corpus-quarantine/` keeps its seed; `fuzz_self_comparison` stays out of the `fuzz-smoke`
  matrix; `fuzz_open_xlsx_bytes` stays at `-runs=0`. **And a fourth, recorded this cycle:** the
  alignment invariant is reachable only by `fuzz_self_comparison`, so it is unfuzzed in CI until that
  returns. See `fuzz/corpus-quarantine/README.md`.
- **RFC-028's assurance row must still read `Partially`.**
- **`benches/memory.rs:382`** is still knowingly left alone, as in the last five cuts.

## Required implementation

1. **Stamp `[Unreleased]` as `[3.5.0]`** with today's date, in the form `[3.4.0]` uses.

2. **Section order.** `Added`, `Changed`, `Documentation` is what is there and it matches the house
   order with Documentation last. Leave it.

3. **Write the release summary paragraph. Lead with the two behaviour changes, not the additions** —
   a reader deciding whether to upgrade needs what might alter their logic before what they can newly
   do. It must:
   - say that a sheet which reported `Exact` under `RowKey` or `RowSignature` with a repeated key or
     signature now reports `Medium`, and that **this is the fix, not a regression**: that `Exact` sat
     beside the engine's own warning that the pairing may be a guess;
   - say the new `duplicate_row_signature` warning appears under `RowSignature`, in terms a consumer
     counting diagnostics can match against;
   - **then** `row_placement`: what it carries, that a change is identified by its address *together
     with* its placement, and that collapsing by address merges distinct changes — the instruction we
     published before 3.4.0 and corrected in it;
   - say the navigation route exists, that `next_after` and `previous_before` are deprecated with
     **unchanged behaviour**, and that a `ChangeKey` identifies a change within one comparison and
     not across two;
   - say the JSON gains `row_placement` on every cell diff, and `reasons` on an alignment summary;
   - say why it is a minor, in the terms above;
   - **credit where this came from.** ForskScope asked for the row-space field and were blocked on it
     for a release; the shape went through two revisions because they said the first was unusable.
     The reason value is a set because they showed a single value would hide the veto their gate turns
     on behind the cause they ignore. The invariant is theirs — *"you are not short of a mode
     constructed in a test, you are short of an invariant"* — including the limitation it carries.
     Say so, as 3.3.0 and 3.4.0 did.

4. **Bump `Cargo.toml` to 3.5.0**; update both lockfiles. The benchmark's `sheets-diff 1.2.0`
   dev-dependency stays untouched — it is the subject of the `deny.toml` exemption.

5. **Run the pre-release sweep** — the same 17 commands, each with its exit status and a line of real
   output, one scratch target dir for the whole sweep, deleted when the evidence is captured — plus:

   - **`cargo public-api --all-features diff 3.4.0..HEAD`.** Expect **additions only**: Removed and
     Changed both `(none)`. The commit-range form does an in-place checkout and fails on a dirty
     tree; use a detached worktree or two rustdoc-JSON files, as every unit did. **`#[deprecated]` is
     an attribute `public-api` does not list**, so the two deprecations will not appear there — check
     them in the source and confirm the CHANGELOG says so.
   - **`cargo publish --dry-run` at 3.5.0.** Check the packaged list for `.git-exclude/` (none) and
     `fuzz/corpus*` (none), and for the **four new fixture files** under
     `tests/fixtures/generated/duplicate_row_signature/`.
   - **Goldens:** 27 scenarios, **25 carry `row_placement`** (the two with no cell diffs do not), and
     **no golden carries a populated `alignment_summary`**, because the golden path is `Positional`.
     Verify rather than assume — if a golden gained `reasons`, something is wrong.
   - **The documented install command at 3.5.0**, with `--format json` populating `iso`.
   - **A last read of `docs/` and the CHANGELOG against each other.** Nine sweeps have found
     something every time. The likely places this cycle: any statement that `confidence` is derived
     from the counts; any remaining claim that one address carries one `CellDiff` without naming the
     mode; `ChangeAnchor` described as an identifier or as stable; whether `RowSignature`'s
     documentation lists both of its warnings; and whether the migration page's two sections agree
     with `src/`. **Use `grep -rn "^\s*#\[allow("`, not `grep -rn "#\[allow("`.**

## Required tests

No new tests. The suite is the test: **458 all-features across 30 suites, 424 default across 29**,
measured 2026-10-06, up from 3.4.0's 420/386. Doctests **51 + 13**. Report the numbers you get; a
difference is a finding, not a rounding.

## Acceptance criteria

1. `[3.5.0]` stamped with today's date; **zero non-blank characters** between `[Unreleased]` and it.
2. The summary paragraph covering all seven points of *Required implementation* 3, leading with the
   behaviour changes.
3. `Cargo.toml` at 3.5.0; exactly two `version = "3.5.0"` lines across the two lockfiles; the
   `sheets-diff 1.2.0` dev-dependency untouched.
4. MSRV confirmed 1.88.0.
5. `public-api` range diff: additions only, and the two deprecations confirmed in source.
6. `publish --dry-run` list checked.
7. Golden state confirmed: 27 / 25 / no populated `alignment_summary`.
8. Test and doctest totals reported.
9. Full sweep green, rule 003 included, one scratch dir, deleted.
10. **Nothing committed** — the four release files uncommitted alongside five units' work.

## Prohibited shortcuts

- Do not commit, tag, push or publish.
- Do not reword a reviewed CHANGELOG entry.
- Do not fix what the sweep finds. Report it.
- Do not touch `deny.toml`.
- Do not use `public-api`'s commit-range form against the dirty tree.
- Do not read a green `fuzz-smoke` as evidence anything was fuzzed this cycle, and do not read it as
  covering the alignment invariant at all.

## Known risks

**1. Five units' work in one tree, and this is the first time they are assembled.** `src/model.rs` is
touched by three of them and `src/align.rs` by three. The `public-api` range diff is what catches a
bad interaction. **Read both files' full diffs once before you start.**

**2. Two behaviour changes in one release.** Both are defect fixes and both are consumer-visible. The
CHANGELOG has to let a reader work out whether they were affected, which is why the summary leads
with them rather than with the field.

**3. The entries were written before the version was decided.** They say `[Unreleased]`, which is
correct. **Check that none says "patch"** or names a version.

**4. `cargo deny` is green only because of a committed exemption** whose three conditions are stated
in `deny.toml`. If it fails, those conditions may have stopped holding — a finding, and mine.

## Required evidence

Under `.git-exclude/review-request/release-3.5.0-01-release-preparation/evidence/`:

1. The CHANGELOG stamp and summary, as a diff.
2. `Cargo.toml` and both lockfile diffs.
3. The `public-api` range diff, by whichever isolation method you used, plus the deprecations shown
   in source.
4. `publish --dry-run`'s packaged list.
5. The golden state: the three numbers.
6. Test and doctest totals.
7. The full sweep, one line of real output per command; scratch dir created and deleted.
8. The `docs/`-versus-CHANGELOG read, and what it found.

## Review request format

`.git-exclude/review-request/release-3.5.0-01-release-preparation/README.md`, with:

- The summary paragraph quoted in full, so I review the words.
- The totals, and whether they match the numbers above.
- What the `public-api` range diff printed.
- The golden state.
- Anything the `docs/` read found. **Every cut so far has found something; if this one finds nothing,
  say so explicitly** so I know it was looked for rather than skipped.
