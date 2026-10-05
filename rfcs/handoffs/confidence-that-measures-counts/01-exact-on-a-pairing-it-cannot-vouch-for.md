# Handoff 01 — `Exact` on a pairing it cannot vouch for

**Unit:** confidence-that-measures-counts 01. **Added 2026-10-06.**
**Scoped by:** the architect. **Read `README.md` in this directory first** — it carries both
reproductions and the measured output.
**Semver: minor.** Settled — A-04 means this unit adds public API, so the question of whether the
behaviour change alone would have been a patch no longer arises.
**Revised 2026-10-06** after the consumer's reply: *Required implementation* 5 is new and is the
largest part of the unit.

## Purpose

`AlignmentSummary.confidence` reports `Exact` for pairings the engine cannot vouch for, and its
other values do not mean one thing each. Make it report what the pairing actually supports, and say
what it is reporting.

**A consumer will gate on this.** ForskScope's cascade keeps whichever alignment reports fewer
changed cells; reading our measurement they found that rule is **biased toward mis-pairings**, since
`RowSignature` pairs by similarity and a mis-pairing therefore produces a small diff by
construction. They are replacing the rule with one that gates on `confidence`. This unit is a
precondition for that being correct, not a refinement of it.

## The defect in one line

`confidence_for(n_matched, n_removed, n_inserted)` sees three counts, so "every row was matched" is
`Exact` whether the rows were matched by unique keys or because LCS had two identical keys and took
them in row order.

## Background you need

**The principle is already in the file.** f130 added a clamp at `src/align.rs:251` for keyless rows,
with this comment:

> *"`High` says the pairing is reliable apart from a few real insertions and removals. Rows placed
> by their content alone, or not at all, are neither — so a sheet with any keyless row is at most
> `Medium`, however many of them paired."*

Rows paired ambiguously among duplicate keys are **placed by their position alone**. That sentence
already covers them; the clamp was written for one case and not extended. The signature path
(`src/align.rs:342`) has no clamp at all.

**This is not hypothetical.** Both reproductions in the README report two changes that are purely
artefacts of an ambiguous pairing — the rows were reordered — while the summary says `Exact`.

## Change scope

- `src/align.rs` — `confidence_for` and/or its callers, and whatever has to be threaded to them.
- `src/model.rs` — `AlignmentSummary.confidence`'s field doc and `MatchConfidence`'s doc. **A-03 is
  in this unit**, see *Required implementation* 4.
- `tests/integration.rs` and/or `src/align/tests.rs` — the assertions.
- `rfcs/accepted/` or `rfcs/done/011-*` — wherever row alignment's confidence is specified. **Find
  it before you change behaviour**; if the RFC specifies the current rule, that is a finding and the
  RFC changes with the code, in the same unit.
- `src/options.rs` — `AlignmentMode::RowSignature`'s doc comment; see *Required implementation* 6.
- `CHANGELOG.md`.

## Non-change scope

- **Do not change which rows get paired.** This unit changes what we *claim* about a pairing, not
  the pairing. If you find yourself improving the matcher, stop — that is a different unit and a
  different risk.
- Do not add diagnostics to the signature path. Unit 02.
- Do not touch `CellDiff`, `RowPlacement` or `output::view`. Different milestone.
- Do not add a per-row confidence. It was considered and rejected; the README quotes the reasoning.

## Required implementation

**1. Establish what the matcher knows, and report it rather than guessing.** Before changing
anything, answer: at the point the summary is built, what does the code know about how each pairing
was reached? Duplicate keys are already detected (`find_duplicate_keys`, `src/align.rs:464`) — so
that fact is available. Whether colliding *signatures* are detected anywhere is a question I have
not answered for you; find out and report it, because unit 02 needs the same answer.

**2. Make `confidence` not claim more than the pairing supports.** The shape of the fix is yours to
propose, but the property is fixed: **a sheet containing any pairing the engine could not make
unambiguously must not report `Exact`.** The keyless clamp is the precedent for how to express
that — extend the principle rather than inventing a second mechanism beside it.

Two sub-questions to answer explicitly rather than by implication:

- **Is the right value `Medium`, or is `Low` what an ambiguous pairing deserves?** `Low` exists and
  nothing produces it as far as I can tell — check. If `Low` is unreachable today, that is itself a
  finding worth reporting, and this may be what it is for.
- **Does an ambiguity that could not have changed the outcome still lower confidence?** Two
  identical rows paired in either order give the same diff. Pairing them is ambiguous and harmless.
  A rule that lowers confidence there is noisy; a rule that tries to tell the difference is a
  content comparison the matcher does not do. **Pick the conservative, cheap answer and say you
  picked it** — do not build the clever one.

