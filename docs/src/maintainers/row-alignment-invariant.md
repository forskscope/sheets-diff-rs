# The row-alignment invariant

Every row of a sheet that has a cell is in exactly one place in the alignment's result: paired with a row
of the other sheet, or reported as removed (old side) or inserted (new side). A row can disappear from the
comparison only by breaking that. Two rows have done it: `RowKey`'s keyless rows, through 3.0.0 (f130), and
`RowSignature`'s rows with no sampled cell, through 3.3.0 (the-row-that-vanishes/01). In both, the summary
said `Exact`.

The invariant is checked in debug builds at the single place every mapping leaves the alignment code,
`compute_row_mapping` (`src/align.rs`). A violation panics with the offending rows and the clause that failed.
In release builds the check is compiled out, so it can never be a panic a caller can reach.

## The four clauses

For a mapping produced for `(old, new)`:

1. **Old side, total and disjoint.** Every old row with a cell is in exactly one of: a key of `matched`, or `removed`.
2. **New side, total and disjoint.** Every new row with a cell is in exactly one of: a value of `matched`, or `inserted`.
3. **No row is invented.** Every key of `matched` and every entry of `removed` is an old row with a cell;
   every value of `matched` and every entry of `inserted` is a new row with a cell.
4. **`matched` is injective.** No two old rows are matched to the same new row.

A row "with a cell" is what `unmapped_rows` means by a row: a row with no cell at all is not in the sheet as far as
any comparison is concerned.

## Why each clause holds, by path

- **LCS (`lcs_match`).** The traceback advances both indices on every match and advances one of them otherwise,
  so each old and each new index is used at most once (clause 4), and each row of the sequence is either used
  or left over (clauses 1 and 2 for the sequence's rows). A tie between two alignments always leaves a row
  unmatched, so ties cannot make a row count twice.
- **Rescue (`pair_identical_rows`, over `unmapped_rows`).** `unmapped_rows` is exactly the rows with cells that the key or
  signature map did not cover, so the rescue's rows are disjoint from the LCS's rows on both sides (clauses 1
  and 2 across the two). Each row is paired at most once, because it is popped from a queue once (clause 4 for the
  rescued pairs). Leftovers are reported as removed or inserted (clauses 1 and 2).
- **Positional.** `compute_row_mapping` returns no mapping, so there is nothing to check.
- **Bound exceeded.** `compute_row_mapping` returns no mapping when the row product exceeds
  `max_alignment_product`, so again nothing is checked.

Clause 4 cannot be violated by the engine on any input we have found, and the test that hand-builds a
violation (`clause_4_catches_two_old_rows_matched_to_one_new_row`) shows the checker would catch it if that
stopped being true.

## What the invariant does not catch

**A mis-pairing satisfies all four clauses.** Pair two unrelated rows that look alike and the counts are right,
every row is in one bucket, nothing is invented, and the diff is wrong. That is the similarity bias
(`RowSignature` pairs by similarity, so a mis-pairing produces a small diff). It is a different problem, and no
invariant over the mapping's shape detects it. Do not read a passing check as a statement about pairing quality;
that is what `confidence` is for, and it is a separate question.

## Evidence it catches what it was built for

- The property test (`the_invariant_holds_for_generated_sheets_under_every_mode`, `src/align/tests.rs`) generates
  sheets with empty and one-sided cases, identical rows, duplicate keys, and rows with cells only outside the
  sampled or key columns, and checks more than 5,000 mappings. It passes today.
- With the signature rescue removed, it fails with `clause 1 (old side, total): old row 2 has cells but is in neither
  matched nor removed`. With the keyless rescue removed, it fails the same way. So the assertion would have caught
  both defects.
- The fuzz corpus of `fuzz_self_comparison`, the target that reaches the alignment modes, replays clean with
  debug assertions on.
