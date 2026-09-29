# Quarantined seeds

**Seeds that are correct, wanted, and temporarily out of the corpus libFuzzer runs in CI, because
they seed a crash we have not fixed yet.** They are still committed, still decoded and still
exercised in-process by `tests/fuzz_corpus_reaches_reader.rs`, which walks this directory as well as
`fuzz/corpus/`. Nothing here is broken or deprecated.

**This directory should be empty. It is a debt marker, not a category.**

## `fuzz_open_xlsx_bytes/paired_encrypted`

Quarantined 2026-09-29 (M9 unit 01 review). RFC-028 §6's "password-protected" category, built from
`tests/fixtures/corrupt/encrypted.xlsx`.

The seed itself is fine: fed to `compare_bytes` it returns `EncryptedWorkbook`, which is what it is
for, and the guard test asserts exactly that. **Mutations of it are not.** An encrypted `.xlsx` is a
CFB container, and `calamine`'s CFB header parser takes a sector-count field straight to
`Vec::with_capacity` without checking it against the file's length — so a few flipped bytes give a
multi-gigabyte allocation and an out-of-memory abort. Measured at CI's own budget
(`-runs=20000`, five different `-seed` values, corpus copied fresh each run):

| Corpus | Result |
|---|---|
| With this seed | **out-of-memory, 5 of 5** |
| Without it | **clean, 5 of 5** |

It is the only seed that does this; the other eleven reach the sheet reader and run clean.

**Return it to `fuzz/corpus/fuzz_open_xlsx_bytes/` in the same change that closes the defect** — see
`docs/src/maintainers/threat-model.md`, *Opening a workbook: two inputs that defeat every bound*, and
RFC-028 §7. Deleting this directory is then part of that change.
