# Handoff f132-01 — A 512-byte file aborts the process; decline it before delegating

**Defect response. Ahead of M9's remaining units. The next release waits for this** (owner,
2026-09-29). **Written:** 2026-09-29. **Found by:** M9 unit 01, the first fuzz run that reached
past the ZIP header. **RFC:** RFC-028 §7 (annotated: currently violated).

## Purpose

`sheets_diff::compare_bytes` aborts the calling process on a 512-byte input. Close it here, at the
one place we control, without waiting for an upstream release — and do it without changing what a
caller sees for any input that works today.

## Background

**Verified from the public API, twice, by two people.**

```
compare_bytes(w, w) where w is 512 bytes
  -> memory allocation of 9261285372 bytes failed   (9.26 GB, one allocation, then abort)
```

`Xlsx::new` calls `check_for_password_protected` before returning, which parses the input as a CFB
(OLE2) container. A sector-count field in the CFB header reaches `Vec::with_capacity` without being
checked against the file's actual length (`calamine` 0.36.1: `xlsx/mod.rs:2939` → `cfb.rs:260`,
field read at `cfb.rs:224`).

**Every bound we advertise is upstream of it.** `max_input_bytes` bounds the input's *length* — 512
bytes passes trivially — while the allocation's size comes from a field *inside* that input.
`max_cells_read` and `max_alignment_product` are never reached: no sheet is read. **`Limits::hardened()`,
which the README and the threat model recommend for untrusted input, does not prevent it** — verified,
identical abort. And it is an abort, not an `Err`, so a caller cannot catch it.

It is in **every released version**, including 3.1.0. It is hand-craftable — the fuzzer minimized it
to 515 bytes, but nothing about the file requires a fuzzer.

**The second defect from the same unit is not this unit's.** "Attempt to multiply with overflow" in
`get_row_and_optional_column` (`xlsx/mod.rs:2838`) lives *inside a valid archive*, so the pre-screen
below cannot reach it; it panics only where debug assertions are on. Report it upstream (§*Required
implementation* 5) and leave it.

## Change scope

- `src/open.rs` — the pre-screen, at `open_workbook_from_cursor` (`:185`) or immediately before it.
- `src/error.rs` — only if a new variant proves necessary; **prefer not** (see *Non-change scope*).
- `tests/` — the regression tests.
- `fuzz/corpus-quarantine/` — **deleted**, its seed returned to `fuzz/corpus/fuzz_open_xlsx_bytes/`.
- `.github/workflows/ci.yaml` — `fuzz-smoke` gains `fuzz_self_comparison`.
- `fuzz/README.md` — drop the quarantine paragraph and the "not in the matrix yet" paragraph.
- `CHANGELOG.md`, `docs/src/maintainers/threat-model.md`, `rfcs/done/028-*.md` — the status of a
  defect those three currently describe as open.

## Non-change scope

- **Do not add a public API.** No new `Limits` field, no new option, no new builder method. A caller
  should not have to opt in to not aborting.
- **Prefer no new public error variant.** `SheetsDiffError` is `#[non_exhaustive]`, so adding one is
  a minor rather than a break — but adding a variant to describe a *dependency's* bug teaches callers
  about our internals. Reuse `EncryptedWorkbook` and the existing not-an-xlsx path. If you conclude a
  new variant is genuinely necessary, **stop and say why** rather than adding it.
- **Do not upgrade or patch `calamine` here**, and do not vendor it. If a fixed upstream release
  appears later, dropping the pre-screen is a separate decision — and probably the wrong one, since
  the pre-screen is cheap and independent of upstream.
- Do not touch the overflow defect (above).

## Required implementation

**1. Decline non-ZIP input before it reaches `calamine`.**
A `.xlsx` is a ZIP archive: it begins `50 4B 03 04` (`PK\x03\x04`), or `50 4B 05 06` for an empty
archive. Anything else cannot be an `.xlsx` and must not be handed to `Xlsx::new`, because that call
parses unknown bytes as CFB before it fails.

