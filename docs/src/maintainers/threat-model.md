# Threat model

This document exists so that a consumer's decision to re-enable `.xlsx`
comparison — after having disabled it over `RUSTSEC-2026-0194` /
`RUSTSEC-2026-0195` — is informed rather than a leap of faith. It says what
this crate defends against, what it does not, and how each claim is checked.
Path mirrors ForskScope's own consumer-side threat model so the two sit
side by side.

This is not a guarantee. Where a control is partial, this document says so;
a threat model that reads as a guarantee invites reliance the code does not
support, which is worse than no threat model at all.

## Assets

- **Confidentiality of workbook content.** `sheets-diff` never writes
  workbook content anywhere it wasn't asked to (no network, no telemetry,
  no logging of cell values). See [Non-defences](#explicit-non-defences)
  for what this does *not* cover.
- **Availability of the host process.** A workbook must not be able to
  crash, hang, or exhaust memory on the process that calls this crate.
- **Integrity of the diff result** — first-class, not an afterthought. In a
  diff/**merge** workstation, a silently missed difference means a user is
  shown "identical", accepts a merge, and loses data — a data-loss path
  reachable from ordinary input with **no attacker involved**. A false
  negative is therefore an integrity failure, not a quality defect, and is
  treated with the same severity as a crash.

  RFC-035 Handoff 05 fixed four paths where this crate reported "identical"
  for cells that differed:
  - ISO-typed date/time and duration values (`Data::DateTimeIso`/
    `DurationIso`) always compared equal to each other, regardless of their
    actual values — `2024-01-01T00:00:00` and `2099-12-31T23:59:59` were
    reported identical.
  - The 1900/1904 date-epoch flag was hardcoded `false`, so two dates
    recorded under different epochs could be silently misread as different
    when they were the same instant, or vice versa depending on policy.
  - Row alignment could silently merge two unrelated cells into one
    coordinate when an inserted row's number coincided with an unrelated
    matched or removed row's number — reachable on any sheet of ordinary
    size under a non-default alignment mode.
  - Formula text could attach to the wrong cell when calamine's formula
    range and value range had different origins — an ordinary shape (a
    label row above a formula), not an edge case.

  The `formula` fixture is the worked example of how one of these hid in
  plain sight: it existed since RFC-015 to test formula-versus-value
  changes, and its first bless recorded the wrong cell address. It passed
  on every run for years, because a golden only detects *change* — it
  cannot detect having been born wrong. See
  [`tests/fixtures/corpus/README.md`](../../../tests/fixtures/corpus/README.md#a-goldens-first-bless-is-the-one-moment-its-content-is-unreviewed)
  for the lesson in full.

## Trust boundary

**The workbook bytes are untrusted.** Both files passed to any
`compare_*`/`compare_*_with_options` entry point are treated as hostile
input, regardless of source.

**The caller is trusted.** `sheets-diff` defends against a hostile
workbook, not a hostile consumer. `DiffOptions`, callback closures
(`Cancellation`, `ProgressSink`), and the process environment are not
attacker-controlled in this model. If your embedding lets an untrusted
party choose `DiffOptions` or supply a `Cancellation`/progress callback,
that is a boundary this crate does not defend.

## Actors

**Someone who supplies a workbook** — the only actor in scope. Not a
network attacker: `sheets-diff` has no network code path (NF-015) and no
telemetry (NF-016), so there is no remote surface to attack. Not a supply
chain attacker in the dependency-substitution sense either — that is
`deny.toml`'s [sources] gate's job, covered under
[Verification map](#verification-map) below, not a runtime threat this
document's actors model addresses.

## Surfaces

Each surface below has a stated mitigation and a stated residual risk. A
surface with no residual risk line has one implicitly: "whatever the
mitigation doesn't cover."

### The zip container

`.xlsx` is a ZIP archive. **Mitigation:** parsing goes through `calamine`
→ `zip` 8.6.0, `calamine`'s own pinned, `deny.toml`-gated dependency (not
this crate's own zip handling — `sheets-diff` never manipulates ZIP
structure directly). `max_input_bytes` (500 MiB default, `Limits::hardened()`
50 MiB) bounds the compressed input size *before* any byte is read
(`fs::metadata`/`Seek`-measured length/slice length, all checked ahead of
`fs::read`/`read_to_end`/the owning copy). **Residual risk:** a zip bomb
within the size bound (e.g. a small compressed file expanding to a large
one) is bounded by `zip`'s own decompression behaviour, which this crate
does not independently cap. `fuzz_open_xlsx_bytes` exercises arbitrary
byte sequences through this path, including malformed archives, but a fuzz
corpus is not exhaustive.

### XML parsing

`calamine` parses `xl/workbook.xml`, `xl/worksheets/sheetN.xml`, shared
strings, and styles via `quick-xml`. **Mitigation:** `calamine` 0.36
resolves `quick-xml` to 0.41.0, which fixes both `RUSTSEC-2026-0194`
(quadratic runtime on duplicate-attribute checking) and `RUSTSEC-2026-0195`
(unbounded namespace-declaration allocation) — the two advisories that
caused this crate to be disabled in the first place. **Residual risk:**
`sheets-diff` inherits whatever XML-parsing behaviour `quick-xml`/`calamine`
have; a future advisory in that chain is caught by the `deny.toml`
`[advisories]` gate on the next dependency-tree scan, not proactively.

### Normalisation (`src/normalize.rs`)

Converting `calamine::Data` into this crate's public `CellValue`.
**Mitigation:** a single normalisation boundary (RFC-026) — calamine types
never leak into the public API, so a change in calamine's internal
representation cannot silently change public behaviour without going
through this one function. `#![forbid(unsafe_code)]` is in force
crate-wide; the one `unsafe` block this crate ever had
(`String::from_utf8_unchecked` in `address::col_to_label`) is gone.
**Residual risk:** normalisation correctness is only as good as its test
coverage — RFC-035 Handoff 05 found and fixed four cases where it silently
produced a wrong-but-plausible result (see [Assets](#assets) above) via
manual audit, not systematic verification; there is no proof no fifth case
remains.

### Alignment (`src/align.rs`, `src/diff.rs`)

The optional row-alignment feature (`RowKey`/`RowSignature`/`HeaderColumn`
modes) that reduces false-positive cascades after row insertions/deletions.
**Mitigation:** `max_alignment_product` (default 25,000,000, `Limits::hardened()`
same value — this bound was already conservative) caps the `old_rows ×
new_rows` LCS matrix *before* it is allocated; measured to keep the
default's worst case under ~15ms and ~95MB (RFC-035 §9), with the failure
mode it exists to prevent (~10GB, process-aborting) sitting two size
classes above the bound. Exceeding it degrades to positional comparison
with an `alignment_bound_exceeded` diagnostic — never an error, never a
process abort. The coordinate-space collision defect (D-03) that could
silently merge two unrelated cells is fixed as of Handoff 05. **Residual
risk:** two correctly-computed diffs can still share a *display* address in
rare cases (an inserted row and a matched/removed row's numbers coinciding)
— a labelling ambiguity, not a merged/lost cell; see
[Residual risks](#residual-risks-worth-naming). Positional mode (the
default `AlignmentMode`) does not use this bound at all and has no
alignment-related resource surface.

### The bounds themselves (`Limits`)

**Mitigation:** `Limits::default()` bounds the two superlinear/unbounded-by-
construction paths (`max_alignment_product`, `max_input_bytes`); the four
genuinely linear paths (`max_sheets`, `max_cells_read`, `max_cells_compared`,
`max_diffs_returned`) stay unbounded by default, since bounding a path that
scales predictably with input the caller chose to open would break working
code for no safety benefit the caller couldn't have anticipated (RFC-035
§5.1). `Limits::hardened()` sets a concrete value on every dimension for
callers comparing genuinely untrusted input at scale. `max_cells_compared`
specifically bounds **coordinates visited** — the union of both sides'
populated cells per sheet, remapped by alignment when alignment applies —
checked cumulatively across the whole comparison and before each sheet's
comparison work begins, not the number of differences found (that is
`max_diffs_returned`'s separate dimension). **Residual risk:** a
caller who does not call `Limits::hardened()` and does not set the four
linear limits explicitly has no protection against a workbook with an
enormous number of sheets, cells, or diffs — this is the documented,
deliberate trade-off, not an oversight, but it is a real gap for a caller
who assumes `default()` means "safe."

## Explicit non-defences

Said plainly, because the failure mode of a threat model is overclaiming:

- **No sandboxing.** `sheets-diff` runs in the caller's process with the
  caller's privileges. It relies entirely on Rust memory safety
  (`#![forbid(unsafe_code)]`) and its own logic being correct — there is no
  process isolation, no seccomp filter, no capability restriction.
- **No defence against a malicious caller.** See
  [Trust boundary](#trust-boundary) — `DiffOptions`, callbacks, and the
  environment are trusted inputs, not modelled as attacker-controlled.
- **No guarantee for arbitrarily large input, without `Limits::hardened()`.**
  See [The bounds themselves](#the-bounds-themselves-limits).
- **No macro, formula, or external-link execution** (NF-017). Formula
  *text* is compared as a string; it is never evaluated. There is no code
  path that executes anything a workbook contains.
- **No attempt at Excel-complete semantics.** Object categories this crate
  does not compare, and why, are documented in `src/objects.rs`'s coverage
  diagnostic (emitted on every comparison) — see that module's doc comment
  for the current split between "not exposed by calamine's API at all"
  (charts, images, comments, data validation, conditional formatting, cell
  styles/number formats) and "available upstream, not yet used by this
  crate" (hyperlinks, merged regions, tables, pivot tables).
- **No path-leakage guarantee beyond best-effort** (NF-018, SHOULD not
  MUST). Error variants carry a `display_name` derived from the path's file
  name, not the full path, but this is a SHOULD-level requirement, not a
  hard guarantee verified by a dedicated test.

## Verification map

| Control | Machine-checked? | Where |
|---|---|---|
| No `RUSTSEC-*` advisory reachable | Yes | `deny.toml` `[advisories]`, CI `deps` job |
| No network-capable crate in the dependency tree (NF-015) | Yes | `deny.toml` `[bans]` — 17 explicitly denied crates (`reqwest`, `hyper`, `tokio`, …), CI `deps` job |
| Dependency versions come only from crates.io | Yes | `deny.toml` `[sources]` (`unknown-registry`/`unknown-git` denied), CI `deps` job |
| License compliance | Yes | `deny.toml` `[licenses]` allowlist, CI `deps` job |
| No `unsafe` code | Yes | `#![forbid(unsafe_code)]` in `src/lib.rs` — a compile error, not a lint |
| No panic on arbitrary/malformed input | Partially | `fuzz_open_xlsx_bytes` (arbitrary bytes through `compare_bytes`), `fuzz_addr_roundtrip`, `fuzz_range_merge`, `fuzz_diff_options_builder` — bounded smoke runs (`-runs=20000`) in CI's `fuzz-smoke` job, not a continuous fuzzing campaign |
| Full feature-combination matrix builds and tests | Yes | CI `test` job — 5 feature combinations × 2 OSes |
| MSRV floor is real, not merely declared | Yes | CI `msrv` job — builds at the pinned toolchain, asserts the resolved version matches |
| Comparison output does not silently drift | Yes | The fixture corpus (`tests/fixtures/generated/*/expected.json`) — CI `tree` job additionally asserts the test suite itself never dirties the working tree |
| Resource bounds actually bound (§5.1–5.4) | Partially | Unit tests assert degrade-not-error and the measured default (RFC-035 §9); there is no continuous benchmark asserting the *measured* costs stay within the stated envelope over time |
| Normalisation/alignment/formula-attachment correctness | No dedicated ongoing check beyond the fixture corpus | The four Handoff 05 defects were found by manual audit, not by an automated property; a fifth of the same shape would only be caught if it happens to move a golden or fail a hand-written test |
| Comparison never accesses the network (NF-015) | Indirectly | Enforced structurally (no networking dependency can enter the tree, per the `[bans]` row above) rather than by a runtime sandbox or a dedicated test that observes zero syscalls |

A control with "Partially"/"No"/"Indirectly" in the second column is not a
finding to fix reflexively — some of these (a continuous fuzzing campaign,
a runtime syscall sandbox) are a different order of engineering investment
than this crate currently makes. The point of this table is that a reader
can tell the difference between "checked on every commit" and "checked
once, by a person, and trusted to stay true."

## Residual risks worth naming

All surfaced during M2; none currently fixed.

- **`CellValue::Duration` is unreachable through `.xlsx`.** Calamine's
  `Data::DurationIso` — the only source of `CellValue::Duration` — is
  produced exclusively by calamine's `.ods` reader; the `.xlsx` cell reader
  has no code path that constructs it. Since `sheets-diff` opens only
  `.xlsx`, this variant, its normalisation arm, and its comparison arm are
  presently dead code for every input this crate accepts (RFC-035 Handoff
  05 §6). The fix that exists is correct for if this path ever becomes
  reachable; it is not reachable today.
- **Two correctly-computed diffs can share a display address.** The
  D-03 fix (Handoff 05) resolved the coordinate-space collision that could
  *silently merge or misattribute* two cells. It kept the existing
  addressing convention (matched/removed rows report at their old-sheet row
  number, inserted rows at their new-sheet row number), so a numeric
  coincidence between the two can still produce two distinct, individually
  correct `CellDiff` entries with the same `.address`. This is a labelling
  ambiguity a consumer's UI would need to disambiguate (e.g. via which
  `CellChangeKind` applies), not a lost or wrong comparison.
- **The bytes-input path owns a copy where it could borrow.** `open_bytes`
  turns a borrowed `impl AsRef<[u8]>` into an owned `Vec<u8>` via `to_vec()`
  before handing it to the `Cursor`-based reader (RFC-035 Handoff 04
  review §3). Eliminating it would need `Xlsx<Cursor<&[u8]>>` and a lifetime
  parameter on the internal `OpenedWorkbook` type — a real refactor with API
  reach, not a line change — and was never in scope for M2.

  **Corrected 2026-08-17 (M7 Handoff 01).** This entry previously said the copy
  was *"doubling peak memory."* That was inferred from reading the code and is
  wrong: measured, `compare_bytes` peaks **2.6–4.8% above `compare_paths`** at
  10,000 cells and up. The raw input bytes do roughly double, but they are only
  2.4–2.6% of peak — peak is dominated by the ~450 B/cell normalised
  representation both entry points build identically. Peak is instead dominated,
  among costs we control, by `cell_map_to_align`'s clone of every `CellValue`
  (+33%, paid only by non-`Positional` alignment modes). Method and full
  figures: [performance.md](./performance.md).

## Advisory-response policy

What happens when a `RUSTSEC` advisory lands in this crate's dependency
tree, per RFC-035 §5.7:

1. **The `deny.toml` `[advisories]` gate failing the build is the trigger.**
   There is no manual watch process, no scheduled review — the CI `deps`
   job fails on the next commit or scheduled run that resolves an affected
   version, and that failure is the notification.
2. **Assess reachability from untrusted workbook input** — this crate's
   trust boundary (see above). An advisory in a codec `sheets-diff` never
   exercises against untrusted bytes is a different priority than one in
   the zip/XML parsing path itself.
3. **If reachable, respond with one of:** a dependency version bump; an
   upstream fix contributed if none exists yet; or a documented, expiry-dated
   exception in `deny.toml`'s `[advisories.ignore]` — never silence by
   deleting or weakening the gate. An empty `ignore` list is the default
   and the goal; an entry in it is a decision made once, expires, and gets
   re-decided, not a permanent accommodation.
4. **Notify known consumers.** A downstream consumer running a fail-closed
   posture (as ForskScope did) pays a higher cost for staying silent than
   this project does for speaking up — a consumer who doesn't hear about a
   fix stays disabled longer than necessary; one who doesn't hear about a
   new advisory stays exposed.

### Why this policy exists

`RUSTSEC-2026-0194` and `RUSTSEC-2026-0195` sat in this crate's dependency
tree (via `calamine` 0.35 → `quick-xml` 0.39.4) for two months before this
project acted on them. Nobody here noticed first — **the consumer's own
`cargo audit` gate caught it**, and their response was to disable `.xlsx`
comparison entirely rather than trust an unpatched dependency. `deny.toml`'s
`[advisories]` gate (RFC-035 Handoff 03) exists so that the next one is
visible here on the day it lands, not discovered by a consumer's gate two
months later.
