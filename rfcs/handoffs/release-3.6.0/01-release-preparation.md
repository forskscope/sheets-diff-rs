# Handoff 01 — 3.6.0 release preparation

**Unit:** release-3.6.0 01. **Added 2026-10-07.** **Scoped by:** the architect.
**Authorised by the owner 2026-10-07.** **The cut is the owner's and mine — this unit prepares it and
stops.**

## What 3.6.0 is — and it is the first release this quarter with no behaviour change

**Three units**, all reviewed and approved, all uncommitted in the working tree:

| Unit | What | Public API |
|---|---|---|
| `never-hold-a-version-you-could-read/01` | the `versions` CI gate: 26 version claims across 21 sites, read from `Cargo.toml` and `Cargo.lock`; two prose corrections | none |
| `formulas-move-with-their-rows/01` | the formula reference tokenizer and mapper, refusal-first | **none** — `pub(crate)` only |
| `formulas-move-with-their-rows/02` | `FormulaChange::difference` and `FormulaDifference` | additive |

**Nothing a consumer observes changes except by addition.** 3.5.0 shipped two behaviour changes and
told readers to re-check results; **3.6.0 changes no reported value, no reported change, and no
count.** The annotation is information added beside what was already there, and unit 02's review
established that structurally rather than by sampling: both of `compare_formulas`'s early returns
precede the call that computes the annotation, so it cannot reach the decision to report.

**Say that plainly in the summary.** A consumer who read 3.5.0's notes is primed to expect another
round of re-checking, and the useful first sentence tells them there is none.

## Why minor

Additive public API — `FormulaDifference` with four variants, and `FormulaChange::difference` — and
nothing removed or re-signed. `compare_formulas`'s signature changed, and that is **not** public:
it lives in `pub(crate) mod compare`. Confirm that rather than take it from here.

## The tree you are starting from

All three units' work is uncommitted. **Leave it that way**; your four release files join it, and the
owner and I commit, tag and publish the whole set.

Two files under `rfcs/handoffs/formulas-move-with-their-rows/` are also modified — the architect's
corrections to a misattribution in his own handoffs. They are records, not release content, and they
travel in the same commit.

## Change scope

- `CHANGELOG.md` — stamp `[Unreleased]` as `[3.6.0]`, add the summary paragraph.
- `Cargo.toml` — `3.5.0` → `3.6.0`; both lockfiles follow.
- Nothing else.

## Non-change scope

- **Do not commit, tag, push or publish.**
- Nothing under `src/`, `tests/`, `docs/`, `rfcs/`, `fuzz/`, `.github/`. **If the sweep finds
  something, report it.**
- **Do not add or reword an existing CHANGELOG entry.** All three units' entries have been reviewed,
  including the release note for unit 02, whose words I reviewed specifically.
- **Keep `## [Unreleased]` above the stamped heading**, empty, with zero non-blank characters between
  them. It was removed at 3.5.0's prep and restored on review; the reason is that without it the next
  unit's entry lands under a shipped release.
- **MSRV stays 1.88.0.** Confirm. Two units need it exactly — the invariant's let-chain and the
  navigation tests' `#[expect]`, both already shipped.
- **`deny.toml` is not yours.** The `RUSTSEC-2026-0317` exemption is committed with its three stated
  conditions.
- **Four things still wait on `calamine` #694, confirmed open 2026-10-07:** the quarantine's seed,
  `fuzz_self_comparison`'s absence from the matrix, `fuzz_open_xlsx_bytes` at `-runs=0`, and — recorded
  last cycle — **the alignment invariant being unfuzzed in CI**, since the only target that reaches it
  is the absent one. `calamine` #729, our patch, is also still open.
- **RFC-028's assurance row still reads `Partially`.** `benches/memory.rs:382` still knowingly alone.

## Required implementation

1. **Stamp `[Unreleased]` as `[3.6.0]`** with today's date, in the form `[3.5.0]` uses.

2. **Section order** is `Added`, `Changed`, `Documentation` — the house order with Documentation last.
   Leave it.

3. **Write the release summary. Lead with the absence of behaviour change**, then the field. It must:
   - say **no reported change, value or count differs from 3.5.0**, so nothing needs re-checking —
     and that this is established by the structure of the code, not only by comparison;
   - then what `FormulaChange::difference` is for: under `RowKey` or `RowSignature` a formula that was
     never edited was reported as changed because Excel rewrites row references when rows move, and
     filtering **out** `ExplainedByRowMapping` removes that cascade;
   - say **`NotDetermined` means we declined**, not that the formula is unchanged, and that stage 1
     declines whole classes — the entry already names them, so the summary need only point at it;
   - say that **`RowKey`'s documentation was wrong**: it claimed to reduce cascades after row
     insertion and deletion without qualification, and a consumer may have relied on that. It now says
     it removes the value cascade and not the formula one;
   - say the **`versions` CI gate** exists and what it is for, in one sentence — it is maintainer-
     facing, not consumer-facing, so do not give it more room than the field;
   - say why it is a minor;
   - **credit ForskScope.** They measured the formula cascade and asked for the capability, having
     already checked two things about its definition that we would otherwise have had to establish —
     that mapping must go through the row mapping rather than compare relative form, and that `$` does
     not decide it. And the rule *never hold a version you could read* is theirs.

