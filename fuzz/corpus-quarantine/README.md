# Quarantined seeds and targets

**Seeds and targets that are correct, wanted, and temporarily out of what libFuzzer runs in CI**,
because they reach a crash we have not closed yet. Quarantined seeds are still committed, still
decoded and still exercised in-process by `tests/fuzz_corpus_reaches_reader.rs`, which walks this
directory as well as `fuzz/corpus/`. Nothing here is broken or deprecated.

**This directory should be empty. It is a debt marker, not a category.**

## `fuzz_open_xlsx_bytes/paired_encrypted`

Quarantined 2026-09-29 (M9 unit 01 review). RFC-028 §6's "password-protected" category, built from
`tests/fixtures/corrupt/encrypted.xlsx`.

**The defect it was quarantined for is fixed (f132, 2026-09-29):** an encrypted `.xlsx` is a CFB
container, and mutations of one used to reach a `calamine` bug that took a sector-count field from
the CFB header straight to `Vec::with_capacity` unchecked, aborting the process. `src/open.rs` now
declines any non-ZIP input before it reaches `calamine` at all, with a byte-scan carve-out for a real
encrypted workbook; the specific abort this seed used to cause is gone (verified: the minimized
crash artifact returns an ordinary `Err` now — see `.git-exclude/review-request/f132-01-*/evidence`).

**It stays quarantined anyway, for a different, measured reason.** Restoring it to the corpus
`fuzz_open_xlsx_bytes` actually fuzzes with raises the hit rate of a *second*, still-open defect —
`calamine`'s `get_row_and_optional_column` integer overflow (`xlsx/mod.rs:2838`, reported upstream,
not ours to fix) — within CI's own budget. Measured at `-runs=20000`, 20 runs each, fresh corpus per
run:

| Corpus | Crashed |
|---|---|
| 11 seeds, without `paired_encrypted` | **0 / 20** |
| 12 seeds, with `paired_encrypted` restored | **3 / 20** |

Plausibly: the CFB-shaped bytes give libFuzzer's crossover mutator more structurally distinct
material to combine with the other seeds, reaching further into `calamine`'s cell-parsing code —
this was not investigated further, since the conclusion (do not restore yet) does not depend on the
mechanism. **The condition for its return is that the overflow defect closes** (upstream fix, or a
guard on our side if one is ever justified — see the threat model), not that this seed's own defect
does; that condition has already been met and was not sufficient by itself.

## `fuzz_self_comparison` — the whole target

Not in CI's `fuzz-smoke` matrix. Quarantined for two reasons found running it at `-runs=20000` × 20
seeds, fresh corpus per run; **one is now fixed.**

- **Fixed (f133):** `src/matcher.rs`'s exact-name matching (`match_sheets`, the
  `new_sheets.iter().position(|n| n.name == old.name)` line) did not check whether a new-side sheet
  was already claimed before pairing an old-side sheet to it. Two old sheets sharing a name —
  reachable here because 3 bytes flipped inside a compressed ZIP stream corrupted a sheet name to the
  empty string on more than one sheet — both matched the *same* first same-named new sheet; one new
  sheet was left unmatched and reported `Added`, with its cells reported as spurious changes.
  **Reproduced deterministically outside the fuzzer** (`compare_bytes(bytes, bytes)`,
  `cells_changed == 3`, ten repeats, same three cells every time). Unlike the other defect below, this
  one did **not** crash or panic in an ordinary build: it was a **silent wrong answer**, which this
  project treats as the worse category. See the threat model, *Sheet matching: a second, related
  defect found while fixing the first*, for the fix and for a related, distinct, and worse defect it
  exposed in `ExactNameThenIndex` mode — also fixed, same release (f134); it was never a reason this
  target is quarantined (it was a correctness defect, reachable by construction, not something running
  the fuzzer risked crashing on).
- **Still open:** the same overflow defect as `paired_encrypted` above, reached through a different
  call path (`read_sheet_cells` / `worksheet_cells_reader`), independent of any specific seed.

Add the target to the matrix once the overflow defect closes, in the same change that closes it —
the same condition as `paired_encrypted`, above.
