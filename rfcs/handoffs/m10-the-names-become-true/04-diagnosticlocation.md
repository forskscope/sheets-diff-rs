# Handoff 04 — `DiagnosticLocation` is half-populated

**Governing.** [RFC-037](../../accepted/037-v3-scope.md) (M9 finding O-A/F-4,
moved here 2026-09-25); RFC-005 (diagnostics)
**Roadmap.** M10 — unit 04
**Sequence.** Any time. Parallel with 02 and 03.

## Purpose

`DiagnosticLocation` says where a diagnostic came from. Of eleven construction
sites, **two** fill it, one fills half of it, and eight leave it empty — and
`--format json` now publishes the field. Decide per site what the location
should say, and make each site say it.

## Background

**This is not "populate everything".** For a workbook-level diagnostic — a
defined name changed, workbook metadata differs — `sheet_name: None` is the
*correct* answer, and filling it in would be a new lie. The defect is that the
field is populated where it is convenient rather than where it is known, so a
consumer cannot tell "this diagnostic is not about a sheet" from "nobody set
this".

The eleven sites, as they stand (verified 2026-09-25):

| Site | `sheet_order` | `sheet_name` | Level |
|---|---|---|---|
| `meta.rs:78`, `:98`, `:117`, `:138` | `None` | `None` | workbook |
| `meta.rs:184` | `None` | `Some(name)` | workbook |
| `objects.rs:90` | `Some(index)` | `Some(name)` | workbook |
| `objects.rs:116` | `None` | `None` | workbook |
| `matcher.rs:178` | `None` | `None` | workbook |
| `diff.rs:770` | `Some(index)` | `Some(name)` + `address` | **sheet** |
| `align.rs:83`, `:151` | `None` | `None` | **sheet** |

**The two that are plainly wrong are `align.rs:83` and `:151`.** They are
*sheet-level* diagnostics — `AlignmentBoundExceeded` (the sheet silently fell
back to positional comparison) and `DuplicateAlignmentKey` (rows may have been
paired wrongly) — pushed into a specific `SheetDiff`, so the sheet is known to
the caller. They record nothing.

M8 unit 02 worked around this: `render_unified` names the sheet from the owning
`SheetDiff` rather than from the location. That was right for the renderer and
it does not help a JSON consumer, who sees these two diagnostics inside the
correct sheet's array with `"sheet_name": null` and no way to say so from the
location alone.

**`meta.rs:184` is the other clear inconsistency**: it has the name and sets
`sheet_name` without `sheet_order`.

## Change scope

- `src/align.rs`, `src/meta.rs`, `src/objects.rs`, `src/matcher.rs`, `src/diff.rs`
  — the sites your audit says should change
- `src/model.rs` — `DiagnosticLocation`'s doc comment
- `tests/`
- `rfcs/done/005-*.md`, `rfcs/done/033-*.md`
- `CHANGELOG.md`

## Non-change scope

- **Do not change `DiagnosticLocation`'s shape.** No new fields, no changing
  `Option<T>` to `T`. The optionality is meaningful and this unit gives it a
  meaning, rather than removing it.
- **Do not change which diagnostics are produced**, their `kind`, or their
  `severity`.
- Do not change `render_unified`'s sheet-naming. It reads the owning
  `SheetDiff` and that remains more reliable than the location, because a
  `SheetDiff` cannot be wrong about which sheet it is. **Leave it.**
- Do not touch `DiffStage` — unit 02's.

## Required implementation

1. **Audit all eleven sites and decide each**, on one rule you state up front.
   The architect's proposed rule, which you may improve on:

   > **If the code pushing the diagnostic knows which sheet it concerns, the
   > location names that sheet — both `sheet_order` and `sheet_name`, never one
   > without the other. If the diagnostic is not about a particular sheet, both
   > stay `None`, and the doc comment says that `None` means "not about a
   > sheet", not "not recorded".**

2. **`align.rs:83` and `:151` name their sheet.** The two functions do not
   currently receive it; pass what is needed. This is the sheet-level data a
   JSON consumer has no other way to reach.
3. **`meta.rs:184` sets both fields or neither**, per your rule.
4. **Every other site is either changed or explicitly justified** in the review
   request. "Left as-is because the diagnostic is not about a sheet" is a good
   answer; silence is not.
5. **`DiagnosticLocation`'s doc comment states the rule**, including what `None`
   means. That sentence is the deliverable a consumer actually reads.
6. **RFC-005** records the rule.

## Required tests

- A comparison that trips `DuplicateAlignmentKey` reports the sheet in
  `location`, by name **and** order, and the values match the owning `SheetDiff`.
  Same for `AlignmentBoundExceeded`.
- **A cross-check**: for every sheet-level diagnostic the corpus and the
  synthetic fixtures produce, `location.sheet_name` equals the owning sheet's
  name. This is the guard on the rule, not on the two sites.
- A workbook-level diagnostic still reports `None`, asserted deliberately so the
  rule is pinned in both directions.

Show each failing first by a targeted change.

## Acceptance criteria

1. The rule is stated in one sentence and applied to all eleven sites.
2. `align.rs`'s two sites name their sheet, order and name.
3. No site sets one of the pair without the other.
4. Every unchanged site is justified in the review request.
5. `DiagnosticLocation`'s doc says what `None` means.
6. The cross-check test passes, and the both-`None` test pins the other
   direction.
7. **Which goldens moved, and proof each differs only in `location`.** Any
   fixture producing a sheet-level diagnostic will move.
8. No change to which diagnostics are produced — diagnostic counts per scenario
   identical to 2.6.0.
9. CHANGELOG `### Changed`, naming the serialised effect.
10. Gates green.

## Prohibited shortcuts

- **Do not fill `sheet_name` everywhere to make it uniform.** A workbook-level
  diagnostic given a sheet name is a worse defect than one given none, and it is
  the defect this milestone exists to remove.
- Do not derive the sheet in the renderer and call the location fixed. The
  library caller who never renders is the one being fixed.
- Do not change `AlignmentBoundExceeded`'s or `DuplicateAlignmentKey`'s payload
  to carry the sheet. The location field exists for this.

## Compatibility constraints

**Serialised output changes.** `location.sheet_name` and `location.sheet_order`
go from `null` to values for sheet-level diagnostics. `--format json` shipped in
2.6.0, so this is a published, machine-readable surface — name it in the
CHANGELOG rather than describing it only as a fix.

No type changes, so a Rust caller still compiles. This one is safe at a minor;
it rides in v3 because v3 is next, not because it needs a major.

## Known risks

- **`align.rs`'s functions may not have the sheet in scope.** Threading it
  through is the work. If it requires a signature change to a `pub(crate)`
  function, that is fine; if it would require a **public** API change, stop and
  report — that is a different unit.
- Goldens will move for any fixture producing a sheet-level diagnostic. M8 unit
  02's review established the corpus produces **no** sheet-level *warnings*, but
  it does produce sheet-level `Info` (`FormulaUnavailable`) — which is already
  fully populated at `diff.rs:770`, so the movement may be zero. **Check; do not
  assume either way.**

## Required evidence

- The stated rule, and the eleven-site audit with a decision per site
- Pre-fix failure transcripts
- The cross-check across corpus and synthetic fixtures
- Which goldens moved and proof of what changed in each
- Per-scenario diagnostic counts, 2.6.0 vs now, showing production unchanged
- CI run link

## Review request format

Per development policy §9.2. Additionally: state the rule in one sentence and
give the eleven-site table with your decision and reason for each, including the
ones you left alone.
