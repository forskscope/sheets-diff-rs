# Handoffs — RFC-034, Build Assurance and Fixture Integrity

Companion execution documents for
[RFC-034](../../done/034-build-assurance-and-fixture-integrity.md).
Their lifecycle state is inherited from that RFC — they have no status of their
own, and they must not redefine it.

## Order

These are sequenced, not parallel. Each depends on the previous having merged.

| | Unit | Depends on |
|---|---|---|
| — | [Remove the `parallel` feature](../025-deterministic-parallel-execution/implementation-handoff.md) | — |
| 01 | [Fixture integrity](./01-fixture-integrity.md) | the `parallel` removal |
| 02 | [CI pipeline](./02-ci-pipeline.md) | 01 |

The `parallel` removal is governed by RFC-025 rather than RFC-034, but it heads
this queue: the feature matrix in unit 02 cannot go green while a feature that
does not compile is still declared.

Unit 01 must precede unit 02 because the `tree` job asserts a clean working tree,
which today's fixture harness violates on every run.

## Status

RFC-034 was **accepted on 2026-08-15** and is in `accepted/`. These handoffs are
live: implementation may begin, starting at the top of the queue above.

## Standing constraints for all units

- No change to comparison behaviour, output, ordering, or the public API.
- No library source may be edited to make a check pass. A red check is a finding
  to report.
- Every unit ships with the evidence its acceptance criteria name. "Tests pass"
  without the output is not evidence.
- Where a unit asks for a deliberate failure to be demonstrated, that
  demonstration is part of the deliverable. A guard never seen to fire has not
  been shown to work.
