# RFC-037: v3 Scope — the breaks, and the closed list

**Status.** **Proposed** 2026-09-25. Awaiting the owner's acceptance.
**Target:** 3.0.0
**Created:** 2026-09-25
**Author:** high-capability model (architect / design authority)

---

## 1. Why a major, and why now

M8 exists because public surfaces promise what the engine does not do. Four of
its findings cannot be fixed inside 2.x, because the fix is to **remove or
rename a public item** — and cargo hands `^2` users a new minor automatically,
so a deleted name is a build failure they did not ask for. Adding is minor;
removing is major. That is the whole of the constraint.

The alternative — document each lie in place and carry it — was the plan until
2026-09-25. The owner's objection was correct: **a major costs almost nothing
right now**, because there is no production use. It will cost more later, and
the list of things needing one is still short. This is the cheapest this will
ever be.

## 2. The rule this list is built on

Not "everything unused". The test is **whether a competent reader learns the
truth from the type**:

- **Honest reservations stay.** `ExecutionMode` has one variant and says so,
  with RFC-025's reasoning. `CellFormat`, `WorkbookChange`, `ObjectChange` are
  documented as reserved and always empty. These exist *to avoid future breaks*
  and are doing their job. Removing them would be a break now and another break
  when they are filled.
- **False or unreachable claims go.** A variant that describes an outcome the
  engine never produces, or an option that can only return an error, teaches the
  reader something untrue. Those are removed.

## 3. The closed list

### 3.1 `SheetMatchReason` — rename and remove (M8 O1)

Verified: three variants, **one** ever constructed, and it is wrong at all three
sites.

| Variant | Construction sites | Disposition |
|---|---|---|
| `ExactName` | none, and **structurally impossible** — the enum appears only inside `SheetChange::Renamed` / `RenamedAndMoved`, and a rename is not an exact-name match | **Remove** |
| `ContentSimilarity` | none; the matcher inspects no cell content anywhere | **Remove** |
| `IndexAndContent` | `matcher.rs:143`, `:151`, `:212` — **every rename this crate reports** | **Replace** with honest variants |

The three sites are two different things:

- `:143` and `:212` pair on index alone (`:212` is inside a function named
  `index_match`). → **`IndexOnly`**
- `:151` pairs the last unmatched sheet on each side, with **unequal** indices.
  Neither index nor content matched. → **`SoleRemainingPair`** (name to be
  settled in the handoff)

### 3.2 `DiagnosticKind` — remove four unreachable variants (M8 A4)

Verified repo-wide, including `#[cfg(test)]` modules: each appears only in its
own declaration and its `code()` arm.

| Variant | Why it cannot occur |
|---|---|
| `FormulaCachedValueUnverified` | nothing constructs it; **RFC-018 §146 claims the engine "may emit" it** |
| `UnsupportedCellValue { detail }` | its one use moved to `DuplicateAlignmentKey` (RFC-035); nothing replaced it |
| `DateTimeNotNormalized` | nothing constructs it |
| `LimitTruncatedCells { limit, observed }` | **structurally impossible** — limits return `Err`, so there is no truncation to report |

`code()`'s doc tells callers these strings are the stable programmatic surface
and to match on them. Four of them can never arrive.

### 3.3 Options that can only fail — remove (new, 2026-09-25)

`DiffOptions::validate()` rejects these unconditionally. They are public,
constructible, documented as if usable, and every caller who selects one gets
`InvalidOptions`:

- `FormulaCompareMode::NormalizedText`
- `FormulaCompareMode::RawAndNormalized`
- `FormatCompareMode::NumberFormatOnly`
- `FormatCompareMode::AllAvailable`

`NormalizedText`'s doc says *"Requires a normaliser feature; returns
`InvalidOptions` if selected without one."* **There is no such feature and none
is planned** — so the sentence describes a configuration that cannot exist.

`FormatCompareMode` then has one remaining variant, `Ignore`. Decide in the
handoff whether the enum survives as a single-variant reservation (the
`ExecutionMode` pattern, honest) or the field goes. RFC-022 is `accepted`, which
argues for keeping it.

### 3.4 Other never-constructed public values — decide, then act (new)

Verified as having zero construction sites anywhere in `src/`:

