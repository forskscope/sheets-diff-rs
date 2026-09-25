# Handoffs — M10: the names become true

**OPEN 2026-09-25.** 2.6.0 is published, so the major is next and nothing is
waiting on a minor.

**Governing:** [RFC-037 (v3 scope)](../../accepted/037-v3-scope.md), accepted
2026-09-25. **§3 of that RFC is closed** — nothing joins this milestone without
the owner reopening it.

M8 documented the lies it could not remove. This milestone removes them. Same
theme one release on: **a public value should describe what the engine does.**

## Queue

| | Unit | RFC-037 | Nature |
|---|---|---|---|
| 01 ✅ | [`SheetMatchReason` says a content check happened](./01-sheetmatchreason.md) | §3.1 | **Removes and replaces public variants** |
| 02 ✅ | [Values nothing produces](./02-values-nothing-produces.md) | §3.2, §3.4 | Removals + documentation |
| 03 ✅ | [The builder's surface, settled once](./03-the-builder-settled-once.md) | §3.7 | Adds two, removes two |
| 04 ✅ | [`DiagnosticLocation` is half-populated](./04-diagnosticlocation.md) | M9 O-A | Changes serialised output |
| 05 ✅ | [What `cells_read` counts](./05-what-cells-read-counts.md) | §3.5 | **Public metric + a security limit; 3 of 80 goldens moved.** |
| — | *blocked* — options that can only fail | §3.3 | needs §7 Q2 |
| — | *blocked* — `AlignmentMode::HeaderColumn` | §3.6 | needs §7 Q3 |
| L | The v2→v3 migration guide | §5.6 | **Last. Every removal needs a row.** |

**Order: 01 ✅, then 02, 03, 04 in any order; 05 before the migration guide;
the migration guide last.**

**All five code units landed 2026-09-25. Only the migration guide remains.**

Unit 05 corrected two of the architect's claims at review: the change **loosens
`max_cells_read` and cannot tighten it** (a box contains every cell in it, so
`populated ≤ area`, so no workbook passed in 2.6.0 and fails now), and **three
of eighty goldens moved**, not every one. RFC-037 §3.5 carries both corrections.

**Units 01–04 landed 2026-09-25; unit 05 is released.** §3.5 could not be
implemented as written — `DiffMetrics::cells_read` and `Limits::max_cells_read`
are one accumulator, so changing the metric changes what `hardened()` bounds.
The owner settled it on 2026-09-25: **both count populated cells.** RFC-037
§3.5 carries the decision and the two options that lost.

**Unit 01 landed 2026-09-25** and found the cause of the whole milestone:
RFC-009 §6 specified a scoring matcher with `sampled_content_similarity` that
was never built, and `SheetMatchReason`'s variants were named after that design
rather than after the code. §6 and §9 of RFC-009 are annotated accordingly —
**the belief now has no source left in the record.** 05 is late because it moves every golden, and the guide
is last because it must describe what actually landed, not what was planned.

## §3.4 — the decisions, recorded (RFC-037 criterion 1)

The RFC listed six never-constructed values "for a decision rather than
prejudged". Here they are, against §2's rule — *does a competent reader learn
the truth from the type?* **Only one is a removal.**

| Value | Decision | Why |
|---|---|---|
| `DiffStage::Open`, `::Normalize`, `::Aggregate` | **Keep, document** | A taxonomy of a pipeline that really has these stages, not a claim about an outcome. Same category as `ExecutionMode`. |
| `DisplaySource::ReaderProvided`, `::ApplicationProvided` | **Keep, document** | **Not dead.** `CellDisplay::new(text, format, source)` is `pub` and `source` is a `pub` field, so these are vocabulary *for callers who supply their own display text*. This crate only ever produces `SheetsDiffDefault`; say that, and say the others are yours to use. |
| `SourceKind::Unknown` | **REMOVE** | `SourceDescription` is an output: the crate builds it, a caller reads it. The three entry points — path, bytes, reader — always know which they are, so `Unknown` cannot arise and a reader matching it writes a dead arm. |
| `ObjectCompareMode::CompareAvailable` | **Keep** | Its own doc already says it behaves identically to `WarnIfPresent` today and why (calamine exposes the APIs; this crate does not call them). An honest reservation. |
| `CellError::Other(String)` | **Keep, document** | A forward-compatible catch-all for an Excel error string we do not recognise. Nothing produces it because we recognise all of them today. |
| `CellValue::Integer`, `::Duration`, `::Unsupported` | **Keep** | M4 unit 01 decided this deliberately, with a distinct reason per variant, and wrote the wording this project reuses. Do not relitigate it. |

**The distinction that does the work**: a value the *engine* would have to produce
and never does is a dead arm (`SourceKind::Unknown`). A value a *caller* may hand
us, or that describes the shape of the pipeline, is vocabulary, and vocabulary is
allowed to be wider than today's usage.

## Standing constraints

- **Every removal needs a row in the migration guide** saying what to use
  instead. That is RFC-037 criterion 2 and the last unit enforces it.
- **`ObjectCompareMode` is the one public enum here that is *not*
  `#[non_exhaustive]`.** Verified. Adding a variant to it is itself a break;
  do not add one casually.
- **No new features.** RFC-037 §4. v3 is a correction release.
- **MSRV stays 1.88** unless something forces otherwise.
- **Every change arrives with a test that fails without it**, demonstrated by
  removing the specific code under test — never by reverting a file. For a
  removal, a compile failure is a legitimate demonstration; say so plainly
  rather than dressing it as a runtime failure.
- Gates as always, plus `.git-exclude/rules/002-comparing-two-builds.md` for any
  before/after measurement.

## What is deliberately not here

- **M9** — fuzzing, rule deviations, record corrections, measurement. Six of its
  seven units are invisible and need no release; they land on `main` as they are
  done. Its seventh is unit 04 above.
- **Anything found after 2026-09-25** joins M9 or a later milestone, not this
  one, unless the owner reopens RFC-037 §3.
