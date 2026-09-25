# Handoff 09 — One error type, and two more structs

**Governing.** [RFC-037 §3.10 and §3.11](../../accepted/037-v3-scope.md);
RFC-013 (CLI), RFC-014 (serde), RFC-033
**Roadmap.** M10 — unit 09
**Sequence.** After 08. **Before the migration guide**, which must describe what
landed.

## Purpose

Two functions return a `Result` that cannot be `Err`, with the only `String`
error in the crate. Two public structs can still only be extended by a break.

## Background — §3.10

```rust
pub fn to_json(diff: &WorkbookDiff) -> Result<String, String>
pub fn to_json_pretty(diff: &WorkbookDiff) -> Result<String, String>
```

**The only two public functions that do not return `SheetsDiffError.`** All six
`compare_*` entry points and `DiffOptionsBuilder::build` do.

**And `Err` is unreachable.** Established at audit, each point checked rather
than assumed:

- **No maps or sets anywhere in the serialised shape** — so serde_json's
  non-string-key failure cannot arise. `AlignmentSummary` exposes counts, not
  `RowMapping`.
- **Every `Serialize` is derived** — no custom impl can error.
- **Non-finite floats serialise as `null`, measured**:
  `to_string(&vec![NAN, INFINITY, NEG_INFINITY, 1.5])` → `[null,null,null,1.5]`.
  `CellValue::Number(f64)` is the only float in the model.
- Writing to a `Vec<u8>` does not fail.

**The cost is downstream.** `src/main.rs` carries an error arm that exits 2 for
a serialisation failure, and M8 unit 03 could only test it by **fault injection
in a scratch copy**. An unreachable `Result` forced an unreachable branch, which
forced a test that faked the failure.

## Background — §3.11

`address::CellAddress` (3 public fields) and `address::ComparedRange` (2) are
`pub`, in a `pub mod`, and not `#[non_exhaustive]`. **`ComparedRange` is
reachable as `SheetDiff::compared_range` — a result field**, which is why §3.9's
claim that "the results got it right" was wrong.

## Change scope

- `src/output/json.rs` — the two signatures
- `src/main.rs` — the JSON dispatch and its dead arm
- `src/address.rs` — the two structs
- `tests/`
- `rfcs/done/013-*.md`, `rfcs/done/014-*.md`, `rfcs/done/033-*.md`
- `docs/`
- `CHANGELOG.md`

## Non-change scope

- **Do not change the serialised shape.** Not a field, not a name, not an
  ordering. This unit changes a *signature* and two *attributes*.
- Do not touch `render_summary` / `render_unified` — they already return
  `String`.
- **Do not mark the six structs in `output::view`.** They wait on M9's O5;
  marking them would answer that question by accident.
- Do not reintroduce `SheetsDiffError::Internal`.

## Required implementation

1. **`to_json` and `to_json_pretty` return `String`.**
2. **Their doc comments say why there is no `Result`** — briefly, and in terms a
   caller can check: the shape contains no maps, every `Serialize` is derived,
   and non-finite floats serialise as `null`. **A future field that breaks any
   of those restores the need for a `Result`**, so say that too; it is the
   sentence that stops someone adding a `HashMap` to the model without noticing.
3. **`src/main.rs`'s JSON arm loses its error branch.** Check what else that
   simplifies — the `match` may collapse.
4. **`#[non_exhaustive]` on `CellAddress` and `ComparedRange`**, each with the
   same doc paragraph unit 08 used: build with the constructor or `Default`, not
   a literal; a future field is additive; and a `compile_fail,E0639` doctest.
5. **Check `CellAddress` has a usable constructor.** `#[non_exhaustive]` blocks
   literal construction from outside, so if callers currently build one by
   literal they need a constructor — `new_unchecked` exists internally; decide
   whether the public path is adequate and **report it**, do not invent a new
   API silently.
6. **RFC-014** records the signature change; **RFC-013** records that the CLI's
   JSON path can no longer fail.

## Required tests

- `to_json` / `to_json_pretty` return `String` and parse as JSON — the existing
  assertions, minus the `Result` handling.
- **The CLI's `--format json` exit codes are unchanged** for every input class.
  Removing an error arm is exactly the change that could silently move one.
- `E0639` doctests on both structs, as unit 08 did.
- **A downstream test reads `CellAddress`'s and `ComparedRange`'s fields** and
  constructs them by whatever path survives — proving `#[non_exhaustive]` did
  not lock callers out of a type they must be able to make.

The signature change demonstrates by compile failure; say so plainly.

## Acceptance criteria

1. Both functions return `String`; no public function outside `SheetsDiffError`.
2. Their docs state the four conditions and that a future field can restore the
   need for a `Result`.
3. `main.rs`'s dead arm is gone; **exit codes unchanged, shown as a table.**
4. `CellAddress` and `ComparedRange` are `#[non_exhaustive]` with `E0639`
   doctests.
5. **The construction path for both is stated** — what a caller uses now.
6. `output::view`'s six are untouched.
7. Serialised output byte-identical; corpus byte-identical; goldens unmoved.
8. CHANGELOG `### Changed`, both items, with the migration for each.
9. Gates green, **including `cargo check --manifest-path fuzz/Cargo.toml --bins`**
   (rule 003 — `fuzz/` is a separate crate no local gate compiles).

## Prohibited shortcuts

- **Do not keep the `Result` and return `Ok` always.** That is the defect.
- Do not add a `SheetsDiffError` variant for a failure that cannot happen.
- Do not make the fields private instead of marking the structs.
- Do not mark `output::view`'s structs "while you are there".

## Compatibility constraints

**Breaking, twice:**

1. A caller writing `to_json(&d)?` or `.unwrap()` will not compile. The fix is
   to delete the `?`. Say that in the migration — it is the rare break that
   makes calling code shorter.
2. Constructing `CellAddress` or `ComparedRange` by struct literal fails, as
   with unit 08's eight.

## Known risks

- **`CellAddress` may be constructed by literal in tests, docs or `fuzz/`.**
  `fuzz/src/fuzz_addr_roundtrip.rs` is named for it. **Ask the compiler across
  both manifests**, per rule 003 — this is the unit where that rule earns itself.
- `docs/src/` executes; anything calling `to_json(...)?` will fail to compile,
  which is the migration working.

## Required evidence

- The unreachability argument re-checked, not quoted from the RFC
- Compile-failure transcripts for both breaks
- The exit-code table for `--format json`
- `E0639` for both structs; the construction-path statement
- Every migrated site, across **both manifests**, with the commands shown
- Gates, each with exit status, including the `fuzz/` check
- CI run link

## Review request format

Per development policy §9.2. Additionally: state the construction path for
`CellAddress` after the change, and confirm `fuzz/` compiles — naming the
command.