**3. Propose before implementing.** Send me the rule in one paragraph and the two sub-answers before
you change `src/align.rs`. This changes a value consumers may branch on and I would rather argue
about a paragraph than a diff.

**4. Fix A-03 — the undocumented promise.** `AlignmentSummary.confidence` has no field doc, and
`MatchConfidence`'s only doc says *"How confident the **sheet-matching** algorithm is about a
non-exact pairing"* — which is RFC-009 sheet matching, a different subject reusing the same type.
No variant has a doc comment. Give the field a doc saying what it claims **about rows**, and give
each variant one. **This is why the defect survived:** a value nothing documents is a value nothing
can contradict. Write the docs you would need to catch this reading them.

**5. Give the value a meaning a consumer can act on — which needs more than a doc comment.**

Measured (README, A-04), under `RowKey`:

| sheet | `inserted / removed / matched` | changes | `confidence` |
|---|---|---|---|
| every row keyed, every row matched | 0 / 0 / 2 | 0 | `Exact` |
| one keyless spacer row, otherwise identical | 0 / 0 / 2 | 0 | `Medium` |
| every row keyed, nothing matched | 2 / 2 / 0 | 8 | `Medium` |

**`Medium` is the catch-all for at least two unrelated claims** — "some rows were placed by content
alone" and "barely anything matched" — and your clamp adds a third. Rows two and three are the most
dissimilar outcomes in the table and share a value; rows one and two have identical counts and
different values, so `confidence` cannot be derived or checked from the summary it arrives in.

The consumer asked for *"what `Medium` means we should do"*, and said the middle is where their
cascade will spend its time. **That request cannot be met by writing prose**, because `Medium` has
no single meaning to write down. So the unit has to add the missing value.

**The precedent is in the same file.** Sheet matching ships `MatchConfidence` *and*
`SheetMatchReason`, *"set alongside"* it, whose variants carry actionable prose —
*"Nothing positive links the two sheets… treat it as the weakest kind of rename."* Row alignment
ships the confidence alone. **Build the row-alignment analogue:** something on `AlignmentSummary`
that says *why* the confidence is what it is. Keyless rows present; duplicate keys present;
signatures that could not be told apart; too few rows matched.

Open questions, which is why this is a proposal and not an instruction:

- **One value or several?** The causes co-occur — a sheet can have keyless rows *and* duplicate keys.
  A single enum then has to pick, which is how `Medium` got into this state. A set is honest and
  costs more surface. **Propose, with the reasoning.**
- **Does `confidence` survive as a separate field?** It may be that the reason subsumes it and
  `confidence` becomes derived; it may be that the ordinal is still what a consumer wants to
  threshold on. The consumer's words suggest both: *"`Exact` we will trust and `Low` we will not"*,
  so the ordering is load-bearing. **Do not remove the field** — that is breaking, and not yours.
- **Do not add variants to `MatchConfidence`.** It is shared with sheet matching, and new variants
  would appear there too. If you think that is nonetheless the right answer, say so and stop.

**This is the part of the unit I most want to see as a paragraph before a diff.** It is a public API
addition on the strength of one consumer's requirement, and I would rather over-discuss it than
ship a second value that needs a third.

**6. Warn about the hazard in `AlignmentMode::RowSignature`'s own documentation.** The consumer's
finding, which we have no claim on:

> *"Our tie-breaker is fewer changed cells wins. That is not neutral. It preferentially selects
> mis-pairings, because making the two sheets look more alike than they are is exactly what a
> mis-pairing does."*

It generalises past their cascade: **any rule that selects an alignment by the size of the diff it
produces is selecting for similarity matching's characteristic error.** And we created the
conditions — `RowSignature` is documented as *"Reduces cascades after row insertion/deletion"*, so
the obvious way to use two modes is to keep whichever reduced the cascade more. That is the
documented purpose being used as the selection criterion, and it is unsafe for a reason only visible
from inside the matcher.

Put it in `src/options.rs` on `AlignmentMode::RowSignature`: a mis-pairing produces a *smaller* diff
than the truth, so diff size is not a safe way to choose between alignments; gate on the confidence
and its reason instead. **I have told the consumer in writing that this is going there** — it is not
optional, and it does not belong only in a CHANGELOG entry.

## Required tests

1. **`RowKey` with duplicate keys does not report `Exact`** — the README's first reproduction,
   asserting the confidence value and the spurious-change count together, so the test records what
   the ambiguity cost.
2. **`RowSignature` with colliding sampled signatures does not report `Exact`** — the second
   reproduction. Note it currently emits no diagnostic; that stays unit 02's, and this test must not
   assert a diagnostic that does not yet exist.
