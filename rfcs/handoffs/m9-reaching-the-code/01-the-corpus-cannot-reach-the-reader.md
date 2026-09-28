# Handoff 01 — The fuzz corpus cannot reach the sheet reader

**Milestone:** M9, unit 01 (the substantive one). **Item:** D1.
**Written:** 2026-09-29. **Depends on:** unit 00 (CI asserts the standing properties).
**RFC:** RFC-028 (`rfcs/done/028-fuzzing-and-hostile-input-hardening.md`) §5, §6, §8.

## Purpose

`fuzz_open_xlsx_bytes` has never read a cell. Every seed in its corpus fails at
`not an xlsx file`, so every run of it has fuzzed the ZIP-header check and nothing
behind it — no sheet reader, no normaliser, no comparer, no alignment, no `Limits`.
Make the fuzzer reach the code it is supposed to guard, and leave behind a check
that says so in the ordinary gate, not a claim in a README.

## Background

**Verified 2026-09-29, before writing this.** All three seeds, through the target's
own splitting logic:

```
empty              0 bytes -> Err(cannot open old workbook '<unknown>': not an xlsx file)
random_bytes      32 bytes -> Err(cannot open old workbook '<unknown>': not an xlsx file)
truncated_zip     10 bytes -> Err(cannot open old workbook '<unknown>': not an xlsx file)
```

A real fixture pair, for contrast: `Ok, 1 sheet, 8 cell diffs`.

**D1 said the corpus has no valid `.xlsx`. There is a second reason, and it is the
one that matters: the target's framing makes a valid seed nearly impossible to
write.** `fuzz_open_xlsx_bytes` takes one byte string and splits it at
`data.len() / 2`:

```rust
let mid = data.len() / 2;
let old = &data[..mid];
let new = &data[mid..];
```

So a seed must be two valid workbooks concatenated **and of exactly equal length**,
or the split lands inside one of them and both halves are invalid. Demonstrated:

```
the same two files concatenated and halved by the target -> Err(cannot open new workbook ...)
```

**This is why the corpus is the way it is.** Nobody was careless; the target cannot
accept the seed that RFC-028 §6 asks for. Adding files to `fuzz/corpus/` without
changing the framing would produce exactly the same three errors, and a unit that
reported "seeds added" would be another marker credited with more than it measured.

**RFC-028 is another RFC-009.** Its §6 lists ten seed categories; three exist, and
**all seven missing ones require a valid workbook** — "valid ZIP but not XLSX",
"workbook with empty sheets", "wide columns", "many sheets", "formulas",
"unsupported objects", "password-protected". Its §8 requires fuzz coverage of
"maximum sheet count; maximum cell count; maximum returned diff count; cancellation
during long comparison" — **no target constructs a `Limits` or a `Cancellation` at
all**. Its §5 suggests six targets; four exist (`fuzz_cell_value_normalization` and
`fuzz_sheet_matching_manifest` do not, and `fuzz_addr_roundtrip` exists without being
listed). The Status line says "Implemented (2.0.0–2.2.3) — verified 2026-08-16".

**The ruling on that Status is the same one I gave for RFC-009 in M10 unit 01, and
it is deliberately the same.** §5 says "Suggested targets" and §6 says the corpus
"should include" — those are sketches, not contracts. The contract is §10's
acceptance criteria, and those pass. **So: annotate, do not downgrade.** What is
*not* acceptable is §8, which is a requirement in the imperative and is unmet; say so
where it sits.

**Why this is the substantive unit of M9.** f123 — the bounding-box defect
ForskScope reported, where a 5 KB workbook with two distant cells allocated ~646 MB —
lived in the sheet reader, past the archive. So did f130 (keyless rows dropped from
alignment) and f131 (the coordinate loops). **Every defect this crate has shipped and
fixed in the last month lived in code this fuzz target has never executed.**

## Change scope

- `fuzz/src/fuzz_open_xlsx_bytes.rs` — the framing.
- `fuzz/src/` — one new target (see *Required implementation* §2).
- `fuzz/Cargo.toml` — the new `[[bin]]`, and `arbitrary` if you use it.
- `fuzz/corpus/` — seeds.
- `fuzz/README.md` — the corpus section, and the target list.
- `tests/` — one new test file: the permanent guard.
- `.github/workflows/ci.yaml` — the `fuzz-smoke` matrix gains the new target.
- `rfcs/done/028-*.md` — §6 and §8 annotations.
- `CHANGELOG.md` — `### Documentation` or `### Internal`; **no library code changes**,
  so nothing under `Added`/`Fixed`/`Changed`.

## Non-change scope

- **No `src/` change.** If you find a defect with the newly-reaching fuzzer, **stop
  and report it** rather than fixing it here — it is its own unit, with its own
  measurement, and mixing a fix into the unit that found it is how we lose the
  evidence that the fuzzer found it.
- **Do not weaken `fuzz_open_xlsx_bytes`'s oracle.** "Arbitrary bytes must not panic"
  is a real property and it is the one the target has been proving. It keeps it.
