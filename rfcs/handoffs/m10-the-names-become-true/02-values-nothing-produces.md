# Handoff 02 — Values nothing produces

**Governing.** [RFC-037 §3.2 and §3.4](../../accepted/037-v3-scope.md);
RFC-018 (formula comparison); RFC-033 (public model lexicon); RFC-035
**Roadmap.** M10 — unit 02
**Sequence.** Any time. Parallel with 01 (landed), 03, 04.

## Purpose

Five public values that nothing constructs, four of them listed in the stable
diagnostic-code table callers are explicitly told to match on. Remove them. Then
document the six that stay, so the next reader does not re-derive this list.

## Background

### The four `DiagnosticKind` variants (§3.2)

Verified repo-wide, including `#[cfg(test)]` modules: each appears only in its
own declaration and its `code()` arm.

| Variant | Stable code | Why it cannot occur |
|---|---|---|
| `FormulaCachedValueUnverified` | `formula_cached_value_unverified` | Nothing constructs it. **RFC-018 §146 says the engine "may emit" it** — a false record. |
| `UnsupportedCellValue { detail }` | `unsupported_cell_value` | Its one use was the duplicate-alignment-key condition; RFC-035 correctly moved that to `DuplicateAlignmentKey`, and nothing replaced it. |
| `DateTimeNotNormalized` | `datetime_not_normalized` | Nothing constructs it. |
| `LimitTruncatedCells { limit, observed }` | `limit_truncated_cells` | **Structurally impossible.** Its documented meaning is that a cell limit truncated the comparison; limits return `Err(LimitExceeded)`. There is no truncation path. |

`code()`'s own doc tells callers these strings are **the stable programmatic
surface** and to match on them rather than on the `#[non_exhaustive]` enum. A
caller doing exactly what we instruct writes four arms that can never run.

### `SourceKind::Unknown` (§3.4)

Zero construction sites. `open.rs` sets `Path`, `Bytes` or `Reader` at the three
entry points, and those are the only ways a workbook is opened. `SourceDescription`
is an **output**: the crate builds it, a caller reads it. `Unknown` cannot arise.

## The six values that stay, and why

RFC-037 §3.4 listed six more never-constructed values. **They were decided on
2026-09-25 and they are not yours to revisit** — they are here so you do not
remove them by pattern-match, and so you can write the doc comments.

| Value | Why it stays |
|---|---|
| `DiffStage::Open`, `::Normalize`, `::Aggregate` | A taxonomy of a pipeline that really has these stages. Not a claim about an outcome. |
| `DisplaySource::ReaderProvided`, `::ApplicationProvided` | **Not dead.** `CellDisplay::new(text, format, source)` is `pub` and `source` is a `pub` field, so these are vocabulary *for callers supplying their own display text*. |
| `ObjectCompareMode::CompareAvailable` | Its own doc already says it behaves as `WarnIfPresent` today and why. An honest reservation. |
| `CellError::Other(String)` | Forward-compatible catch-all for an Excel error string we do not recognise. |
| `CellValue::Integer`, `::Duration`, `::Unsupported` | M4 unit 01 decided this deliberately, per variant. **Do not relitigate.** |

**The rule that separates them from the removals:** a value the *engine* would
have to produce and never does is a dead arm. A value a *caller* may hand us, or
that describes the shape of the pipeline, is vocabulary — and vocabulary is
allowed to be wider than today's usage.

## Change scope

- `src/model.rs` — the removals, the `code()` arms, the doc comments
- `src/objects.rs` — `CompareAvailable`'s doc, if it needs sharpening
- `tests/`
- `rfcs/done/018-*.md` — the false "may emit" sentence
- `rfcs/done/033-*.md` — the lexicon
- `docs/` — any user-facing diagnostic-code table
- `CHANGELOG.md`

## Non-change scope

- **Do not remove any of the six that stay.** The table above is the decision.
- **Do not change which diagnostics the engine produces**, or any `code()`
  string that survives. A surviving code is a stable identifier and this unit
  does not touch it.
- Do not change `Severity`, `DiffStage`'s variant *list*, or `DiagnosticLocation`
  — the last is unit 04's.