4. **Bump `Cargo.toml` to 3.6.0**; update both lockfiles. The benchmark's `sheets-diff 1.2.0`
   dev-dependency stays untouched.

5. **Run the pre-release sweep** — one scratch target dir, deleted when the evidence is captured —
   **now 18 commands, because the `versions` gate is new this cycle.** Run it: a stale version claim
   blocks the release job by `needs`, so a failure here is a release blocker rather than a note. Plus:

   - **`cargo public-api --all-features diff 3.5.0..HEAD`.** Expect **additions only**: Removed and
     Changed both `(none)`. The commit-range form fails on a dirty tree — use a detached worktree or
     two rustdoc-JSON files. **Confirm `compare_formulas` does not appear**, which is what proves its
     signature change is private.
   - **`cargo publish --dry-run` at 3.6.0.** `.git-exclude/` absent, `fuzz/corpus*` absent, and
     **`.github/scripts/check-version-claims.py` present or absent — report which**, since it is new
     and I do not know what `Cargo.toml`'s packaging rules do with it.
   - **Goldens: 27 scenarios, 4 changed**, each gaining exactly one `"difference": "NoRowMovement"`,
     and **no entry added or removed**. The corpus count stays 27 — no scenario was added, because
     the declined-class tests build their own workbooks.
   - **The documented install command at 3.6.0**, with `--format json` populating `iso`.
   - **A last read of `docs/` and the CHANGELOG against each other.** Ten cuts have found something
     nine times. The likely places this cycle: any remaining unqualified claim that an alignment mode
     reduces cascades; whether `NotDetermined`'s meaning is stated the same way in `src/model.rs`,
     `src/options.rs`, the migration page and the CHANGELOG; and whether the two migration pages
     (`v3.4-to-v3.5.md`, `v3.5-to-v3.6.md`) agree with each other about `row_placement`.

## Required tests

No new tests. **462 default across 30 suites, 496 all-features across 31**, doctests **53 + 13**,
measured 2026-10-07. Report the numbers you get; a difference is a finding.

## Acceptance criteria

1. `[3.6.0]` stamped; `[Unreleased]` retained above it with zero non-blank characters between.
2. The summary covering all seven points of *Required implementation* 3, leading with the absence of
   behaviour change.
3. `Cargo.toml` at 3.6.0; exactly two `version = "3.6.0"` lines across the lockfiles; `1.2.0`
   untouched.
4. MSRV confirmed 1.88.0.
5. `public-api` range: additions only, and `compare_formulas` absent from the diff.
6. `publish --dry-run` list checked, with the new script's presence reported either way.
7. Golden state: 27 / 4 / one key each / nothing added or removed.
8. Totals reported.
9. The 18-command sweep green, rule 003 and `versions` included, one scratch dir, deleted.
10. **Nothing committed.**

## Prohibited shortcuts

- Do not commit, tag, push or publish.
- Do not reword a reviewed entry.
- Do not remove `## [Unreleased]`.
- Do not fix what the sweep finds. Report it.
- Do not touch `deny.toml` or the version-claim site list.

## Known risks

**1. Three units in one tree, and `src/` is touched by two of them.** `src/compare.rs`, `src/diff.rs`,
`src/model.rs` and `src/lib.rs` all carry unit 02's work; `src/formula_refs.rs` is unit 01's.
**Read `src/diff.rs`'s full diff once** — it is where the plumbing lands and where a bad interaction
would show.

**2. The `versions` gate is new and now gates the release.** If it fails during prep, a version claim
has gone stale — that is a finding and it is mine, not something to work around by editing the site
list.

**3. The release promises a capability whose usefulness is unmeasured.** `NotDetermined` may be common
on real sheets. The summary must not imply the cascade is gone everywhere; the entry's declined-class
list is what keeps it honest.

## Required evidence

Under `.git-exclude/review-request/release-3.6.0-01-release-preparation/evidence/`:

1. The CHANGELOG stamp and summary, as a diff.
2. `Cargo.toml` and both lockfile diffs.
3. The `public-api` range diff, and `compare_formulas`'s absence from it.
4. `publish --dry-run`'s packaged list.
5. The golden state: the four numbers.
6. Test and doctest totals.
7. The 18-command sweep, one line of real output per command; scratch dir created and deleted.
8. The `docs/`-versus-CHANGELOG read, and what it found.

## Review request format

`.git-exclude/review-request/release-3.6.0-01-release-preparation/README.md`, with:

- The summary quoted in full.
- The totals, and whether they match.
- What the `public-api` range printed, and that `compare_formulas` is absent.
- Whether the new script is packaged.
- Anything the `docs/` read found. **Nine of ten cuts have found something; if this one finds nothing,
  say so explicitly.**
