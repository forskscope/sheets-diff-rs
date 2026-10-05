# Handoff 00 — Three sentences that are false in shipped code

**Unit:** row-space-and-the-anchor 00. **Added 2026-10-05.**
**Scoped by:** the architect, from `.git-exclude/decisions/006-the-row-space-field.md` §8.3.
**Semver:** patch. **Documentation only — not one line of behaviour.**
**Release: 3.4.0**, decided by the owner 2026-10-06 — this unit **and**
`the-row-that-vanishes/01` (a changed cell that is never reported), as two separate reviews in one
release. It is a minor because that unit adds a `DiagnosticKind` variant; **this unit is still
documentation only and its own `cargo public-api` diff must still be empty.** **That unit goes first**; this one is still ahead of 01 and 02 of this milestone.

## Purpose

Two published doc comments in 3.3.0 state things that are not true. One of them tells consumers to
do something that produces a wrong answer. Correcting prose carries no behaviour risk, and there is
no reason a consumer reading our docs between now and 3.4.0 should be given the wrong instruction.

## The three sentences

**Revised 2026-10-06:** a third was found while answering a consumer's remark. It was two when this
handoff was written; the title and the criteria below are updated.

**1. `src/model.rs:526` — this is the dangerous one.** On `CellDiff`, in bold:

> *"**One `CellDiff` per logical address.** … The `output::view::CellChangeRow` projection follows
> the same rule (one row per address…). Consumers migrating from a per-facet model should **collapse
> to one row per address** rather than preserve the split."*

Under `RowKey` or `RowSignature` alignment, two `CellDiff`s can carry the identical address — one in
each row space. See `README.md` in this directory for the three-row reproduction. **A consumer who
follows that instruction merges two distinct changes into one.** We have direct evidence this is
being read: a consumer told us they read it and did not follow it *"by luck rather than
judgement"*, because their conversion pushes into a vector instead of keying by address.

**2. `src/model.rs:884` — `SheetDiff.alignment_summary`'s field doc, in full:**

> *"Reserved until RFC-011."*

**RFC-011 is in `rfcs/done/` and the field is populated.** It carries a real `AlignmentSummary`
under every non-`Positional` mode, and a consumer gating on it has been reading a field our own doc
calls reserved. The type's doc above it is correct (`None` when the mode is `Positional`); the
field's is simply stale.

While you are there: **say why it is `None`.** `Positional` performs no alignment, so there are no
alignment decisions to summarise — which a consumer told us they had to work out from the code, and
which matters because it means confidence cannot be compared across a positional leg and an aligned
leg of the same comparison. One sentence.

Do **not** touch `src/model.rs:543`'s *"Reserved until RFC-022."* — RFC-022 is still in
`rfcs/accepted/` and blocked upstream, so that one is true.

**3. `src/output/view.rs:40` — `ChangeAnchor`'s doc:**

> *"A stable, deterministic identifier for a single change row."*

It is stable and deterministic. It does not identify: `{ sheet_index, row, col }` cannot distinguish
two rows in different row spaces that share a row number. `next_after` finds its argument by anchor
equality and therefore returns the same row forever when two anchors collide — measured at 8 steps
on a 5-row list in the README.

## Change scope

- `src/model.rs` — **two separate places:** the `CellDiff` doc comment, **and
  `SheetDiff.alignment_summary`'s field doc at ~`:884`** (sentence 2 above). The second was added to
  the descriptive section on 2026-10-06 and not to this one, which is why the first pass through
  this unit missed it. Leave `src/model.rs:543`'s *"Reserved until RFC-022"* alone — that one is true.
- `src/output/view.rs` — `ChangeAnchor`'s doc comment, `next_after`/`previous_before`, and
  `CellChangeRow::anchor`'s field doc, which carries the same false claim on the field a navigation
  consumer reads first.
- `CHANGELOG.md` — a patch entry.
- `docs/src/` — anywhere the same claim is repeated in the book. **Sweep for it**; do not assume
  `src/` is the only place either sentence appears.

## Non-change scope

- **No behaviour. No signatures. No `#[deprecated]` attributes.** The attribute produces new
  warnings in downstream builds, which is not patch behaviour; it is unit 02's, in 3.4.0.
- Do not add `row_placement` or mention it by name as shipping — it has not shipped. You may say
  that a future minor will carry the row numbers, with no version number attached.
- Do not fix the hang. Unit 02.

## Required implementation

**1. Make `CellDiff`'s paragraph true.** The uniqueness claim is wrong as written, and the
instruction built on it is harmful. Say plainly that one address can carry more than one `CellDiff`
under a non-`Positional` alignment mode, that the two are distinct changes in different row-number
spaces, and that collapsing by address merges them. **Keep the part that is true** — a value change
and a formula change at one address really are facets of one entry — and separate it clearly from
the part that is not. A consumer should be able to tell from the paragraph alone whether their
conversion is safe.

