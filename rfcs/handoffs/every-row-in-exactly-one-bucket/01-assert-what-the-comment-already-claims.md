# Handoff 01 — Assert what the comment already claims

**Unit:** every-row-in-exactly-one-bucket 01. **Added 2026-10-06.** **Scoped by:** the architect.
**Origin:** ForskScope's letter of 2026-10-05, §5. The idea is theirs, offered as *"the thing we
would write, not as advice about your codebase."*
**Semver:** no API change expected. Internal assertion plus tests.
**Release:** not scheduled. Take it after the units already queued.

## Why this exists

We have found the same defect twice: a row removed from a collection that later decides what gets
compared. f130 found it in `extract_row_keys`; `the-row-that-vanishes/01` found it in
`compute_row_signatures`, a release later. Both times the row disappeared from the comparison
entirely and the summary reported the alignment as `Exact`.

I owed a standing coverage rule in response — *a mode or option that no test constructs is not
covered whatever the count says* — and had already found it hard to state, because
`RowSignature { sample_columns: None }` **is** constructed by a test and **is** correct. The defect
lived in an argument to the mode, not the mode, so the rule had to reach option *values*.

The consumer's answer is better and I am taking it:

> *"You are not short of a mode constructed in a test. You are short of an **invariant**… It is
> checkable without enumerating option values at all, because it holds for every mode and every
> option value by construction. As an assertion inside the alignment path it would have failed on
> `f130` and failed again here, on the first test that happened to pass the offending option — no
> test needs to target the shape."*

**That is the whole argument.** A rule asks people to remember. An invariant does not.

## The invariant

