# Handoff 01 — The row space a `CellDiff`'s address is numbered in

**Unit:** row-space-and-the-anchor 01. **Added 2026-10-05**, **shape revised the same day** —
see `.git-exclude/decisions/006-the-row-space-field.md` §8. If you read an earlier copy of this
handoff describing a three-variant `RowSpace` enum, that shape is superseded; §8.1 says why.
**Scoped by:** the architect, from decision 006 §8.2.
**Semver:** minor. `CellDiff` is `#[non_exhaustive]`, so a field is additive.
**Read `README.md` in this directory first** — it carries the reproduction and why the corpus cannot
cover this.

## Purpose

`CellDiff` carries one `address` and no indication of which sheet's row numbering that address
uses, nor of what the row is numbered in the other sheet. Add both. **The engine already has all of
it** at a single site: `src/diff.rs:534` produces `(row, col, old_lookup, new_lookup)` from the
`CoordKey` discriminant, where `old_lookup` and `new_lookup` *are* the old-side and new-side row
numbers. They are used for cell lookup and discarded eleven lines later, at the one `CellDiff`
construction site, `:585`.

This unit computes nothing new. It stops throwing away what is already correct.

## Background you need

Three things make this more than a convenience field.

**1. Two `CellDiff`s can share one address.** See the README's reproduction: `B2 Modified` (old row
space) and `B2 Added` (new row space) in one sheet. Without the field, a consumer holding one
`(row, col)` per change cannot represent both.

**2. Our own public doc instructs consumers to merge them.** `src/model.rs:526`, in bold:

> *"**One `CellDiff` per logical address.** … Consumers migrating from a per-facet model should
> **collapse to one row per address** rather than preserve the split."*

A consumer who follows that sentence merges an inserted row's cell with a matched row's cell.
**The field is not an addition to the identity — it is the missing part of an identity we already
claim is unique.** Unit 00 fixes that paragraph as a doc-only patch; this unit must keep it true.

**3. The consumer needs both row numbers, not just the row space.** They do not render a cell list.
They derive two text documents, one per side, and run those through their own text diff so the panes
line up. Each pane prints its own address:

```
  B2 [value]: 1200        <- left pane, old side
  B2 [value]: 1350        <- right pane, new side
```

Under an aligned mode a matched row is numbered in the old sheet's space, so the right pane would
print a row number that is not where that row lives in the new file. Their words: *"it is a false
statement about a file the user still has open"*, and *"if the field lands without it we cannot use
aligned output."* A diff tool printing a row number that does not exist at that position in the file
it names is D-03's defect class one layer out. **This is why the shape carries the numbers and not
just a label.**

## The shape

```rust
/// Which row of each sheet this change belongs to.
///
/// [`CellDiff::address`] carries one row number, and which sheet that number belongs to depends on
/// how the row was aligned. Read the row numbers from here, not from `address`, whenever a sheet's
/// own numbering matters — for instance when showing a user an address in a file they have open.
pub enum RowPlacement {
    /// Alignment paired this row across the sheets. `address.row` is `old_row`; the same row is
    /// numbered `new_row` in the new sheet. The two numbers are often unequal, and may be equal.
    PairedByAlignment { old_row: u32, new_row: u32 },
    /// Alignment found no counterpart in the new sheet. `address.row` is `old_row`; there is no
    /// new-sheet row number for this row.
    UnpairedInOldSheet { old_row: u32 },
    /// Alignment found no counterpart in the old sheet. `address.row` is `new_row`; there is no
    /// old-sheet row number for this row.
    UnpairedInNewSheet { new_row: u32 },
    /// No alignment ran: row N of the old sheet was compared with row N of the new sheet, so the
    /// one number is correct in both. `address.row` is `row`.
    ComparedPositionally { row: u32 },
}
```

…and `pub row_placement: RowPlacement` on `CellDiff`.

Four things about it are decisions, not preferences, and the review will check them.

