# Handoff 00 — `hardened()` promises a guarantee we do not give

**Governing.** R1, from the 2.5.1 release-preparation sweep; RFC-035 §5.3;
RFC-016
**Roadmap.** M8 — **numbered 00 because it is taken before unit 01**, not
after unit 05. It is small, doc-only, and its wording is most misleading right
now, in the weeks after 2.5.1.
**Sequence.** First in M8. Independent of every other unit.

## Purpose

A public rustdoc makes a security guarantee that this crate's own threat model
contradicts in two places. Make the claim true.

## Background

`src/options.rs`, on `Limits::hardened()`:

> a caller who opts into it accepts that a very large but legitimate workbook may
> hit a limit, **in exchange for a guarantee that no workbook — hostile or merely
> huge — can demand unbounded time or memory.**

`docs/src/api-guide.md:131` echoes it: *"`Limits::hardened()` … bounds every
dimension — linear and superlinear alike — for exactly that case: a workbook that
arrived from somewhere you don't control."*

**The threat model names two exceptions to exactly that guarantee:**

- *Zip container:* a zip bomb within the size bound "is bounded by `zip`'s own
  decompression behaviour, **which this crate does not independently cap**".
- *Sheet reading:* styled blank cell records "cost **time**, not memory, are **not
  counted by `max_cells_read`**, and are limited only by `max_input_bytes` and by
  cancellation latency".

### The precise shape of the error, because it matters for the fix

`hardened()` **does** set all six fields — `max_sheets`, `max_cells_read`,
`max_cells_compared`, `max_diffs_returned`, `max_alignment_product`,
`max_input_bytes`. So "bounds every dimension" is **true about the struct**.

The guarantee sentence is quantified over *workbooks*, not over dimensions: it
says no workbook can demand unbounded time or memory. That is false, because
there are resource paths which are not dimensions we defined.

**This is the second time this function has overpromised in this exact way.**
M4's F-E was *"`Limits::hardened()` presets everything for untrusted input"* —
true about the fields, false about the protection, because `max_cells_compared`
bounded diffs rather than coordinates. We corrected that sentence and left this
one, which makes the same move one clause further out.

### Why it is worse now than it was last month

2.5.1's release notes say the sheet-read hole is fixed. A reader who takes
"fixed" to mean "and now the guarantee holds" is drawing exactly the inference
this sentence invites. The defect is pre-existing — it has been wrong since
2.3.0 — but its capacity to mislead is highest immediately after a release that
fixed a resource defect.

It did not block 2.5.1, because 2.5.1 fixed a denial of service and delaying that
for a doc correction would have been the wrong trade. It leads M8 instead.

## Change scope

`src/options.rs` (the `hardened()` rustdoc), `docs/src/api-guide.md`,
`CHANGELOG.md`.

## Non-change scope

- **Do not change what `hardened()` sets.** The values are RFC-035 §5.3's and
  they are not the problem. If you think a value is wrong, that is a finding —
  stop and report.
- **Do not add a new bound** to make the guarantee true. Capping decompression
  or counting blank records is real work with real trade-offs; it is not a
  doc unit, and proposing it is M9's or a later milestone's.
- Nothing under `tests/`; the corpus must not move.

## Required implementation

1. **Replace the guarantee with what is actually true.** `hardened()` bounds
   every dimension `Limits` defines, and those bounds are checked before the
   work they gate. Say that, and say plainly what it does **not** cover —
   decompression ratio within the size bound, and time spent on cell records
   that carry no value.

   **Do not merely soften it** to "helps protect against" or "substantially
   bounds". A vague claim is not more honest than a precise wrong one; it is
   less useful and it fails the same reader. Name the two gaps.

2. **Link the threat model's two residual risks** rather than restating them.
   Two documents describing the same exception in their own words is how they
   drift apart — which is the failure this whole milestone is about.

3. **Correct `api-guide.md:131` to match.** It says "bounds every dimension",
   which is true; the problem is that it sits in a paragraph recommending
   `hardened()` "for exactly that case: a workbook that arrived from somewhere
   you don't control", with nothing saying what remains uncovered. A reader
   choosing a posture for untrusted input needs both halves.

4. **CHANGELOG** under `### Documentation`: the guarantee was wrong, it has been
   wrong since 2.3.0, and here is what is actually bounded. **State the affected
   range** — someone who read it and built a threat model on it deserves to know
   it was not a fresh mistake.

## Required tests

None — doc comments and prose. But:

1. `cargo doc --all-features --no-deps` produces no new warnings.
2. Doctests green on stable and 1.88 (`api-guide.md` is in the harness).
3. Corpus byte-identical, checked rather than inferred.

## Acceptance criteria

1. `hardened()`'s rustdoc no longer claims a guarantee over all workbooks, and
   names the two uncovered paths specifically.
2. It does not achieve this by vagueness — a reader can tell what is and is not
   bounded.
3. The threat model's residual risks are linked, not restated.
4. `api-guide.md`'s `hardened()` recommendation carries the same caveat.
5. CHANGELOG states the correction and that it applies from 2.3.0.
6. `hardened()`'s values are unchanged; nothing under `tests/`; corpus identical.
7. `cargo doc` clean; doctests green on both toolchains.
8. Gates green, full matrix.

## Prohibited shortcuts

- **Do not delete the sentence.** A caller who built a posture on it is owed the
  correction, which is the standard the API guide's `compare_bytes` correction
  set in 2.5.1 — follow its shape.
- Do not write "no known way to exceed these bounds". We know two.
- Do not describe the blank-record path as theoretical. The 2.5.1 work measured
  a fixture built on exactly that shape.

## Known risks

- **The honest version is less reassuring**, and that is the point. If the
  result reads as though `hardened()` is now less useful, say so in the review
  request rather than adjusting the wording until it sounds better — the
  question of whether the gaps should be *closed* is a real one and it is not
  yours to pre-empt.
- `api-guide.md` is in the doctest harness; a botched fence fails CI, which is
  the harness working.

## Required evidence

- The diff
- `cargo doc` output
- Doctests on both toolchains
- Corpus byte-comparison
- CI run link

## Review request format

Per development policy §9.2, plus a plain statement of what `hardened()` does
and does not bound after this change — in your own words, not the text you
wrote, so the two can be compared.
