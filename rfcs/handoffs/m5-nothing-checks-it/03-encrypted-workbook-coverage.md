# Handoff 03 — Encrypted workbooks

**Governing RFC.** RFC-032 (unsupported, corrupt and encrypted workbook
handling)
**Roadmap.** M5
**Sequence.** Any. The largest of the three — the fixture does not exist.

## Purpose

Give `SheetsDiffError::EncryptedWorkbook` its first test.

## Background

The detection exists and is deliberate. `src/error.rs`:

```rust
pub(crate) fn from_open_error(side: Side, source: SourceDescription, e: calamine::XlsxError)
    -> SheetsDiffError
{
    if matches!(e, calamine::XlsxError::Password) {
        SheetsDiffError::EncryptedWorkbook { side }
    } else {
        SheetsDiffError::open_workbook(side, source, e)
    }
}
```

It has a dedicated error variant, a distinct message, and as of M4 unit 03 a
dedicated CLI exit code (3). **Nothing tests any of it.** No fixture in the tree
is encrypted, and `grep -ri encrypted tests/` returns nothing.

So a change to calamine's detection, to our interception, or to the error's
classification would be caught by no check we run. An encrypted workbook
silently reported as "not a valid xlsx" is a bad diagnosis for a case that is
entirely ordinary in the field — a user picks a protected file out of a folder —
and it is exactly the kind of regression that surfaces as a support question
rather than a test failure.

### Why the fixture is the work

`rust_xlsxwriter` cannot produce an encrypted workbook, so the corpus generator
cannot make one. But **you do not need real encryption.** Calamine's detection,
in `xlsx/mod.rs`, is:

```rust
fn check_for_password_protected<RS: Read + Seek>(reader: &mut RS) -> Result<(), XlsxError> {
    let offset_end = reader.seek(SeekFrom::End(0))? as usize;
    reader.seek(SeekFrom::Start(0))?;
    if let Ok(cfb) = crate::cfb::Cfb::new(reader, offset_end) {
        if cfb.has_directory("EncryptedPackage") {
            return Err(XlsxError::Password);
        }
    }
    Ok(())
}
```

It parses the file as a **CFB (Compound File Binary) container** and checks for a
directory entry named `EncryptedPackage`. That is the entire test. A minimal CFB
with that entry and no real encrypted payload triggers it.

That is a fact about calamine 0.36.1's internals, so **treat it as a starting
point you verify, not as a specification.** If it has changed, report that
rather than working around it — it would mean our detection rests on something
we have not been tracking.

## Change scope

`tests/`, a new fixture under `tests/fixtures/`, and `CHANGELOG.md`.
`examples/gen-fixtures.rs` and `Cargo.toml` **only if** you take the
generated-fixture route — see below.

## Non-change scope

- **Nothing under `src/`.** If detection turns out to be broken, that is a
  finding — stop and report.
- **No existing fixture may change.** This adds one. If a golden moves, stop.
- Do not add support for *reading* encrypted workbooks. Detecting and refusing
  is the designed behaviour (RFC-032).

## Required implementation

1. **A fixture that calamine reports as password-protected.** Two routes; pick
   one and justify it:

   - **Commit the bytes.** A minimal CFB with an `EncryptedPackage` entry, built
     once and committed, alongside `not_a_zip.xlsx` — which is the precedent:
     a small hand-made file the generator does not produce. Its provenance must
     be documented, since a committed binary nobody can regenerate is its own
     kind of unchecked thing.
   - **Generate it.** Extend `examples/gen-fixtures.rs`, which likely needs the
     `cfb` crate as a **dev-dependency**. That is a supply-chain decision under
     RFC-026 and RFC-035 — `deny.toml` governs what may enter the tree, and a
     dev-dependency is still a dependency. **Argue for it; do not assume it.**

   The first is smaller and adds no dependency. The second is reproducible. I
   have a mild preference for the first on those grounds, but the argument is
   yours to make.

2. **Document what the fixture is** — that it is a container shaped like an
   encrypted workbook, not an encrypted workbook, and that this is sufficient
   because detection is structural. A future reader must not mistake it for a
   real encrypted file and draw conclusions about decryption.

3. **Confirm the detection path end to end**, not just the classifier: opening
   it produces `EncryptedWorkbook`, not `OpenWorkbook { kind: NotXlsx }`.

## Required tests

1. **`compare_paths` / `compare_bytes` on the fixture returns
   `SheetsDiffError::EncryptedWorkbook`**, asserted on the variant.
2. **The `side` field is correct** — put the encrypted file on each side in turn.
   `Side` is the only data the variant carries; if it is wrong, a consumer tells
   the user the wrong file is protected.
3. **The rendered message names the condition**, so a change that turns it into
   a generic open failure is caught.
4. **The CLI exits 3** for it, in `tests/cli.rs`. M4 unit 03 mapped
   `EncryptedWorkbook => 3` deliberately and reasoned about it at length; that
   reasoning is currently unprotected.

## Acceptance criteria

1. A fixture exists that calamine reports as password-protected, with its
   provenance and construction documented.
2. If a dependency was added, the review request argues for it against RFC-026
   and `deny.toml`, and `cargo deny` passes.
3. Opening it yields `EncryptedWorkbook`, asserted on the variant.
4. `side` is asserted correct from both sides.
5. The rendered message is asserted to name the condition.
6. A `tests/cli.rs` test asserts exit code 3.
7. No existing fixture changed; corpus otherwise byte-identical.
8. Nothing under `src/` changed.
9. CHANGELOG under `### Added`.
10. Gates green, full matrix, including `cargo deny`.

## Prohibited shortcuts

- Do not assert `is_err()`. The point is *which* error.
- Do not build the fixture by renaming an unrelated binary until calamine
  happens to reject it. It must fail as password-protected, not merely fail.
- Do not commit a binary with no explanation of how it was made. The next
  person must be able to rebuild or verify it.
- Do not add `cfb` (or anything else) as a dev-dependency without the argument.

## Known risks

- **The detection mechanism is calamine's internal behaviour**, not a public
  contract. If a future calamine changes it, this test breaks and that is the
  test working. Note the coupling in the fixture's documentation so the next
  person reads a breakage correctly.
- A CFB parser is strict about structure. A hand-built container may need real
  care to be accepted; if you find yourself iterating blindly, switch to the
  `cfb` crate route and argue for the dependency instead.
- `not_a_zip.xlsx` is 25 bytes and undocumented. If you find its provenance
  while working nearby, say so — but do not fix it here.

## Required evidence

- The fixture, and how it was produced
- The dependency argument, if any
- Test output
- Corpus diff showing only the addition
- CI run link

## Review request format

Per development policy §9.2, plus the fixture's provenance and the dependency
argument if one was made.