**One enum, not a label plus two `Option<u32>` fields.** Three flat fields can express states the
engine cannot produce — an unpaired row carrying a counterpart number, a paired row missing one —
and a consumer then has to work out which field wins. The enum makes those unrepresentable, and
makes the useful case structural: for `UnpairedInOldSheet` the consumer's new-side pane prints
nothing for that row, and the type says so instead of the consumer having to remember.

**Every variant says what `address.row` is.** That is the promise we made in writing — *"we will
name it so the framing is unavoidable"* — discharged in the doc comments rather than hoped for. A
reader who looks at one variant learns both which sheet `address.row` belongs to and what the other
sheet calls that row.

**No `Added`/`Removed`/`Modified` vocabulary.** `CellChangeKind` already owns those three words for
the *cell's* change, and a row's pairing is a different question — a cell inside a paired row can
still be `Removed`. Reusing the words would create exactly the misreading this field exists to
prevent.

**Exhaustive — no `#[non_exhaustive]`.** RFC-031 §6's criterion is "public enums likely to grow";
this domain is closed at four, and a fifth would need a new alignment concept rather than a new
case. `#[non_exhaustive]` would force consumers to write `_ =>`, which silently absorbs a future
variant and places changes in the wrong row space — D-03's failure class in a consumer's code. A
compile error is the better failure. `AlignmentMode` is the existing precedent, and the consumer
endorsed this deviation unprompted.

I considered marking the individual variants `#[non_exhaustive]` so their fields could grow, and
rejected it: it forces `..` into every pattern, and the only plausible growth is a per-row
confidence, which belongs on `CellDiff` or `AlignmentSummary` rather than inside a row's placement.
**Be aware of the consequence:** adding a field to `PairedByAlignment` later would be breaking.

If you think any of the four is wrong, **stop and say so before implementing** — a disagreement here
is cheap now and expensive after 25 goldens move. This shape has already been revised once, after
the consumer told us the first version was unusable for them; a third revision is worth having if it
is right, and worth avoiding if it is not.

## Change scope

- `src/model.rs` — the enum and the field. Unit 00 owns the "one `CellDiff` per logical address"
  paragraph; **keep what unit 00 wrote true** rather than rewriting it.
- `src/diff.rs` — carry the discriminant *and* the two lookups from the existing `match *key` at
  ~`:534` into the construction at ~`:585`. A few lines; if it is not, report why before continuing.
- `tests/integration.rs` — the shared reproduction helper and the dedicated assertions.
- `tests/fixtures/generated/*/expected.json` — all 25, blessed.
- `docs/src/semantics.md` — row placement beside the alignment section.
- `docs/src/maintainers/threat-model.md` ~`:535` — it already says *"Two correctly-computed diffs can
  share a display address"*, which is the one place in the book that admitted what `CellDiff`'s own
  doc denied. Its suggested disambiguator is *"via which `CellChangeKind` applies"*, and that is
  **wrong**: `change_kind()` returns `Added`/`Removed`/`Modified` and says nothing about row space,
  so two `Modified` changes at one address stay indistinguishable by it. Replace the hint with
  `row_placement`. Found by unit 00's sweep and left for this unit, correctly.
- `rfcs/done/033-public-model-lexicon.md` §5 — it pins `CellDiff`'s exact shape.
- `docs/src/migration/` — a note for consumers who were told to collapse by address.

## Non-change scope

- **`src/output/view.rs` is unit 02's.** Do not touch it here, even though it is obviously affected.
- Do not expose `compute_row_mapping` or any part of `mod align`.
- Do not change how alignment works, and do not alter `old_lookup`/`new_lookup` — they are load-
  bearing for cell lookup. This unit reads them; it does not move them.
- Do not add a per-row confidence.

## Required implementation

**1. The enum and the field**, exactly as above, with those doc comments.

**2. Feed it from the `CoordKey` discriminant and the two lookups.** The mapping is total:

| `CoordKey` | lookups | `RowPlacement` |
|---|---|---|
| `Old(r, _)` | `new_lookup = Some(n)` | `PairedByAlignment { old_row: r, new_row: n }` |
| `Old(r, _)` | `new_lookup = None` | `UnpairedInOldSheet { old_row: r }` |
| `InsertedNew(r, _)` | — | `UnpairedInNewSheet { new_row: r }` |
| `Positional(r, _)` | — | `ComparedPositionally { row: r }` |