Our own f130 comment states it in prose. Stated precisely, against `unmapped_rows`'s definition of
a row (*"a row with no cell at all is not in `cells` and is not a row of this sheet as far as any
comparison is concerned"*):

For a `RowMapping` produced for `(old_cells, new_cells)`:

1. **Old side, total and disjoint.** For every row `r` with at least one cell in `old_cells`,
   exactly one of: `r` is a key of `mapping.matched`, or `r` is in `mapping.removed`.
2. **New side, total and disjoint.** For every row `r` with at least one cell in `new_cells`,
   exactly one of: `r` is a value of `mapping.matched`, or `r` is in `mapping.inserted`.
3. **No row is invented.** Every key of `matched` and every entry of `removed` is a row of
   `old_cells`; every value of `matched` and every entry of `inserted` is a row of `new_cells`.
4. **`matched` is injective.** No two old rows map to the same new row — without this, clause 2's
   "exactly one" is unsatisfiable on the new side and the invariant is not even well-formed.

**Clause 4 is the one I am least sure of, and it is the first thing to establish.** LCS matching is
monotone and should be injective by construction, and `pair_identical_rows` should be too — but
"should be" is how both earlier instances of this defect survived. If it does not hold, that is a
finding about the matcher and this unit stops and reports rather than weakening the clause to fit.

## What it does not catch, stated up front

A **mis-pairing** satisfies every clause. Pair two unrelated rows that merely look alike and the
counts are perfect, the buckets are disjoint, nothing is invented and nothing is lost — and the diff
is wrong. That is the similarity bias, it is a different problem, and neither the consumer nor I can
think of an invariant that catches it.

**Do not stretch this unit to cover it.** An invariant that catches rows which *disappear* is worth
having on its own, and claiming it covers mis-pairing would be exactly the kind of overclaim this
quarter has been spent deleting.

## Change scope

- `src/align.rs` — the assertion, at every point a `RowMapping` is returned to the caller.
- `src/align/tests.rs` and/or a new test file — the property test.
- `fuzz/` — only if replaying the corpus needs a change; expect none.
- `docs/src/maintainers/` — where the invariant lives as prose for the next person.
- **No RFC change yet.** Whether this becomes a documented obligation is mine to write once the unit
  reports whether the invariant actually holds.

## Non-change scope

- **Do not change the matcher.** If the invariant fails anywhere, stop and report. A fix is a
  separate unit and probably a separate release.
- Do not add a release-time check that can panic in a consumer's production build. See *Required
  implementation* 2.
- Do not weaken a clause to make it pass.
- Do not take on mis-pairing.

## Required implementation

**1. Establish the four clauses before asserting anything.** Read the three paths that build a
`RowMapping` — `row_key_alignment`, `row_signature_alignment`, and whatever `Positional` does
(check: `compute_row_mapping` may return `None` rather than a mapping) — and say for each clause
whether it holds by construction, and why. **Clause 4 first.** Report the four answers; they are the
unit's primary output, not the assertion.

Also establish what happens on the `alignment_bound_exceeded` path. If alignment bails out and
returns no mapping, there is nothing to check and the assertion must not fire on a `None`.

**2. Assert it in debug builds, not in release.** `debug_assert!`-style: free in release, live in
every test and every debug build, which is the property that makes it catch a defect no test
targets. **It must not become a panic a consumer can trigger in a release build on hostile input** —
this is an internal consistency check, not input validation, and the threat model does not permit
turning malformed input into a panic.

Write the failure message so it names the offending rows and which clause failed. The person who
sees it will be debugging something else entirely.

**3. A property test over generated workbooks.** Vary: the alignment mode; `sample_columns` and key
columns including values no row populates; rows with cells only outside the sampled or key set;
duplicate keys; identical rows; empty sheets; one side empty. The generator matters more than the
assertion here — a property test that never produces a row outside the sampled set would pass
against the pre-fix code.

**Demonstrate it fails against the pre-fix code.** Check out `compute_row_signatures` as it was
before `the-row-that-vanishes/01`, or reproduce that state locally, and show the property test
failing. If it does not fail, the generator is not reaching the shape and the test is decorative.

**4. Replay the fuzz corpus with the assertion live.** `fuzz_open_xlsx_bytes` runs at `runs: 0`,
which replays the corpus without mutating; the corpus is the one body of input nobody designed. Copy
it first — libFuzzer writes discovered inputs back into the corpus directory, so each trial needs a
fresh copy (rule 002). Report whether anything fired.

**5. Write the invariant down as prose where a maintainer will find it**, with its limitation from
*What it does not catch*. The f130 comment stated it and the signature path still broke; prose alone
is not the fix, but prose plus an assertion is better than an assertion alone for whoever changes
the matcher next.

## Required tests

1. The property test, failing against the pre-fix signature path and passing now.
2. A unit test per clause, each constructed to violate exactly that clause via a deliberately
   corrupted `RowMapping`, confirming the assertion catches it. **If clause 4 cannot be violated
   without hand-building an invalid mapping, say so** — that is evidence it holds by construction,
   which is worth more than the test.
3. The existing suites, unchanged. `tests/rowkey_keyless_rows.rs` and
   `tests/rowsignature_unmapped_rows.rs` both exercise the rescue paths; both must still pass.

## Acceptance criteria

1. The four clauses, each established as holding or not, with the reasoning.
2. The bound-exceeded path accounted for.
3. A debug-only assertion at every return point of a `RowMapping`, with a message naming rows and
   clause.
4. The property test, demonstrated to fail against the pre-fix code.
5. The fuzz corpus replayed with the assertion live, result reported, corpus copied not mutated.
6. The invariant written down with its limitation.
7. Gates green, rule 003, one scratch dir, deleted. Nothing committed.
8. `cargo public-api` diff empty.

## Prohibited shortcuts

- Do not assert in release builds.
- Do not weaken clause 4 if it fails. Report it.
- Do not claim the invariant covers mis-pairing.
- Do not write a property test whose generator cannot produce a row outside the sampled set. That is
  the shape the whole unit exists for, and a generator that misses it would have passed against both
  known instances of the defect.

## Known risks

**1. The invariant may not hold today.** That is the most valuable possible outcome of this unit and
it stops the unit rather than failing it. Clause 4 under duplicate keys is where I would look first:
LCS over two sequences containing repeated keys is the case where injectivity is least obvious, and
it is the same input that produces `Exact` on an admittedly ambiguous pairing
(`confidence-that-measures-counts/01`).

**2. A debug assertion is reached by consumers' debug builds and by our fuzz targets.** That is
intended. It also means a false positive is expensive — hence establishing the clauses before
asserting them, rather than asserting and seeing what breaks.

**3. This does not discharge the coverage obligation it replaces.** It redirects the hard half. The
easy half — that a mode nobody constructs is not covered — is still mine to write, and the
consumer is adopting their own version of it. Do not record this unit as closing that.

## Required evidence

Under `.git-exclude/review-request/invariant-01-every-row-in-one-bucket/evidence/`:

1. The four clauses, established per mapping-producing path.
2. The bound-exceeded path's behaviour.
3. The property test failing against the pre-fix code, with its output.
4. The per-clause assertion tests, or the argument that a clause cannot be violated without an
   invalid mapping.
5. The fuzz replay, with the corpus copy created and deleted.
6. Gate sweep, both manifests, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/invariant-01-every-row-in-one-bucket/README.md`, with:

- The four answers, plainly, before anything about the implementation.
- Whether clause 4 holds, and how you established it.
- What the fuzz replay found.
- Whether you think the assertion belongs in release builds after all. I have said no on threat-model
  grounds; you are closer to the cost and if a release check is cheap and the panic risk is not real,
  I want the argument.
