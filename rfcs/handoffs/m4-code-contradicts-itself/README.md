# Handoffs — M4: the code contradicts itself

Three units closing four findings, each a case of **the code stating something
that is not true**. Governed by the RFCs each finding belongs to — RFC-007,
RFC-013, RFC-021, RFC-024/027 — all in `done/`, since these are defects against
decisions already made, not new design.

This directory is keyed to the milestone rather than to one RFC number, because
the four findings belong to four different RFCs and no single one governs the
set. That is a deliberate departure from RFC 000's `handoffs/NNN-slug/`
convention, noted here rather than done silently.

## Queue

| | Unit | Governing RFC | Release impact |
|---|---|---|---|
| 01 | [Two doc-only truths](./01-doc-only-truths.md) | RFC-021, RFC-007 | none — comments and docs |
| 02 | [`cells_compared`](./02-cells-compared.md) | RFC-024, RFC-027 | a public metric's value changes |
| 03 | [Exit code 3](./03-exit-code-3.md) | RFC-013 | **CLI contract change** |

Any order. 01 is the cheapest and removes two false statements immediately.

## Why these four are one milestone

Not because they are small. Because they are the same defect:

- CHANGELOG 2.2.0 documented a `parallel` feature that had never compiled.
- CHANGELOG 2.2.3 claimed `cells_compared` was fixed; it was not, and still is not.
- `meta.rs`'s comments describe a `WorkbookMetadataMode` default; the type does
  not exist.
- RFC-013 specifies exit code 3; nothing emits it.

Each sounds like a fact because it sits beside code. This project has now been
caught by that pattern three times, which is why the set is worth clearing
together rather than dispersing into other milestones.

## Standing constraints

- **No new public API.** Building `WorkbookMetadataMode` is out of scope — unit
  01 corrects the comments that claim it exists. Whether to build it is a later
  decision.
- **Every fix arrives with an assertion**, per RFC-036 §5.3, or the review
  request says why none is needed.
- **The fixture corpus must not move.** These are behaviour fixes outside the
  comparison engine; if a golden moves, that is a finding — stop and report.
- Gates as always: fmt, clippy `-D warnings`, `deny`, MSRV, the full matrix.
