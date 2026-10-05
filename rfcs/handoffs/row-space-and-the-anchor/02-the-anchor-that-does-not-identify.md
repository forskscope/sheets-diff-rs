# Handoff 02 — The anchor that does not identify, and the navigation that never ends

**Unit:** row-space-and-the-anchor 02. **Added 2026-10-05.**
**Scoped by:** the architect, from `.git-exclude/decisions/006-the-row-space-field.md` §5.
**Semver:** minor — additive only. **The real repair is a major and is deferred to v4 by decision.**
**Read `README.md` in this directory first**, and take units 00 and 01 before this one.
**Revised 2026-10-05:** the documentation half of this unit moved into unit 00, a doc-only patch —
the consumer's point, which I accepted, was *"the deprecation can take its window; the sentence
should not."* What remains here is the replacement route and the deprecation.

## Purpose

`DiffView::next_after` hangs. Not slowly — permanently, on the same row, forever. It is reachable
through the documented public API in shipped 3.3.0 with any non-`Positional` alignment mode, which
is the mode a consumer selects deliberately to get cascade reduction.

## What was measured

On the README's reproduction, walking forward from `first()`:

```
step 0 -> ChangeAnchor { sheet_index: 0, row: 2, col: 2 }
step 1 -> ChangeAnchor { sheet_index: 0, row: 2, col: 2 }
step 2 -> ChangeAnchor { sheet_index: 0, row: 2, col: 2 }
...
walk emitted 8 steps for a 5-row list     (capped at 8; it was still going)
```

Two of the five rows share an anchor. `next_after` finds `current` by **anchor equality**, so it
always matches the *first* `B2` and returns the *second* — including when it is called *with* the
second. A GUI "next change" button stops there and never advances; rows 3 onward are unreachable.
`previous_before` is wrong symmetrically, returning the first `B2`'s predecessor when asked about
the second.

`ChangeAnchor`'s own doc, `src/output/view.rs:40`:

> *"A stable, deterministic identifier for a single change row."*

It is stable and deterministic. **It does not identify.** `{ sheet_index, row, col }` cannot, because
`row` alone does not say which row space it is in — the same gap unit 01 closes on `CellDiff`.

## The constraint that decides this unit's shape

**`next_after` takes `&ChangeAnchor`. When two rows share an anchor, the parameter cannot say which
one the caller meant.** The information needed for a correct answer is not in the argument, so no
change to the body can be correct. I checked all three candidates before writing this:

| Fix | Walk over `[A2, B2ₐ, B2_b, A3, B3]` | Verdict |
|---|---|---|
| Current (first match) | `A2 → B2 → B2 → B2 → …` forever | hangs |
| Match the *last* occurrence | `A2 → B2 → A3 → B3 → None` | terminates, **skips a change** |
| Return `None` on a duplicate | `A2 → B2 → None` | terminates, **drops the rest** |

**Do not implement any of them.** For a diff tool the two terminating options are worse than the
hang: a hang is visible, a silently skipped change is a wrong answer delivered confidently. That is
this quarter's recurring defect category — a marker credited with more than it measured — and we are
not going to close it by trading a visible failure for an invisible one.

Adding the row space to `ChangeAnchor` *would* be correct, and is a **breaking change**:
`src/output/view.rs` contains **no `#[non_exhaustive]` anywhere**, so every struct in it is closed to
added fields under RFC-031 §6. That repair is v4's. This unit is what we can do correctly now.

## Change scope

- `src/output/view.rs` — the new navigation route, the deprecations, the corrected docs.
- `tests/integration.rs` — the bounded reproduction and the new route's properties.
- `docs/src/` — wherever the view API is described, plus a migration note.
- `rfcs/accepted/037-v3-scope.md` (or its v4 successor) — record the deferred repair.

## Non-change scope

- **Do not add a field to `ChangeAnchor`, and do not mark it `#[non_exhaustive]`.** Both are
  breaking. This is the decision, not an oversight.
- Do not remove `next_after` or `previous_before`. RFC-031 §7: deprecate before removal, keep for at
  least one minor, and never remove in a patch.
- Do not change the row ordering or the filtered row set. Ordering is canonical and asserted
  elsewhere.
- Unit 01 owns `src/model.rs` and `src/diff.rs`.

## Required implementation

**1. Reproduce it first, bounded.** A test that hangs is not a test. Step forward a fixed number of
times — more than the row count — and assert the walk terminated and made progress. This test must
fail on current `main` for the right reason, and its failure message must name the duplicate anchor.

**2. Add a correct navigation route.** Shape is yours to propose; what it must satisfy:

- given a position in the filtered row list, it moves **exactly one** row;
- a forward walk from the start visits **every** row **exactly once**, in canonical order, and
  terminates;
- the same backwards;
- it is **additive** — no existing signature changes.

A positional cursor over the filtered list is the obvious candidate; so is returning an index
alongside each row. **Propose before building**, and say what a GUI does with it to restore
scroll position across a re-diff, because that is what `ChangeAnchor` was for and your replacement
has to serve it.

**3. Deprecate `next_after` and `previous_before`** with `#[deprecated]` naming the replacement,
per RFC-031 §7. **Leave their behaviour alone** — see *The constraint* above. A deprecated method
that hangs is bad; a deprecated method that silently skips a change is worse, and we say so in the
doc rather than papering over it.

