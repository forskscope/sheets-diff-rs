# Handoff 07 — The last four diagnostic codes no scenario reaches

**Milestone:** M9, unit 07. **Added 2026-10-01**, from unit 05's §4.
**Scoped by:** the architect, under unit 05's "report the list and I will scope the rest."
**Governed by:** RFC-036 §5.4's new last bullet, adopted at unit 05's review — these four are
**deferred by name** there, and this unit is what un-defers them.

## Purpose

Four `DiagnosticKind` variants have no scenario whose assertion fires on them. Close the ones that
can be closed; defer the rest **with a reason**, which §5.4 now permits and requires.

## Background

Unit 05 closed `formula_unavailable`, `missing_alignment_key` and `alignment_bound_exceeded` and
found these four while doing it:

| Code | Unit 05's read |
|---|---|
| `ambiguous_sheet_match` | **A shape question, not an options one** — the default matching mode already permits an ambiguous rename; no scenario has that shape |
| `duplicate_alignment_key` | Likely needs the dedicated-options pattern (a non-default `AlignmentMode`), like rows 3/4/13/14 |
| `unsupported_workbook_metadata` | Not investigated — shape or options unknown |
| `defined_name_scope_unknown` | Not investigated — shape or options unknown |

**Two of the four were not investigated at all.** Unit 05 said so rather than guessing, which is why
this handoff does not pretend to know what they need. **Establish that first.**

## Change scope

- `examples/gen-fixtures.rs` and `tests/fixtures/generated/` — new scenarios.
- `tests/integration.rs` — the dedicated assertions.
- `rfcs/accepted/036-*.md` §5.4 and `tests/fixtures/corpus/README.md` — the matrix and the deferred
  list.
- The three hardcoded corpus-count assertions unit 05 had to bump
  (`tests/diagnostic_codes.rs`, `tests/json_infallible.rs`,
  `tests/sheet_match_classification.rs`'s baseline table) — see *Known risks* 2.

## Non-change scope

- **Nothing under `src/`** except temporarily, for the failing-first demonstrations.
- **Do not make a diagnostic easier to produce in order to cover it.** If a variant is unreachable
  because the engine cannot construct it from any input, that is a **finding about the variant** —
  report it, and it becomes a question about whether the variant should exist, which is mine.
- Do not add a scenario that produces the code incidentally. §5.4 now says, in the RFC, that this
  does not count.

## Required implementation

**1. Establish what each of the four needs** — a shape under default options, the
dedicated-options pattern, or neither — before writing any fixture. Report the four answers.

**2. Close the ones that can be closed**, following unit 05's pattern exactly: a committed fixture
picked up by the default-options golden like any other, plus an assertion in `tests/integration.rs`
that fires **on the diagnostic**, asserting its payload fields and not merely `code()`.

**3. Defer what cannot be closed, by name and with a reason, in RFC-036 §5.4.** Replace the
"pending M9 unit 07" wording there with the outcome. **A deferral with a reason is a success
condition of this unit, not a failure** — §5.4's own second bullet says an undocumented gap is a
defect and a documented one is a decision. What is not acceptable is a fixture that pretends.

**4. `ambiguous_sheet_match` deserves particular care.** `SheetChange::Ambiguous` and the
ambiguity diagnostic are the conservative behaviour RFC-009 §8 specifies — *"ambiguous matches must
not be hidden"* — and M10 unit 01 found that RFC's §6 described a matcher that was never built.
**Check what the engine actually does with two plausible rename candidates before building a
fixture for it.** If the behaviour and RFC-009 §8 disagree, stop and report: that is a defect, not
a coverage gap, and it would be the seventh instance of this quarter's pattern.

## Required tests

- One assertion per closed code, on the diagnostic's payload.
- **Failing-first per code**, by short-circuiting the specific emitting statement — unit 05's
  `if false && …` technique, one line at a time, `cmp`-verified between each, never two edits live
  at once.
- The existing `every_code_in_the_table_is_producible_and_nothing_else_is` must still pass; it may
  get easier.

## Acceptance criteria

1. The four answers from §1, reported.
2. Each closable code closed with a payload-level assertion and a failing-first demonstration.
3. Each unclosable code deferred in RFC-036 §5.4 **with its reason**.
4. No existing golden moves. If one does, report it.
5. The corpus-count assertions updated if the count changed.
6. Gates as always, plus rule 003. `git diff src/` empty at the end.

## Prohibited shortcuts

- Do not cover a code by asserting a scenario exists. §5.4 now forbids it in writing.
- Do not relax a precondition in `src/` to make a code reachable.
- Do not leave a code silently uncovered. Defer it in the RFC or close it.

## Known risks

**1. One or more may be genuinely unreachable.** That is a real outcome and §5.4 accommodates it.
Reaching for a contrived fixture to avoid writing "deferred" would be the wrong instinct.

**2. Three hardcoded corpus counts.** Unit 05 had to bump 19→22 in two files and add rows to
`sheet_match_classification.rs`'s baseline table. Any new scenario moves them again. Note unit 05's
own care there: those baseline rows are documented as **not** sourced from the 2.6.0 binary, since
the scenarios postdate it — keep that note true for any rows you add.

**3. `ambiguous_sheet_match` may be a defect rather than a gap.** See *Required implementation* 4.

## Required evidence

Under `.git-exclude/review-request/m9-07-the-last-four-codes/evidence/`:

1. What each of the four needs, established rather than assumed.
2. Per closed code: the golden or dedicated assertion showing the diagnostic, and the failing-first
   demonstration with its `cmp` restore.
3. Per deferred code: why, concretely.
4. For `ambiguous_sheet_match`: what the engine does with two plausible candidates, and whether it
   matches RFC-009 §8.
5. Corpus: which goldens moved (expect only new ones). Gate sweep, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/m9-07-the-last-four-codes/README.md`, with:

- The four answers, and which you closed.
- Each deferral's reason, written to be read in a year.
- Whether `ambiguous_sheet_match`'s behaviour matches RFC-009 §8.
- Whether RFC-036 §5.4's new bullet was workable as written. I adopted it from unit 05's proposal
  with one tightening; you are the first to work under it, and if it is wrong I would rather hear it
  from the first unit than the fifth.
