# The row that vanishes

**Added 2026-10-05.** **Scoped by:** the architect. **One unit.**

`AlignmentMode::RowSignature { sample_columns: Some(cols) }` does not report changes in a row that
has no cell in any sampled column. The change is not mis-labelled or mis-placed — it is absent, with
no diagnostic, under an alignment summary that says `Exact`.

**Take `01` before every other open unit.** A diff tool that reports no difference where a cell
changed is the worst outcome this product has, and everything else currently queued is about
labelling, placing or describing differences we *do* report.

## How it was found

Not by a test. ForskScope, reviewing the confidence work, asked whether `RowSignature` can produce
the keyless-row condition — *"we do not use `RowKey`… you are better placed to say."* Reading
`compute_row_signatures` to answer them showed that it cannot produce the condition because it
loses the row instead.

**f130 fixed this exact defect on the `RowKey` path** and its comment states the requirement — *"What
it must not do is disappear"* — then lists the consequences, every one of which describes the
signature path today, including *"the summary would call the alignment exact."* One path was fixed.

## Related, and deliberately not merged into this unit

- `confidence-that-measures-counts/01` — `confidence` claiming more than the pairing supports.
  `Exact` on this input is a symptom; it resolves once the row stops vanishing, and the claim itself
  is that unit's.
- `confidence-that-measures-counts/02` — whether `RowSignature` ever warns about anything. **01 of
  this milestone covers the diagnostic for this case only**, because silence is part of this defect.

## Standing rules

Rule 002 (one scratch target dir per sweep, deleted after), rule 003 (sweep both manifests —
`./Cargo.toml` and `./fuzz/Cargo.toml`, the latter only via
`cargo check --manifest-path fuzz/Cargo.toml --bins`). Do not commit; review first.