**2. Classify CFB input without parsing it — this is the design decision, and it is already made.**
An encrypted `.xlsx` *is* a CFB container, so "reject all CFB" would turn today's
`EncryptedWorkbook` into a generic failure and break `tests/encrypted_workbook.rs` and the CLI's
exit code 3. A legacy `.xls` is also CFB, and calling it "encrypted" would be a lie.

Distinguish them with a **byte scan, not a parse**:

- CFB magic (`D0 CF 11 E0 A1 B1 1A E1`) **and** the byte string `EncryptedPackage` encoded
  **UTF-16LE** appears anywhere in the input → `EncryptedWorkbook`. Verified present in
  `tests/fixtures/corrupt/encrypted.xlsx` and absent from the crash artifact.
- CFB magic without it → the same outcome a non-`.xlsx` gets today.

A scan is O(n) over bytes already in memory and allocates nothing. **Never allocate a buffer whose
size comes from the input's own contents** — that is the whole defect.

**3. Bound the scan.** Search a prefix (a few hundred KiB is far more than enough — the CFB
directory sits near the front) rather than the whole file, so a 500 MiB input does not pay a full
scan to be rejected. State the bound you chose and why it is safe for a real encrypted workbook.

**4. Keep the error identical for everything that works today.** Same variant, same `Display`, same
CLI exit code, same JSON. The observable change must be exactly: *inputs that used to abort now
return an error.*

**5. Draft the upstream reports; do not file them.** ~~Report both defects upstream to `calamine`.~~
**Corrected 2026-09-29 by the owner: this instruction was wrong and should not have been given.**
Filing an issue on a third party's tracker is outward-facing communication published under the
owner's name, and it is theirs to send — the same rule as letters to a consumer. The dev team was
right to stop at a draft. Now written down as `.git-exclude/rules/004-outward-facing-communication.md`.

Write the reports as drafts under `.git-exclude/upstream/calamine/send/draft/`, pure report content
(a `**Subject:**` line and the body, no internal notes), one file per defect, named
`YYYY-MM-DD-<slug>.md`. The owner files them and moves the file up out of `draft/`. Do not attach a
weaponized artifact to a public tracker; offer it privately.

## Required tests

- **The abort is gone:** the minimized 515-byte artifact
  (`.git-exclude/review-request/m9-01-corpus-reaches-the-reader/evidence/oom-crash/self_comparison-minimized-515b`,
  after its 3-byte header) through `compare_bytes` returns an `Err`. Commit it as a fixture; it is
  515 bytes and it is the whole point.
- **Failing first**, by removing the pre-screen (not by reverting the file): the test aborts. Because
  an abort kills the test process, demonstrate it as a **separate process** with a memory cap — e.g.
  `ulimit -v` plus a small binary — and capture the output. Do not leave a test that can abort the
  suite.
- **Encrypted workbooks are unchanged:** `tests/encrypted_workbook.rs` passes untouched, and the
  error variant is still `EncryptedWorkbook` — asserted, not assumed.
- **Valid workbooks are unchanged:** the full corpus, all 80 goldens byte-identical.
- **A CFB that is not an encrypted `.xlsx`** gets the not-an-`.xlsx` outcome, not `EncryptedWorkbook`.
  Construct one (a CFB header with no `EncryptedPackage` stream name).
- **`fuzz_open_xlsx_bytes` with the quarantined seed restored:** clean at `-runs=20000`, **five
  different `-seed` values, corpus copied fresh for each run** — libFuzzer writes discovered inputs
  back into the corpus directory, and reusing it makes the result meaningless (both the review and I
  hit this).
- Same for `fuzz_self_comparison` before adding it to the matrix.

## Acceptance criteria

1. The 515-byte artifact returns an `Err` from `compare_bytes`; the abort is gone, demonstrated
   failing-first in a separate process.
2. No public API change: `cargo public-api --simplified diff 3.1.0` shows nothing beyond what is
   already unreleased.