- Do not add the two RFC-028 §5 targets that do not exist
  (`fuzz_cell_value_normalization`, `fuzz_sheet_matching_manifest`). Out of scope;
  note them in the annotation and leave them.
- `fuzz/` stays out of the workspace and off the default gate; the only compile check
  is `cargo check --manifest-path fuzz/Cargo.toml --bins` (rule 003).

## Required implementation

**1. Give `fuzz_open_xlsx_bytes` a framing a seed can satisfy.**
The midpoint split is the defect. Replace it with an explicit one — a length prefix,
or `libfuzzer_sys`'s `Arbitrary` support for a tuple/struct input. Either is fine;
**say which and why in the review request.** The requirement is that a seed file
containing two specific workbooks round-trips to exactly those two workbooks.

**2. Add one target that reaches past the archive with an oracle worth having.**
The new target compares a workbook **with itself** — `compare_bytes_with_options(w, w, opts)`.
This is deliberate and it buys three things at once:

- **No framing problem.** One seed is one valid `.xlsx`. Nothing to split.
- **A real oracle, not "did not panic":** a workbook compared with itself must
  produce **zero cell diffs**, in **every** alignment mode, and no sheet change other
  than unchanged/same-name. **I verified this fires before asking you for it.** On a
  400-row workbook with a blank key every twentieth row, the current tree gives 0 / 0 / 0
  under `Positional` / `RowKey[1]` / `RowSignature`; with f130's defect reintroduced
  (keyless rows dropped before pairing, `src/align.rs:205`) it gives **0 / 20 / 0**.
  So the oracle has a demonstrated capacity to fail — **and note which mode caught it.**
  `RowSignature` stayed at 0 because every row has a signature and it has no keyless
  path at all, so the invariant is only load-bearing for `RowKey` here. A target that
  only ever selected `RowSignature` would prove nothing about f130's class.
- **It reaches the code.** read → normalise → match → align → compare, all of it, on
  every input.

Drive `AlignmentMode` and `Limits` from the fuzz input as well, since RFC-028 §8
requires it and nothing does it. **Bound what you drive** — see *Known risks*.

**3. Seed both corpora from the fixtures that already exist.**
`tests/fixtures/generated/*/{old,new}.xlsx` are valid, small (~5 KB) and already
version-controlled. Derive seeds from them rather than committing new binaries where
you can. For the RFC-028 §6 categories the fixtures do not cover (many sheets, wide
columns, formulas, empty sheets), generate them with `rust_xlsxwriter` from a small
committed script, so the corpus is reproducible rather than a set of opaque blobs.
**Say in the review request which of the ten §6 categories are now present and which
are not, by name.** Do not claim the list is closed if it is not.

**4. The permanent guard — this is the deliverable that outlives the unit.**
A test in `tests/` (main crate, ordinary gate, no nightly, no cargo-fuzz) that walks
each `fuzz/corpus/<target>/` directory, feeds every file through **the same framing
the target uses**, and asserts that the corpus still reaches the reader: at least one
seed per corpus must get past the archive, and for the self-comparison corpus every
valid seed must yield zero cell diffs.

Factor the framing so the test and the target share one function rather than two
copies that can drift — the drift is the failure mode here, and a test that decodes
seeds differently from the target proves nothing about the target.

This is the point of the unit. A coverage number is a measurement taken once; this
fails in CI the day someone minimises the corpus down to the empty file again.

## Required tests

