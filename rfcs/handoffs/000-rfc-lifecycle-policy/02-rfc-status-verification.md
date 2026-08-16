# Handoff 02 — Verify the thirty unverified RFC statuses

**Governing RFC.** [RFC 000](../../done/000-rfc-lifecycle-policy.md)
**Roadmap.** M3, track D
**Sequence.** After unit 01 — RFC-033 is part of what several are verified against.

## Purpose

Turn thirty statuses that disclose their own uncertainty into statuses that are
true.

## Background

Thirty RFCs in `done/` read:

> **Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually
> re-verified against the implementation.

That caveat was honest when written, in a restoration that had to choose between
guessing and disclosing. It has stood for a month, and **at least three of the
claims are positively wrong.** `rfcs/README.md` already records that 014 ships
`Serialize` without `Deserialize`, 020's `CellNumberFormat` is always `None`, and
021/023 surface only diagnostics with their structured types permanently empty.
All three sit in `done/` saying *Implemented*.

RFC 000 makes the folder and the Status field the source of truth for state. A
permanent "not verified" note is not a state; it is a deferred check wearing a
state's clothing.

## Change scope

`rfcs/done/*.md` Status fields; moves between `done/` and `accepted/` where a
status proves wrong; `rfcs/README.md`.

## Non-change scope

Do **not** change `src/`, behaviour, or the public API. If an RFC is
unimplemented, the RFC moves — the code does not. Fixing the gap is M4's
business or later, and confusing the two would let this unit quietly become an
implementation milestone.

## Required implementation

For each of the thirty, read the RFC's acceptance criteria and check them
against the code. Assign one of:

- **Implemented** — criteria met. Status states the version and drops the caveat.
- **Partially implemented** — the main design decision shipped, parts did not.
  RFC 000 §"Granularity of transitions" permits this explicitly: it stays in
  `done/` with the deferred work named in its Status. Name what is missing;
  "partially" without a list is the same defect in a new form.
- **Not implemented** — the main decision did not ship. It **moves to
  `accepted/`**, since the design stands and the work does not. Update
  `rfcs/README.md` in the same commit, per RFC 000's link rule.

Start with 014, 020, 021, 023, 022 and 025 — the six with known findings — so
the method is exercised where the answer is already partly known before it is
applied to the unknown twenty-four.

Also in scope, noticed while checking: RFC 000's illustrative Status examples
cite "RFC 042" and "RFC 035". **035 is now a real RFC here**, so the example
reads as a live cross-reference. Disambiguate it.

## Required tests

None; documentation only. Nothing under `src/` may change, so the suite must be
untouched — confirm with `git status`.

## Acceptance criteria

1. All thirty carry a status stating a verified conclusion. Zero instances of
   "not individually re-verified" remain.
2. Every "Partially implemented" names what is missing.
3. Every RFC moved out of `done/` has `rfcs/README.md` updated in the same
   commit, and inbound links fixed.
4. RFC 000's example numbers no longer collide with real RFCs.
5. `git status` shows no change under `src/`, `tests/` or `benches/`.
6. Findings — RFCs whose design never shipped — are listed for M4's queue.

## Prohibited shortcuts

- Do not verify by reading the RFC alone. The claim is about the code; check the
  code.
- Do not mark something Implemented because it mostly is. That is the caveat
  again, in shorter words.
- Do not implement anything to make a status true.
- Do not batch thirty status edits into one unreviewable commit. Group them so a
  reviewer can follow the reasoning per RFC.

## Known risks

- More than three may prove wrong. That is the point; report the total honestly
  even if it is unflattering to the M1 restoration, which is mine.
- Some acceptance criteria may be ambiguous enough that verification is a
  judgement call. Say so per RFC rather than forcing a verdict.

## Required evidence

- Per RFC: the criteria checked, what was found, the assigned status
- The before/after count of unverified statuses
- `git status` showing no source change
- The findings list for M4

## Review request format

Per development policy §9.2, plus the findings list and the count of statuses
that proved wrong.