3. `EncryptedWorkbook` still reported for real encrypted workbooks, same variant and exit code.
4. All 80 goldens byte-identical; corpus clean.
5. Both fuzz targets clean at `-runs=20000` × 5 seeds, fresh corpus each run, **with the quarantined
   seed restored**.
6. `fuzz/corpus-quarantine/` **deleted**; `fuzz_self_comparison` in the `fuzz-smoke` matrix.
7. Upstream report(s) **drafted** under `.git-exclude/upstream/calamine/send/draft/`. Filing is the
   owner's, and the CHANGELOG links the issue once it exists — so this criterion is met by the
   drafts, not by a link.
8. CHANGELOG moves this from "known, unfixed" to fixed, under `### Security`, naming the versions
   affected (all released) and that `Limits::hardened()` did not protect against it. The threat model
   and RFC-028 §7 likewise — and RFC-028's assurance row goes from **No** back to **Partially**, not
   to Yes: the overflow defect is still open.
9. Gates as always, plus `cargo check --manifest-path fuzz/Cargo.toml --bins` (rule 003).

## Prohibited shortcuts

- **Do not catch the abort.** It is an allocation failure, not a panic; `catch_unwind` does not
  apply, and a process-level workaround is not a fix.
- **Do not bound it with a new `Limits` field.** The input is 512 bytes; no limit a caller can set
  describes this, and offering one implies they were meant to.
- Do not parse the CFB header to validate the field. Reimplementing the parser we are avoiding is how
  this defect gets a second home.
- Do not skip *drafting* the upstream report because we have a local fix. Every other `calamine`
  user is exposed. Equally, **do not file it** — see *Required implementation* 5.
- Do not restore the quarantined seed before the fix passes — and do not leave it quarantined after.

## Known risks

**1. The pre-screen could reject a valid workbook.** That would be a regression worse than the
defect. Some `.xlsx` files carry a ZIP prefix (self-extracting archives) or leading whitespace —
neither is a thing Excel writes, but assert against the whole fixture corpus, not against reasoning.
If you find any real workbook that does not start with `PK`, **stop and report it**; the rule needs
changing before it ships.

**2. `EncryptedPackage` could appear in a non-encrypted CFB**, mislabelling a legacy `.xls` as
encrypted. Low: it is a specific stream name. Note it as a known imprecision in the code comment;
both outcomes are errors, and the imprecision is in which error, not whether.

**3. The scan's prefix bound could miss the marker** in an unusual encrypted workbook, turning
`EncryptedWorkbook` into the generic error. Same class as risk 1 and the reason §3 asks for the
bound to be justified rather than picked.

**4. This closes one of two defects.** RFC-028 §7 stays annotated as violated by the overflow, and
the assurance row stays **Partially**. Resist writing "fixed" where "one of two fixed" is true.

## Required evidence

Under `.git-exclude/review-request/f132-01-decline-before-delegating/evidence/`:

1. The artifact through `compare_bytes`, before (separate process, memory-capped, showing the abort)
   and after (an `Err`).
2. The failing-first demonstration with the pre-screen removed, and the `cmp`-verified restore.
3. Encrypted, legacy-CFB, and valid-workbook behaviour, before and after, side by side.
4. Goldens and corpus: 0 differ.
5. Both fuzz targets, 5 seeds each, fresh corpus per run, with the seed restored.
6. `cargo public-api --simplified diff 3.1.0`.
7. The paths of the upstream drafts (not links — they are not filed by this unit).
8. Gate sweep including rule 003, one scratch dir, deleted.

## Review request format

`.git-exclude/review-request/f132-01-decline-before-delegating/README.md`, with:

- The prefix bound you chose for the scan, and why it cannot miss a real encrypted workbook.
- Whether any fixture or real-world workbook fails the `PK` test.
- Whether you needed a new error variant, and if so why the existing ones would not do.
- **Your view on whether the release should wait for an upstream `calamine` fix as well**, or ship
  with the pre-screen alone. The owner has decided the release waits for *this* fix; whether it also
  waits on upstream is a separate question and I want your read before I put it to them.