- The guard in §4 above.
- **Failing first, by removing the specific code under test** — never by reverting a
  file (M9 standing constraint):
  1. Delete the valid seeds from the corpus → the guard fails ("no seed reaches the
     reader").
  2. Restore the midpoint split in the framing function → the guard fails, because
     the seeds no longer decode to the workbooks they contain.
  3. In `src/align.rs`, drop the keyless rows before pairing (`:205`, f130's original
     defect) → the **self-comparison oracle** must fail under `RowKey`. If it does not,
     the oracle is not reaching alignment and the target needs work, not the test.
     Restore `cmp`-identical. (Do not expect `Positional` or `RowSignature` to move.)
- `cargo check --manifest-path fuzz/Cargo.toml --bins` (rule 003).

## Acceptance criteria

1. Every seed in every corpus decodes, under the target's own framing, to the input it
   is meant to represent — **demonstrated, not asserted**.
2. At least one seed per corpus reaches past the archive into the sheet reader, and
   the guard in `tests/` enforces it in the ordinary gate.
3. The new target's self-comparison oracle **fails** when f130's defect is
   reintroduced, and passes on the current tree — **under `RowKey`**, which is the mode
   that has a keyless path. State the diff count you get; mine was 20 on a 400-row
   workbook with a blank key every twentieth row.
4. `AlignmentMode` and a bounded `Limits` are driven by fuzz input in at least one
   target; RFC-028 §8's four items are each either covered or named as not covered.
5. `cargo fuzz run` on both byte-oriented targets, **`-runs=100000` minimum**, no
   crash — and report **coverage of `src/` before and after** the change. See
   *Known risks* on the prerequisite.
6. Which of RFC-028 §6's ten seed categories exist, by name, and which do not.
7. RFC-028 §6 and §8 annotated; §8 stated as unmet where it sits; Status unchanged,
   with the reasoning recorded.
8. `fuzz/README.md` describes the corpus that exists.
9. CI `fuzz-smoke` matrix includes the new target and is green.
10. Gates as always, plus rule 003. No `src/` change: `git diff --stat src/` empty.

## Prohibited shortcuts

- **Do not report "seeds added" as the outcome.** The outcome is "the fuzzer executes
  the sheet reader", and the two are not the same claim. This unit exists because the
  second was assumed from the first for three releases.
- **Do not let the guard test decode seeds its own way.** One framing function, shared.
- Do not raise `-runs` to hide a corpus that cannot reach the reader; a fuzzer that
  finds the reader by luck at run 90,000 has not been seeded.
- Do not commit large binary seeds. If a category needs a big workbook, generate it.
- Do not "fix" a defect the new target finds. Report it.
- Do not mark a coverage figure as the acceptance of criterion 2. Criterion 2 is the
  test, precisely because a figure cannot fail later.

## Known risks

**1. Mutation destroys ZIP validity, and this is the central technical risk.**
A coverage-guided mutator flipping bytes in a `.xlsx` breaks the CRC and the central
directory almost immediately, so a seeded corpus can reach the reader on run 1 and
spend the rest of the campaign back at `not an xlsx file`. **Measure it; do not assume
either outcome.** If coverage shows this happening, the honest answer is a
**structure-aware** target — build the workbook *from* the fuzz input with
`rust_xlsxwriter`, so every input is a valid archive and the mutation space is the
cell content, sheet count and options rather than the bytes of a zip. That is a
larger piece of work. **If your measurement says it is needed, say so and stop there**
— I will scope it as its own unit rather than have it absorbed into this one. The
self-comparison target in §2 is deliberately the cheapest thing that reaches the
reader; the structure-aware generator is the thing that *stays* there.

**2. `cargo fuzz coverage` needs a component that is not installed.**
`cargo-fuzz` is present (0.13.2) and nightly is available, but
`llvm-tools-preview` is **not** in the nightly toolchain on this machine — I checked.
`rustup component add llvm-tools-preview --toolchain nightly` first. If coverage still
cannot be produced, say so plainly and give criterion 5's before/after some other way
(a counter, a temporary `eprintln!` at the reader's entry, removed before commit). **A
missing measurement is a finding to report, not a criterion to quietly drop** — and it
is better reported than approximated.

**3. Fuzzer-driven `Limits` can OOM the fuzzer and look like a crash.**
`max_alignment_product` at `None` with fuzzer-chosen row counts allocates an
`(m+1)(n+1)×4`-byte table; at 50,000 rows that is ~9.5 GB and the process dies. An OOM
is indistinguishable from a real find in a CI log. Bound what the input can select —
cap the generated row/sheet/cell counts, and keep `max_alignment_product` at a value
that cannot exceed the fuzzer's `-rss_limit_mb`. State the caps you chose.

**4. Overlap with unit 03 (item C4).** `fuzz/README.md:33` describes the corpus and
`:45` wrongly says CI does not run the fuzz targets; C4 belongs to unit 03. **You own
the corpus section here; leave `:45` to unit 03** and say in the review request that
you did, so we do not get two edits to one paragraph or neither.

**5. An RFC-028 §10 criterion names an API that no longer exists.** §10's first
criterion is about `try_from_bytes`; `grep -rn try_from_bytes src/` finds **nothing**.
Note it in the annotation — it is pre-2.0 wording, not a regression, and it is not
yours to rewrite. Flag it and move on.

## Required evidence

Under `.git-exclude/review-request/m9-01-corpus-reaches-the-reader/evidence/`:

1. Every seed, through the framing, with what each reaches (the table in *Background*,
   re-run after the change).
2. `cargo fuzz run` output for both byte targets at `-runs=100000`.
3. Coverage of `src/` before and after — or the §2-risk explanation.
4. The three failing-first demonstrations, with the exact statement removed for each
   and a `cmp` showing the restore.
5. The RFC-028 §6 ten-category table: present / absent, by name.
6. The `Limits` and generated-size caps you chose, and the `-rss_limit_mb` they sit
   under.
7. Gate sweep including rule 003, **one scratch target dir, deleted** (rule 002 as
   narrowed 2026-09-26).
8. `git diff --stat src/` — empty.

## Review request format

`.git-exclude/review-request/m9-01-corpus-reaches-the-reader/README.md`, with:

- Which framing you chose for `fuzz_open_xlsx_bytes`, and why.
- **Whether the seeded corpus survives mutation**, with the measurement — and your
  recommendation on the structure-aware target. This is the question I most want your
  answer to; it decides whether M9 gains a unit.
- Which of RFC-028 §6's ten categories are present and which are not.
- Whether the self-comparison oracle caught the reintroduced f130 defect, and at which
  alignment mode(s).
- Anything the newly-reaching fuzzer found, **unfixed**.
- Your view on whether §8's cancellation item can be fuzzed at all, given that a fuzz
  target has no second thread to trip the token from. I do not know the answer; if it
  cannot, that is the finding, and it belongs in the RFC-028 annotation.
