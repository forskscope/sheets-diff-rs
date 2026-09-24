# Handoff 02 — Two inert options, which are not the same option

**Governing RFCs.** RFC-013 (CLI), RFC-033 (public model lexicon), RFC-023
(diagnostics)
**Roadmap.** M8 — unit 02
**Sequence.** After unit 01 (landed 2026-09-24). Parallel with 03 and 04.

## Purpose

`--no-warnings` and `DiagnosticOptions::min_severity` are both public, both
documented, and neither is read by anything. Give each the behaviour its own
documentation already claims.

## Background

Two findings, A2 and A3, which the readiness review listed together and the
ROADMAP's first disposition proposed to fix together. **That disposition was
wrong and this handoff corrects it** — see §"A correction to my own
disposition" below before reading the rest.

### A2 — `--no-warnings` is declared and never read

`src/main.rs:55`:

```rust
/// Suppress warnings in output.
#[arg(long)]
no_warnings: bool,
```

`grep -c no_warnings src/main.rs` returns **1**: the declaration. `main()` never
consults it. The flag parses, appears in `--help`, and does nothing.

### A3 — `min_severity` is public and never read

`src/options.rs:438`:

```rust
/// Minimum severity to collect.  Defaults to `Info` (collect everything).
pub min_severity: Option<crate::model::Severity>,
```

`grep -rn min_severity src/` returns **one line**: that declaration. Nothing
reads it. A caller who sets it observes no change of any kind.

**Its doc comment is also imprecise.** The field is `Option<Severity>`, so its
actual default is `None`. "Defaults to `Info`" describes the intended *effect*
of `None`, not the value. Say both.

### The third severity behaviour, which is the one that actually runs

`src/output/text.rs:164`:

```rust
.filter(|d| d.severity >= Severity::Warning)
```

`render_unified`'s diagnostics section hardcodes a `>= Warning` threshold.
`Info` diagnostics are collected by the engine, counted in `DiffSummary`, and
then silently dropped by the renderer with no way to ask for them.

So there are three severity controls on the surface and **the only one with
teeth is the one that is not configurable.**

### O3 — half the diagnostics reach no output at all

Found while scoping this handoff, and the most consequential of the three.

Diagnostics live in **two** places: `WorkbookDiff::diagnostics` (workbook-level)
and `SheetDiff::diagnostics` (per sheet). There are 11 push sites across five
files — `meta.rs` (5), `align.rs` (2), `objects.rs` (2), `diff.rs` (1),
`matcher.rs` (1).

The sheet-level ones are surfaced **nowhere**:

- `derive_summary` (`src/model.rs:864`) takes `&sheet_diffs` *and*
  `&workbook_diagnostics`, but its diagnostic loop reads only the second
  argument. Sheet diagnostics never reach `DiffSummary::diagnostics`.
- `render_summary` prints its `diagnostics: N error(s), M warning(s)` line from
  those counters — so it under-reports.
- `render_unified` passes `&diff.diagnostics`, the workbook-level vector, to its
  diagnostics section. Sheet diagnostics are not in it.
- `metrics.diagnostics_emitted` (`src/diff.rs:302`) sums **both**. So the metric
  and the summary disagree with each other whenever a sheet diagnostic exists.

**What is actually invisible.** `align.rs` pushes two diagnostics, both
`Severity::Warning`, both sheet-level:

- `AlignmentBoundExceeded` — `max_alignment_product` was exceeded and the sheet
  **silently fell back to positional comparison**. The comparison degraded and
  nothing tells the user.
- `DuplicateAlignmentKey` — two or more rows share an alignment key, so rows may
  have been paired wrongly. **The results may be misleading and the warning that
  says so is never printed.**

A warning that exists to tell the user their diff may be wrong, and that no
output carries, is the same class of failure as unit 01: the engine knows and
the CLI does not say.

`src/output/view.rs` reads `sd.diagnostics`, and nothing in the crate calls
`view`. That is a separate question (a `pub mod` with no internal caller) and it
is **M9's**, not this unit's — do not act on it.

## A correction to my own disposition

ROADMAP §M8 currently proposes:

> **A2 and A3 together, one mechanism.** Implement `min_severity` in the engine
> and make `--no-warnings` its CLI face.

**Do not do that.** Reading the code to write this handoff showed it is wrong,
and the reason matters more than the correction:

- `min_severity` says *"Minimum severity to **collect**."* It is a collection
  filter. Diagnostics below it never enter `WorkbookDiff::diagnostics`, and so
  never reach `DiffSummary::diagnostics` or `metrics.diagnostics_emitted`.
