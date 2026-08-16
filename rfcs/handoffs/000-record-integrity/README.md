# Handoffs — record integrity (M3 track B)

Repair work on the project's own record, governed by
[RFC 000](../../done/000-rfc-lifecycle-policy.md) — the RFC that defines what a
correct record looks like.

These units do not change the lifecycle policy. They fix two places where the
record violates it.

## Queue

| | Unit | Depends on |
|---|---|---|
| 01 | [Reconstruct RFC-033](./01-rfc-033-reconstruction.md) | — |
| 02 | [Verify the 30 unverified RFC statuses](./02-rfc-status-verification.md) | 01 |

02 depends on 01 because RFC-033 is part of what several of the thirty would be
verified against.

Both run **after** track A, per the owner's sequencing decision.

## Why this track exists

Two defects in the project's own record, both self-inflicted during the M1
restoration:

- **RFC-033 does not exist**, yet `src/` cites it as normative at **20 sites**
  across seven files, including as the canonical lexicon for the public model.
- **Thirty RFCs in `done/` say "Implemented … not individually re-verified"**,
  and at least three of those claims are positively wrong — `rfcs/README.md`
  already records that 014 ships `Serialize` without `Deserialize`, 020's
  `CellNumberFormat` is always `None`, and 021/023 surface only diagnostics with
  their structured types permanently empty.

A permanent "not verified" caveat is not a state. It is a deferred check wearing
a state's clothing, and RFC 000 makes the folder and Status field the source of
truth.
