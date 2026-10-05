# Every row in exactly one bucket

**Added 2026-10-06.** **One unit.** **Not scheduled** — take it after the units already queued.

Twice now a row has been removed from a collection that later decides what gets compared, and
vanished from the comparison with the summary reporting `Exact`: f130 in `extract_row_keys`, and
`the-row-that-vanishes/01` in `compute_row_signatures` a release later.

The response is not another rule. It is the invariant our own f130 comment already states in prose —
*every row present in either sheet is in exactly one of `matched`, `removed` or `inserted`* —
asserted in debug builds at every point a `RowMapping` is returned, plus a property test whose
generator can produce the offending shape.

**The idea is ForskScope's**, from their letter of 2026-10-05 §5, offered with its limitation
named: it catches rows that **disappear** and says nothing about rows **mis-paired**, which is a
different problem that no invariant either of us can think of would catch.

**The unit's primary output is not the assertion.** It is the answer to whether the invariant holds
today — four clauses, three mapping-producing paths. If it does not hold, the unit stops and reports,
and the fix is a separate unit.

## Standing rules

Rule 002 (one scratch target dir per sweep, deleted after — and the fuzz corpus is copied, never
mutated in place), rule 003 (sweep both manifests). Do not commit; review first.