- `--no-warnings` says *"Suppress warnings in **output**."* It is a display
  control.

Wiring the flag to the field would mean `--no-warnings` changes the **counts**.
`render_summary` prints `diagnostics: N error(s), M warning(s)` from those
counters, so `--no-warnings` would make the summary report zero warnings for a
workbook that had them. A flag that suppresses a display and thereby falsifies a
count is a worse instance of exactly the defect M8 exists to fix.

They are two layers with two meanings. Implement them as two.

## Change scope

- `src/options.rs` — `min_severity`'s doc comment
- `src/diff.rs` — the single filter point, and `diagnostics_emitted`
- `src/model.rs` — `derive_summary` must count sheet diagnostics (O3)
- `src/main.rs` — read `no_warnings`; pass it to the renderer
- `src/output/text.rs` — the diagnostics section's source, threshold and suppression
- `tests/cli.rs`, `tests/integration.rs`
- `rfcs/done/023-*.md` or whichever RFC owns diagnostics — Status correction
- `CHANGELOG.md`

**The 11 push sites in `meta.rs`, `align.rs`, `objects.rs`, `matcher.rs` and
`diff.rs` are *not* in scope to edit.** The filter is applied once, after
assembly — see implementation item 2. They are listed here so you know they
exist, not so you change them.

## Non-change scope

- **Do not change which diagnostics the engine produces**, only which are kept.
- Do not touch `Severity`'s variants or their ordering. `Info < Warning < Error`
  is relied on by the `>=` comparison and by `derive_summary`.
- Do not add a `--warnings-only` / `--min-severity` CLI flag. A severity
  threshold on the command line is a reasonable feature and it is not this
  unit's; if you think it should exist, say so in the review request.
- Do not touch `render_unified`'s hunk logic — only its diagnostics section.

## Required implementation

1. **`min_severity` becomes a collection filter.** A diagnostic whose severity
   is below the configured minimum is not pushed into `diagnostics`. `None`
   keeps every diagnostic, matching the documented default.
2. **The filter applies once, in `src/diff.rs` after assembly and before
   `metrics.diagnostics_emitted` is computed (line 302) and before
   `derive_summary` is called (line 307)** — not at the 11 push sites. One
   place to read, one place to be wrong. **It must cover both** the
   workbook-level vector and every `SheetDiff::diagnostics`; filtering only the
   first would reproduce O3 inside the fix.
3. **O3: `derive_summary` counts sheet diagnostics too.** Its signature already
   takes `&[SheetDiff]`; its diagnostic loop must include each sheet's own
   vector. After this, `DiffSummary::diagnostics` and
   `metrics.diagnostics_emitted` agree — assert that they do.
4. **O3: `render_unified`'s diagnostics section shows sheet diagnostics.**
   Keep them distinguishable — a sheet-level entry should say which sheet, which
   `DiagnosticLocation` already carries.
5. **`DiffSummary` and `metrics.diagnostics_emitted` count what was kept**, not
   what was generated. That is the honest reading of a collection filter: the
   caller asked not to collect them. State this in `min_severity`'s doc comment
   explicitly, because the opposite reading is equally defensible and a reader
   must not have to guess which one we chose.
6. **`--no-warnings` suppresses the diagnostics section of the output** and
   changes nothing else. Counts in the summary line stay truthful.
7. **`min_severity`'s doc comment** states the field's default (`None`), the
   effect of `None` (every diagnostic is collected), and that the counters
   follow the filter.
8. **The renderer's hardcoded `>= Warning` gets a comment** saying it is a
   display threshold, distinct from `min_severity`, and why the two exist.

## Required tests

- `min_severity = Some(Warning)` on a comparison that produces an `Info`
  diagnostic: it is absent from `diagnostics`, and `DiffSummary` agrees.
- `min_severity = None` (default): the `Info` diagnostic is present. This is the
  control that proves the filter is not always-on.
- `min_severity = Some(Error)`: warnings are dropped too.
- CLI, `--format unified`, a workbook producing a warning: the section appears
  without `--no-warnings` and does not appear with it.
- **CLI, `--no-warnings`, `--format summary`: the `diagnostics: … warning(s)`
  count is unchanged from the run without the flag.** This is the test that
  distinguishes this implementation from the one the ROADMAP originally
  proposed; it must be present.
- **O3: a fixture that trips `DuplicateAlignmentKey`** — the warning appears in
  `--format unified`, and the `--format summary` count includes it. This must be
  shown failing before the fix, because today it is invisible in both.
- **O3: `DiffSummary::diagnostics` totals equal `metrics.diagnostics_emitted`**
  for a workbook with both workbook-level and sheet-level diagnostics. Today
  they differ; that inequality is the pre-fix failure.