**2. Delete or correct the sentence about `output::view::CellChangeRow` following "the same rule".**
It does not. Do not promise what unit 02 will do.

**2b. Correct `SheetDiff.alignment_summary`'s field doc** (`src/model.rs` ~`:884`), which reads
*"Reserved until RFC-011."* RFC-011 is in `rfcs/done/` and the field is populated. Say what it
carries, and add **one sentence on why it is `None` under `Positional`**: no alignment ran, so there
are no alignment decisions to summarise. That sentence matters beyond accuracy — it means confidence
cannot be compared across a positional leg and an aligned leg of the same comparison, which a
consumer had to work out from the source.

**3. Make `ChangeAnchor`'s doc true.** It is not an identifier. Say what it is — a deterministic
sort position — and say plainly that two change rows can share one anchor under non-`Positional`
alignment, with the consequence spelled out: `next_after` and `previous_before` can fail to advance.

**4. Say it on the methods too**, not only on the type. Someone reading `next_after`'s one-line doc
will not look up `ChangeAnchor`.

**4b. Keep the CHANGELOG entry at the file's prevailing wrap width.** The `### Documentation` entry
wraps wider than the `### Security` entry above it; match the narrower one.

**5. Write for someone who has already hit it.** The person most likely to read these words is
debugging a navigation button that stopped working. Give them the cause in the first sentence.

## Required tests

None, and that is deliberate — there is no behaviour here. Two things instead:

1. `cargo test --doc` passes, and any doctest you touch still compiles. If a doctest demonstrated
   the wrong instruction, that is a finding: report it, because it means the claim was executable.
2. **`cargo public-api` shows an empty diff.** This is the unit's real check: it proves the patch is
   documentation only. Run it in a detached worktree — the commit-range form does an in-place
   `git checkout` and fails on a dirty tree.

## Acceptance criteria

1. None of the three states anything false, and `CellDiff`'s paragraph no longer instructs a
   consumer into a merge.
1b. `alignment_summary` is no longer described as reserved, and says why it is `None` under
   `Positional`. `src/model.rs:543` is untouched.
2. Both deprecated-to-be methods carry the warning in their own docs.
3. The book swept; every repetition found and fixed, or confirmed there are none.
4. `cargo public-api` diff **empty**.
5. `git diff` touches no executable line. Say so in the review request and show it.
6. Gates green, rule 003, one scratch dir, deleted. Nothing committed.
7. A `CHANGELOG.md` patch entry that says what was wrong, not that docs were "improved".

## Prohibited shortcuts

- Do not soften any of the three into vagueness. "May not be unique in some cases" helps nobody. Name
  the modes, name the cause.
- Do not change behaviour "while you are in there", however small and however obviously right. A
  patch whose `public-api` diff is empty and whose `git diff` is prose only is the point of this
  unit.
- Do not describe `row_placement` as available.

## Known risks

**1. The temptation to fix the hang.** It is four lines away and you will see it. It is unit 02's,
in a minor, with a replacement route alongside — because there is no correct patch for it. The
decision record's §5 has the three candidates I rejected and why; read it before proposing one.

**2. The claim may appear in more places than the three named.** `grep -rn "Reserved until" src/`
returns exactly two hits, one true and one false — that one is settled. The other two claims are
prose and may be paraphrased elsewhere. `docs/src/` has a semantics chapter
and a migration guide. Grep for the claim, not for the sentence — a paraphrase is just as wrong.

**3. The release is decided, so the CHANGELOG entry is real.** 3.4.0 carries this unit and
`the-row-that-vanishes/01`. Write your entries under that heading without restructuring the section
around them — the other unit adds its own, and whichever of you lands second should not be rewriting
the first one's wording.

## Required evidence

Under `.git-exclude/review-request/row-space-00-two-false-sentences/evidence/`:

1. The before and after of both passages, in full.
2. The `docs/src/` sweep: what you searched for and what you found.
3. `cargo public-api` showing an empty diff, from a detached worktree.
4. `git diff --stat` plus the full diff, demonstrating prose only.
5. Gate sweep across both manifests; scratch dir created and deleted.

## Review request format

`.git-exclude/review-request/row-space-00-two-false-sentences/README.md`, with:

- Both new passages quoted in full, so I review the words and not a description of them.
- Whether you found the claim anywhere else.
- Whether anything executable asserted the old claim — a doctest, an example, a test name.
- Anything you wanted to fix and left alone.