**The discriminant is required; the numbers alone are not enough.** Under `RowKey`, a matched row
whose key is unchanged with no insertions above it maps old-row 2 to new-row 2 — so
`PairedByAlignment { 2, 2 }` and `ComparedPositionally { 2 }` carry identical numbers and mean
different things. **Do not derive the variant from whether the lookups are `Some` or equal.** That
is inference from a side effect, and it is how this information was lost in the first place.

**3. Bless the goldens, and read the diff.** `tests/fixtures/corpus/README.md` is explicit that
blessing without reading is the failure mode. Expect **all 25** to change, every cell diff gaining a
tagged `ComparedPositionally` object, because the golden test uses default options. **If any golden
shows a different variant, something is wrong** — the golden path cannot reach the other three. Say
in the review that you confirmed this, with the per-file shape of the diff.

**4. Records.** RFC-033 §5, `docs/src/semantics.md`, the migration note.

## Required tests

All in `tests/integration.rs`, under non-default alignment, because the corpus cannot reach this.
Extend the README's reproduction so `k1`'s value changes too — that gives a matched row whose old
and new numbers are both 1, which is what tests 3 and 4 need.

1. **Two `CellDiff`s at one address, distinguished.** Two entries with `address.a1 == "B2"`, one
   `PairedByAlignment { old_row: 2, new_row: 3 }` + `Modified`, one
   `UnpairedInNewSheet { new_row: 2 }` + `Added`. **This is the test that proves the field carries
   information**; the golden does not.
2. **A paired row's new-side number is the new sheet's.** Assert `new_row == 3` for k2 — not merely
   that the variant is `PairedByAlignment`. This is the consumer's actual requirement: a test that
   checks only the variant would pass with the number wrong, which is this quarter's recurring
   defect in miniature.
3. **An unpaired old row carries no new number**, structurally: `kdel` is `UnpairedInOldSheet`, and
   there is no new-side number to read.
4. **`PairedByAlignment { 1, 1 }` is not `ComparedPositionally { 1 }`.** k1 under `RowKey` must be
   the former. This is the test that pins *Required implementation* 2 — the one a shortcut breaks.
5. **Default options give `ComparedPositionally`** for every cell diff, with the row number equal to
   `address.row`.
6. **`RowSignature` alignment** also produces paired and unpaired placements, with correct numbers.
   `RowKey` is not the only non-default mode, and a fix that only worked for one would pass 1–5.
8. **Two tests that behave like the documented consumer** — the point of the unit, so read this
   twice. `CellDiff`'s doc comment describes a consumer model, and **nothing we run has ever
   consumed it.** That is why the instruction could be wrong for a whole release without a single
   test failing.

   **8a — the corrected model is lossless.** Build the consumer unit 00's corrected paragraph
   describes: key every `CellDiff` from the reproduction by whatever the doc says identifies a
   change, and assert nothing is overwritten and the entry count equals `cell_diffs.len()`. It must
   fail if the key omits the row placement.

   **8b — the old instruction loses a change, permanently asserted.** Implement the instruction we
   published — key by address alone, collapse — and assert that it **drops an entry**. Name it after
   the instruction and quote the old sentence in a comment above it.

   Why 8b rather than a transient red commit: the consumer asked whether we would *"make it fail
   loudly or make it pass by correcting the sentence first"*, and said they would keep it failing
   for one commit so the test's own history records that the instruction was wrong. We cannot land a
   red commit on `main` — the gates are the gates — but we can do better than history. **8b records
   the fact in the suite itself, permanently and in the present tense**, where a reader finds it
   without running `git log`. It also keeps earning its place: if addresses ever become unique — the
   v4 anchor work could do it — 8b fails and tells whoever did it that the doc can now be
   simplified. A fact in a test that still runs beats a fact in a commit message.