- **O3: a numeric fixture producing many `FormulaUnavailable`** — the `info`
  count rises, and neither renderer's output changes. Report the count.

Each must be shown failing before the fix, by removing the specific statement
under test — never by reverting a file.

## Acceptance criteria

1. `min_severity` is read by the engine and filters collection.
2. The filter is applied in exactly one place.
3. `None` collects everything; the control test proves it.
4. Summary counters and `diagnostics_emitted` follow the filter, and the doc
   comment says so.
5. `--no-warnings` suppresses the diagnostics section and **does not change any
   count**, proven by the test above.
6. `min_severity`'s doc comment states the real default.
7. The renderer's display threshold is commented and still `>= Warning`.
8. Every test shown failing pre-fix by a targeted removal.
9. **O3: `derive_summary` counts sheet diagnostics; summary totals and
    `diagnostics_emitted` agree**, asserted by a test.
10. **O3: sheet-level warnings appear in `render_unified`'s section**, with the
    sheet identified.
11. **O3's effect on default output is measured and reported** — which fixtures'
    rendered output changed, and how. "None changed" is a suspicious answer;
    check the alignment fixtures specifically.
12. Fixture corpus byte-identical. **Note:** the corpus is the input `.xlsx`
    files. If any *golden output* file exists for an alignment fixture, it will
    move — that is O3 working. Say which moved and why.
13. CHANGELOG: `### Added` for the two options, **and a separate `### Fixed`
    entry naming O3** and the changed default counts. Gates green.

## Prohibited shortcuts

- **Do not implement `--no-warnings` as `min_severity = Error`.** §"A correction
  to my own disposition" is the whole reason this unit exists in this shape.
- **Do not delete either option.** Both are public surface; `min_severity` is a
  `pub` field on a `pub` struct.
- Do not make `--no-warnings` suppress stderr. Diagnostics are not stderr, and
  errors printed on stderr are not warnings.
- Do not filter at render time and call `min_severity` implemented. A library
  caller who never renders anything must observe the filter.

## Compatibility constraints

**The options are additive. O3 is not, and this is the part to get right.**

`min_severity` keeps its `None` default and the renderer keeps its `>= Warning`
threshold, so neither option changes anything for a caller who does not use it.
A caller who *had* set `min_severity` and observed nothing will now observe
something — that is the fix, and the CHANGELOG must say so plainly rather than
describing it as new functionality.

**O3 changes default output.** Once sheet diagnostics are counted and rendered:

- `render_summary`'s `diagnostics:` line will report higher counts on any
  workbook that produced a sheet-level diagnostic.
- `render_unified` will grow a diagnostics section where it previously had none.
- `DiffSummary::diagnostics` changes for library callers, with no option set.

**That is the correction, not a regression** — the previous numbers were wrong,
and `AlignmentBoundExceeded` / `DuplicateAlignmentKey` are precisely the
warnings a user needs. But it is a visible change to default behaviour and
belongs under `### Fixed` in the CHANGELOG, named, alongside the `### Added`
entry for the two options. Do not let it ride along unmentioned inside an
"implemented `min_severity`" line.

**`FormulaUnavailable` is pushed per cell** (`src/diff.rs:747`), at
`Severity::Info`. It will now reach `DiffSummary::diagnostics.info` in large
numbers on numeric sheets. It stays out of both renderers — `render_summary`
prints only errors and warnings, and the unified section filters `>= Warning` —
so no output floods. **Confirm that in a test on a numeric fixture** rather than
assuming it; an `Info` count in the thousands with no output change is the
expected result, and a reader of the review request should see the number.

## Known risks

- **`derive_summary` iterates `diagnostics` to build its counters** (`src/model.rs:896`).
  If you filter after `derive_summary` runs, the counters and the vector
  disagree — which is the defect, one layer down. Filter first.
- `docs/src/semantics.md`'s examples execute. If any asserts on a diagnostic
  count, it may move; that is the documentation working.
- The `serde` JSON surface serialises `diagnostics`. A filtered vector changes
  serialised output for a caller who set the option — intended, and worth a
  sentence in the CHANGELOG.

## Required evidence

- Pre-fix failure transcripts, one per test
- CLI runs with and without `--no-warnings`, both formats, showing the counts
  identical and the section present/absent
- A library-level run showing `min_severity` filtering with no renderer involved
- Corpus byte-comparison
- CI run link

## Review request format

Per development policy §9.2. Additionally: state in your own words the
difference between the two options, so I can check that the implementation and
the implementer agree about it.