| Value | Note |
|---|---|
| `DiffStage::Open`, `::Normalize`, `::Aggregate` | `DiagnosticLocation.stage` never takes these three |
| `DisplaySource::ReaderProvided`, `::ApplicationProvided` | only `SheetsDiffDefault` is ever set |
| `SourceKind::Unknown` | `open.rs` sets `Path`, `Bytes`, `Reader` only |
| `ObjectCompareMode::CompareAvailable` | RFC-023's compare mode is unimplemented |
| `CellError::Other(String)` | appears only in a `Display` arm; nothing constructs it |
| `CellValue::Integer`, `::Duration`, `::Unsupported` | documented as unreachable by M4 unit 01 |

**These are not automatically removals.** `DiffStage` names pipeline stages that
exist; `CellValue`'s three carry M4's reasoning for keeping them. Each needs a
one-line decision in the handoff against §2's rule — the sweep's job is to make
sure none is missed, not to prejudge it.

### 3.5 `cells_read` — change what it counts (M8 A6, moved here)

`DiffMetrics::cells_read` reports **bounding-box area**, not cells read:
`sparse_range` reports **5,200** against `cells_compared`'s **2**.

This was M8 unit 05 and moves to v3, where it belongs: changing what a public
metric counts is a semantic break, and doing it here means it stops being a
golden-churning problem inside a minor. Every golden moves; each must be shown
to differ only in `cells_read`.

### 3.6 `AlignmentMode::HeaderColumn` — the name describes something else (new)

Found during this sweep. Its doc says *"Match rows using the first row as a
column-header identity."* `header_column_alignment` (`align.rs:187`) delegates
to `row_key_alignment` with `columns = [1]` and never reads a header:

```rust
let key_col: Vec<u32> = vec![1]; // row-1 = header row; match data by that col
row_key_alignment(old_cells, new_cells, &key_col, diagnostics)
```

So `HeaderColumn` is exactly `RowKey { columns: vec![1] }` under a name that
promises header inference. Two public modes, one behaviour, and the second's
name and documentation describe a third thing. Same family as §3.1.

**Decide:** implement what the name says, rename the variant to what it does, or
remove it. Not a documentation fix — the doc would have to say "despite the
name, this is `RowKey` on column 1", which is an admission, not an API.

### 3.7 Builder duplication — decide (M8 unit 07's known risk)

`DiffOptionsBuilder` has both `number_compare` and `number_compare_policy`,
taking the same `NumberComparePolicy` and setting the same field. One should go;
removing a public method is a break.

## 4. Explicitly **not** in scope

Stating these closes the list, which is the point of writing it down.

- **Reserved placeholders stay**: `ExecutionMode`, `CellFormat`, `FormatChange`,
  `WorkbookChange`, `ObjectChange`, and the `Reserved until RFC-0xx` fields.
  They are honest and they exist to prevent the next break.
- **`Deserialize`** stays declined (RFC-014, ForskScope asked directly).
- **No new features.** Not style comparison, not a formula normaliser, not
  parallel execution. v3 is a correction release, not a capability release.
- **MSRV stays 1.88** unless something here forces otherwise.
- **No module reorganisation.** `view.rs`'s status (M9 O5) is a documentation
  decision, not a layout change.
- **Nothing found after this RFC is accepted joins it** without the owner
  reopening the list. That is what "closed" means, and it is the mechanism that
  stops a major becoming a six-month project.

## 5. Sequence

1. **2.6.0 ships first**, carrying M8 units 00–03 (done), 06 and 07. It is the
   release ForskScope adopts. It must not wait for v3.
2. **M8 units 04 and 05 are withdrawn** into this RFC — 04 becomes §3.1/§3.2,
   05 becomes §3.5. M8 closes at unit 07.
3. **v3 handoffs are written against the 2.6.0 tree**, not this one.
4. **A `v2-to-v3` migration guide** is required, matching
   `docs/src/migration/v1-to-v2.md` in form, and every removal here needs a row
   in it saying what to use instead.

## 6. Acceptance criteria for this RFC

1. Every item in §3 has a decision recorded before a handoff is written.
2. The migration guide covers every removal.
3. No item is added to §3 after acceptance without the owner reopening it.
4. ForskScope is told before 3.0.0 is cut, not at it.

## 7. Open questions for the owner

1. **Does 2.x get anything after 3.0.0?** Security-only, or nothing? ForskScope
   will likely be on 2.6.x. Cheaper to decide now than at the cut.
2. **§3.3** — does `FormatCompareMode` survive as a single-variant reservation?
3. **§3.6** — implement, rename, or remove `HeaderColumn`?