9. **Failing-first for 1, 2, 4, 6 and 8a**, by short-circuiting the specific mapping arm — one edit at a
   time, `cmp`-verified restore between each, never two live at once. A test that passes against a
   mutation that should break it is the thing we keep finding.

## Acceptance criteria

1. The field and enum land with the agreed names and payloads, exhaustive, on `CellDiff`.
2. Fed from the discriminant plus the lookups, per the table — not inferred.
3. All nine tests (8a and 8b both), each with its failing-first demonstration where required — 8a
   demonstrated to fail with the placement dropped from the key, and 8b named after the instruction
   it pins.
4. All 25 goldens blessed, the diff read, and the uniform `ComparedPositionally` result confirmed
   and reported.
5. Unit 00's corrected paragraph still true afterwards.
6. RFC-033 §5, `docs/src/semantics.md` and a migration note updated.
7. Gates green, rule 003 swept, one scratch dir, deleted. Nothing committed.
8. `cargo public-api` diff shows the addition and **nothing else** — detached worktree, not a stash;
   the commit-range form does an in-place checkout and fails on a dirty tree.

## Prohibited shortcuts

- Do not shorten the variant names or drop the payloads. Both were promised to a consumer in writing
  and the payloads are the reason they can use the field at all.
- Do not add `#[non_exhaustive]` to `RowPlacement` "to be safe". It is the less safe choice here and
  decision 006 §8.2 says why.
- Do not derive the variant from the lookups. Test 4 exists to catch it.
- Do not bless goldens before reading the diff.
- Do not quietly fix `view.rs` because you noticed it too. Report it to unit 02's scope.

## Known risks

**1. The JSON output changes more visibly than a plain label would.** `CellDiff` derives
`Serialize`, and a data-carrying enum serialises as a tagged object rather than a string. We
implement no `Deserialize`, so nothing round-trips and no key is lost — but this is consumer-visible
and belongs in the release notes. Note it in the review so it reaches them.

**2. RFC-031 §8 says *adding optional fields* is minor-compatible, and `row_placement` is always
present.** I read that as covered — no `Deserialize`, nothing removed or renamed — but §8's wording
does not literally address an always-present addition, still less a tagged object. **Flag it; do not
amend §8 yourself.** That is mine.

**3. Hardcoded corpus counts.** M9 unit 05 had to bump several. This unit adds no scenario, so they
should not move. If one does, stop and report — it means something other than the field changed.

**4. The shape has already been revised once.** It changed because I deferred the new-side row
number and the consumer told us the field would be unusable without it. If something in the four
decisions looks wrong to you, that history is the reason to say so now rather than assume it was
settled by someone with more information.

## Required evidence

Under `.git-exclude/review-request/row-space-01-the-row-placement/evidence/`:

1. The `CoordKey` + lookups → `RowPlacement` threading as a diff, showing it comes from the
   discriminant and not from the `Option`s.
2. Each of the nine tests, with its failing-first mutation and the `cmp`-verified restore. For 8a,
   also the run with the row placement dropped from the key, showing it fails. For 8b, the dropped
   entry it observes, by address.
3. The golden diff: that all 25 moved, uniformly `ComparedPositionally`, with the number matching
   `address.row`.
4. `cargo public-api` output from a detached worktree.
5. Gate sweep across both manifests; the scratch dir created and deleted.

## Review request format

`.git-exclude/review-request/row-space-01-the-row-placement/README.md`, with:

- Whether you agreed with the four settled decisions, and if not, what you would have done.
- Confirmation that every golden shows `ComparedPositionally`, and that you understand why.
- The RFC-031 §8 question, stated for me to decide.
- Anything in `view.rs` you noticed and left for unit 02.
- Whether the mapping table in *Required implementation* 2 was total in practice, or whether you
  found a `CoordKey` and lookup combination it does not cover. If you found one, that is a finding
  about the engine, not about this table.
- For 8a: what you used as the key, and whether the corrected doc comment was precise enough to
  write the test from. **If you had to guess what identifies a change, the doc is still wrong** —
  report that as a finding against unit 00's wording rather than choosing a key and moving on. The
  whole value of that test is that the prose and the code say the same thing.