3. **An unambiguous `RowKey` pairing still reports `Exact`.** The fix must not flatten everything to
   `Medium`; without this test, "never say `Exact`" would pass tests 1 and 2.
4. **The keyless clamp still holds** — f130's behaviour, unchanged. Find its existing test and
   confirm it still passes rather than writing a second one.
5. **Whatever you decide for sub-question 2**, asserted either way, with the decision in a comment.
6. **The three A-04 sheets, each asserting the reason as well as the confidence.** The keyless sheet
   and the nothing-matched sheet must be **distinguishable** afterwards — that is the measurable
   outcome of *Required implementation* 5, and the test that proves `Medium` is no longer a
   catch-all. A fix that leaves those two sheets indistinguishable has not done the unit.
7. **A sheet with two co-occurring causes** — keyless rows and duplicate keys together — asserting
   whatever your proposal says happens. If you chose a single value, this test records what it
   discards.
8. **Failing-first for 1, 2, 3 and 6.**

## Acceptance criteria

1. The rule, agreed with me in writing before implementation.
2. No sheet containing an ambiguous pairing reports `Exact`; unambiguous pairings still do.
3. Both sub-questions answered explicitly, including whether `Low` was previously unreachable.
4. `AlignmentSummary.confidence` and every `MatchConfidence` variant documented, for rows.
4b. The reason value from *Required implementation* 5 landed, proposed in writing first, with
   per-variant docs that say what a consumer should do — the `SheetMatchReason` standard, not a
   restatement of the variant name.
4c. The A-04 keyless sheet and nothing-matched sheet are distinguishable through the public API.
4d. `AlignmentMode::RowSignature`'s doc carries the selection hazard from *Required
   implementation* 6.
5. The governing RFC located, and reconciled with the code if it disagreed.
6. The six tests, with failing-first where required.
7. No golden moves. The corpus compares under default options and `alignment_summary` is `None` for
   `Positional` — so if one moves, stop and report.
8. `cargo public-api` diff empty, or showing only doc changes — detached worktree, not a stash.
9. Gates green, rule 003, one scratch dir, deleted. Nothing committed.

## Prohibited shortcuts

- Do not change the matcher. Only what we claim about it.
- Do not clamp everything to `Medium` and call it conservative. Test 3 exists for that.
- Do not add a per-row confidence field.
- Do not add variants to `MatchConfidence`; it is shared with sheet matching.
- Do not remove or retype `AlignmentSummary.confidence`. Breaking, and not this unit's.
- Do not answer the consumer's "what should I do at `Medium`" by writing a doc comment that asserts
  a single meaning. We measured that there is not one. Add the value that carries the claim.
- Do not leave the docs for later. A-03 is the reason A-01 survived; fixing the behaviour and
  leaving the undocumented field is fixing the instance and keeping the cause.

## Known risks

**1. This changes a value a consumer may branch on.** It is the point of the unit, and it is why the
semver call is the owner's. Write the CHANGELOG entry to say what now reports differently and why,
in terms a consumer can test against — not "improved confidence accuracy".

**2. `MatchConfidence` is shared with sheet matching (RFC-009).** Documenting it for rows must not
make the doc wrong for sheets. If the two subjects genuinely need different vocabularies, say so —
**do not split the type on your own initiative**, that is a public API decision and mine.

**3. `Low` may be unreachable.** If so, RFC-036 §5.4's obligation that every variant have something
asserting it is in scope by analogy. Report it; do not force a use for `Low` to tidy the enum.

**4. The ambiguity may be detectable in more places than duplicate keys.** Keyless rows paired by
content in row order (`src/align.rs:195`'s comment: *"Which of several identical rows pairs with
which has no…"*) are the same situation and are already clamped. Signatures may be a third. **Sweep
for every place the matcher pairs rows it cannot distinguish**, and report the list even if you only
act on two of them — the list is the finding.

## Required evidence

Under `.git-exclude/review-request/confidence-01-exact-on-a-guess/evidence/`:

1. The rule as proposed, and my reply agreeing to it.
2. What the matcher knows at summary-construction time, established from the code.
3. The sweep from *Known risks* 4: every place rows are paired indistinguishably.
4. Whether `Low` was reachable before this unit.
5. Each test with its failing-first mutation and `cmp`-verified restore.
6. The governing RFC, and whether it had to change.
7. `cargo public-api`, from a detached worktree. Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/confidence-01-exact-on-a-guess/README.md`, with:

- The rule, in one paragraph, as shipped.
- Both sub-answers, and which you would revisit if you had another day.
- The full list of places the matcher pairs rows it cannot distinguish.
- The new doc text for the field and all four variants, quoted, so I review the words.
- Whether the governing RFC specified the old rule. If it did, we shipped a documented defect, and I
  want that stated plainly rather than quietly corrected.