- Do not "simplify" `CellError::Other` away because its `Display` arm is the
  only reference. The arm is what makes it usable when it is first constructed.

## Required implementation

1. **Remove the four `DiagnosticKind` variants and their `code()` arms.**
2. **Remove `SourceKind::Unknown`.**
3. **`code()`'s doc comment loses any hedge about unreachable codes** if unit 04
   of M8 left one — after this unit every code in the table is producible, which
   is the first time that has been true. Say so.
4. **Document the six that stay**, each in its own words, reusing M4 unit 01's
   established form (`src/model.rs:239`):

   > Cannot occur: *(reason)*. A match arm on this variant is unreachable today;
   > it is retained as *(what for)*, not as a live case.

   **Except `DisplaySource::ReaderProvided` / `::ApplicationProvided`, which
   need the opposite wording** — they are not unreachable, they are *yours to
   construct*. Say that this crate only ever produces `SheetsDiffDefault` and
   that a caller building a `CellDisplay` may use either of the others. Getting
   this one backwards would be the same defect in a new place.
5. **RFC-018's "may emit `DiagnosticKind::FormulaCachedValueUnverified`"** is
   corrected to say it was specified and never implemented. **Do not delete the
   sentence** — the specification stands; the claim of emission does not.
6. **RFC-033's lexicon** reflects the removals and marks which listed values are
   live.

## Required tests

- **A test asserting no diagnostic produced by the corpus carries a removed
  code.** After the removals this cannot compile against the old names, so write
  it as a positive: every `code()` the corpus produces is in an expected set.
- **A guard on the six that stay**, one assertion that the engine does not
  produce them, commented as *documentation guards* so a later reader does not
  delete them as tautologies.

If either finds something reachable, **stop and report** — a value in the
removal list is alive and the finding is wrong. That would be the better
outcome.

## Acceptance criteria

1. The four `DiagnosticKind` variants and their `code()` arms are gone.
2. `SourceKind::Unknown` is gone.
3. **Your own construction-site search**, command shown, for all eleven values —
   the five removed and the six kept — before and after. My count is five and
   six; verify it.
4. The six kept values are documented, with **`DisplaySource`'s two using
   caller-facing wording, not unreachable-variant wording**.
5. RFC-018 corrected without deleting the specification.
6. RFC-033 agrees with the code.
7. Corpus byte-identical. Goldens: state whether any carried a removed code
   (none should — they cannot be produced).
8. CHANGELOG `### Removed`, listing every removed name **and its code string**,
   since a caller may have matched the string rather than the variant.
9. Gates green.

## Prohibited shortcuts

- **Do not keep a removed variant as `#[deprecated]`.** This is the major.
- Do not remove a `code()` string for a variant that survives.
- **Do not mark a value unreachable without checking it yourself.** My counts
  have been wrong eight times in this project and you have caught every one.
- Do not merge the four removals into one catch-all variant. Removing a lie and
  adding a vaguer one is not progress.

## Compatibility constraints

**Breaking, and that is what v3 is for.** Two distinct breaks, and the CHANGELOG
must name both:

1. A caller matching a removed **variant** fails to compile — the intended
   signal.
2. A caller matching a removed **code string** compiles fine and silently stops
   matching. There is no way to warn them at build time, which is exactly why
   every removed string must be listed by name in `### Removed`.

## Known risks

- `docs/src/` executes. A diagnostic-code table there may be asserted on.
- **`UnsupportedCellValue`'s history is in `CHANGELOG.md:846` and RFC-035.** Do
  not contradict them: it was correctly *removed* from the duplicate-key path
  and is dead because nothing replaced it, not because that removal was wrong.
- `SourceKind` is `#[non_exhaustive]`, so a future source kind is added as its
  own variant rather than falling back to `Unknown`. That is the argument for
  removing it; say it in the CHANGELOG rather than leaving it implied.

## Required evidence

- The eleven-value construction-site search, before and after, command shown
- Corpus byte-comparison and the golden statement
- The two guard tests' output
- Gates, each with exit status
- CI run link

## Review request format

Per development policy §9.2. Additionally: report your own counts for the
removal list and the keep list even if they agree with mine, and say whether any
of the six kept values looks to you like it belongs in the other column.