**4. Extend the docs unit 00 corrected.** Unit 00 already made `ChangeAnchor`'s doc and both
methods' docs state that an anchor is not unique and that these methods can fail to advance — do
not rewrite that from scratch, and do not weaken it. Add the two things unit 00 could not say: the
replacement route by name, and `CellDiff::row_placement` as the reliable way to tell two changes at
one address apart. If unit 00's wording is wrong or incomplete, say so rather than silently
replacing it; it shipped to users and I reviewed it.

**5. `src/model.rs:526` said `CellChangeRow` follows the "one row per address" rule.** Unit 00
corrected or removed that sentence, and unit 01 was told to keep its replacement true. Check what it
says when you start. If your new route changes whether it is accurate, update it — and say which of
the three units ended up owning that sentence, because it has now passed through all of them.

**6. Record the deferred repair** in the v4 scope: `ChangeAnchor` carries the row space, and the
`view.rs` types get `#[non_exhaustive]` so the next fix of this class is a minor. That second half
matters as much as the first — the only reason this unit cannot fix the defect properly is that
those structs were left open to literal construction.

## Required tests

1. **The bounded forward walk terminates and visits every row exactly once** via the new route, on
   the README's reproduction.
2. **The same backwards.**
3. **The old route's documented-but-wrong behaviour is pinned**, so a later change cannot alter it
   silently while it is still shipping. Assert what it actually does today and say in the test's name
   and comment that this is a defect being held still, not a property being endorsed.
4. **Duplicate anchors exist** — assert it directly on the reproduction. It is the premise of
   everything here; if a future change makes anchors unique, this test should fail loudly and tell
   the reader why.
5. **Default (`Positional`) alignment produces no duplicate anchors**, and the old and new routes
   agree there. That is the common case and must be seen to be unaffected.
6. **Failing-first** for 1 and 2.

## Acceptance criteria

1. The bounded reproduction fails on current `main` and passes after.
2. A correct additive route, agreed with me before implementation, with the four properties above.
3. Both old methods deprecated with a replacement named, behaviour unchanged.
4. `ChangeAnchor`'s doc no longer claims to identify a row; both deprecated methods say what they do.
5. The six tests, with failing-first where required.
6. The v4 repair recorded, including `#[non_exhaustive]` for the `view.rs` types.
7. `cargo public-api` shows additions and deprecations and **no removals or signature changes** —
   detached worktree, not a stash.
8. Gates green, rule 003, one scratch dir, deleted. Nothing committed.

## Prohibited shortcuts

- Do not "fix" `next_after` by matching the last occurrence, or by returning `None` on a duplicate.
  Both turn a visible hang into a wrong answer. This is the single most important line in this
  handoff.
- Do not add a field to `ChangeAnchor` or make it `#[non_exhaustive]`.
- Do not write a test that can hang. Bound every walk.
- Do not deprecate without naming the replacement in the attribute.

## Known risks

**0. No consumer we know of uses this API.** The consumer who prompted this work checked and
reported that `DiffView`, `ChangeAnchor`, `next_after`, `previous_before` and `output::view` appear
nowhere in their codebase, and asked us not to weigh the deprecation trade on their behalf. They
agree with the ranking anyway. Read that as freedom to get the replacement right rather than fast —
not as licence to leave the hang in place.

**1. The replacement route is a design decision, not a mechanical one.** `ChangeAnchor` exists so a
GUI can restore position across a re-diff. An index into a filtered list does not survive a re-diff.
If your proposal does not serve that use, say so plainly — "this does not replace anchors for
position restoration, and here is what would" is a better outcome than a route that quietly does not
do the job the old one claimed.

**2. Deprecating a method whose behaviour we are deliberately leaving broken will look odd to a
reader.** It is correct, and the doc is what carries the reasoning. Write it for someone who finds it
in a year with no context.

**3. Test 3 pins a defect.** Expect to feel wrong writing it. The alternative is that the behaviour
drifts during the deprecation window and nobody notices.

**4. This may be reachable in more places than `next_after`.** `first`, `row_count`, `sheet_rows`
and `ChangeAnchor`'s `Ord` all exist. **Sweep the file** for anything else that treats an anchor as an
identity — for example anything that sorts, dedupes, binary-searches or keys a map by it. Report what
you find even if it is nothing; "I checked these and they are fine" is the finding.

## Required evidence

Under `.git-exclude/review-request/row-space-02-the-anchor/evidence/`:

1. The bounded reproduction failing on `main`, with its message.
2. Your proposed route, and my agreement to it, before the implementation diff.
3. The four properties demonstrated, not asserted.
4. The sweep from *Known risks* 4: every place in `view.rs` that treats an anchor as an identity.
5. `cargo public-api` from a detached worktree.
6. Gate sweep, both manifests, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/row-space-02-the-anchor/README.md`, with:

- The replacement route, and honestly whether it serves position restoration across a re-diff.
- What else in `view.rs` treats an anchor as an identity.
- The exact new wording of `ChangeAnchor`'s doc.
- Whether you think the v4 repair is the right call or whether we should break it sooner. You are
  closer to the code than I am, and if holding a known-broken method through a deprecation window is
  the wrong trade, I would rather hear it now than after 3.4.0 ships. Note that the consumer, told
  the reasoning, independently agreed with the ranking — but they do not use the API, so their
  agreement is about the principle and not about the cost.
