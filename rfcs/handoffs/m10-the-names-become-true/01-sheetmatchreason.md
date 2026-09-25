# Handoff 01 — `SheetMatchReason` says a content check happened

**Governing.** [RFC-037 §3.1](../../accepted/037-v3-scope.md); RFC-006 (sheet
matching); RFC-033 (public model lexicon)
**Roadmap.** M10 — first unit
**Sequence.** Any time. Parallel with 02, 03, 04.

## Purpose

Every rename this crate reports carries a reason asserting that cell content was
compared. No cell content is ever compared. Make the value describe what happened.

## Background

Verified at review time, twice. `SheetMatchReason` has three variants and **one**
is ever constructed:

| Variant | Construction sites |
|---|---|
| `ExactName` | none — and **structurally impossible**: the enum appears only inside `SheetChange::Renamed` and `::RenamedAndMoved`, and a rename is not an exact-name match |
| `ContentSimilarity` | none — the matcher inspects no cell content anywhere |
| `IndexAndContent` | `matcher.rs:143`, `:151`, `:212` |

And the one that is constructed is wrong at all three sites:

- **`:212`**, inside a function literally named `index_match`, pairs sheets on
  `n.index == old.index` and nothing else.
- **`:143`**, `conservative_rename` with one unmatched sheet on each side and
  **equal** indices. Index only.
- **`:151`**, the same function with **unequal** indices — the two sheets are
  paired because they are the only ones left. **Neither half of the name is
  true**: no content compared, no index matched. This is a match by elimination
  and it is the worst of the three.

## Change scope

- `src/model.rs` — the enum
- `src/matcher.rs` — the three sites
- `tests/` — see Required tests
- `rfcs/done/033-*.md`, `rfcs/*/006-*.md` — the records
- `docs/` — anywhere match reasons are described
- `CHANGELOG.md`

## Non-change scope

- **Do not change the matching algorithm.** Not what pairs with what, not the
  confidence values, not the order the strategies run in. This unit changes what
  the engine *says it did*, never what it did. A corpus byte-comparison of
  `SheetChange` classifications must be identical apart from the `reason` field.
- **Do not make the matcher compare content.** That is a real proposal, it is
  RFC-006's, and it is not this milestone's — RFC-037 §4 forbids new features.
  If you form a view, write it in the review request.
- Do not touch `MatchConfidence`.

## Required implementation

1. **Remove `ExactName` and `ContentSimilarity`.**
2. **Replace `IndexAndContent` with two variants that are true**, one per
   observed cause:
   - the index matched — `:143` and `:212`;
   - the pair was the only one left, and the indices differed — `:151`.

   **Names are yours to propose**, with two constraints: each must be accurate
   about what the matcher checked, and neither may imply content was inspected.
   `IndexOnly` and `SoleRemainingPair` are the architect's placeholders, not a
   decision — if you have better, use it and say why in the review request.
3. **Each variant's doc comment says what the matcher actually did to reach it**,
   in one sentence a caller can act on.
4. **The enum's own doc comment** — currently "The reason a non-exact sheet pair
   was formed" — says what the matcher considers: name, index, and elimination.
   Not content.
5. **`matcher.rs:151` gets a comment** noting the pair is formed by elimination,
   because that site is the one a future reader will most easily misread.

## Required tests

- A rename matched on equal indices reports the index variant.
- A rename matched by elimination with **unequal** indices reports the
  elimination variant. This is the case that had no true value before; it must
  have its own test.
- **A corpus-wide classification check**: for every scenario, the `SheetChange`
  variant and its `confidence` are unchanged from 2.6.0, and only `reason`
  differs. This is the guard on Non-change scope, and it is the important one.

A removal demonstrates itself by failing to compile; a *replacement* does not.
Show the new variants failing first by a targeted change to the site that
produces them.

## Acceptance criteria

1. `ExactName` and `ContentSimilarity` are gone.
2. Two accurate variants exist, produced at the right sites, each documented.
3. The enum doc names what the matcher considers and says content is not.
4. `matcher.rs:151`'s elimination case is commented.
5. **Matching behaviour is unchanged** — corpus-wide classification check.
6. Records corrected: RFC-033's lexicon, RFC-006, and any doc page.
7. CHANGELOG under `### Changed` **and** `### Removed`, with the migration
   named: what a caller matching `IndexAndContent` should write instead.
8. Gates green.

## Prohibited shortcuts

- **Do not keep `IndexAndContent` as a deprecated alias.** This is the major;
  the whole point is that it goes. A deprecated lie is still a lie.
- Do not collapse the two causes into one variant because it is fewer lines.
  `:151` and `:212` are different facts about how much to trust the pairing, and
  a caller weighing a rename needs to tell them apart.
- Do not change the algorithm to make an existing name true.

## Compatibility constraints

**This is a breaking change and it is why v3 exists.** A caller matching
`SheetMatchReason::IndexAndContent` will not compile. That is the intended
signal: the arm they wrote was acting on a false premise, and a silent
fall-through to a wildcard would have been worse than a build error.

Serialised output changes: `"reason": "IndexAndContent"` becomes one of the new
names. Name that in the CHANGELOG — `--format json` shipped in 2.6.0, so there
is now a machine-readable surface carrying this value.

## Known risks

- **The goldens carry `reason` for renamed sheets.** `renamed_sheet` at minimum
  will move. Check which, and show that each moved golden differs **only** in
  `reason`.
- `docs/src/semantics.md` executes. If it asserts on a match reason it will
  move — that is the documentation working.

## Required evidence

- Per-variant construction-site search, command shown, before and after
- The corpus-wide classification check (§Required tests)
- Which goldens moved and the proof each differs only in `reason`
- Pre-fix failure transcripts for the new variants
- CI run link

## Review request format

Per development policy §9.2. Additionally: name the variants you chose and why,
and state whether you think the matcher *should* compare content — that view
belongs in RFC-006's next revision and I would rather have it from whoever just
read the code.
