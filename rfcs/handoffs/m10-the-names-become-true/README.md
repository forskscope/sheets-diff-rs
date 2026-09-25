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
| 06 ✅ | [Options a caller cannot usefully set](./06-options-a-caller-cannot-set.md) | §3.3, §3.6 | Removes settings, a field, a mode |
| 07 ✅ | [Values that never arrive](./07-values-that-never-arrive.md) | §3.8 | **Removes a field from serialised output** |
| 08 ✅ | [The options tree becomes extensible](./08-options-become-extensible.md) | §3.9 | **`#[non_exhaustive]`; every later option is additive** |
| 09 ✅ | [One error type, and two more structs](./09-one-error-type-and-two-more-structs.md) | §3.10, §3.11 | **Signature change; two more structs** |
| 10 ✅ | [The v2→v3 migration guide](./10-the-v2-to-v3-migration-guide.md) | §5.6 | **Last. Every removal needs a row (criterion 2).** |

**M10 IS COMPLETE — ten units, all landed 2026-09-25.** RFC-037's
implementation is done and 3.0.0 can be prepared.

**One guard is weaker than three of these handoffs claimed.**
`compile_fail,EXXXX` does **not** enforce the error code on stable rustdoc —
only on nightly — so the thirteen `compile_fail` blocks across units 08, 09 and
10 pass if the example fails to compile for *any* reason, including a typo.
Verified both ways at review. The pins are correct today (checked on nightly);
making them enforced is one step in CI, which already installs nightly for
`fuzz-smoke`. **Routed to M9's CI unit.**

**Two things it must carry, beyond a row per removal:**
1. **`..Default::default()` does not work** on a `#[non_exhaustive]` struct from
   another crate (`E0639`) — and the pattern that stopped compiling is one the
   API guide documented and a compatibility test pinned. The migration path is
   the builder, or `Default::default()` plus field assignment.
2. **"New fields are additive" needs a qualification for types deriving `Ord`.**
   `CellAddress` derives it over `row, col, a1`, so a future field extends that
   lexicographic order — additive *for compilation*, not behaviourally neutral.

**Units 01–08 landed 2026-09-25. Unit 09 was added the same day** by an API
audit (`.git-exclude/decisions/002-v3-api-audit.md`), which found that
`to_json`/`to_json_pretty` return a `Result` that cannot be `Err` — the only
`String` error in the crate — and that two more public structs are extensible
only by a break.

**Order was: 01–05, then 06 and 07 in either order, then 08, then the migration
guide.** 08 is last of the code units because it must see the final set of
option fields, and the guide is last because it must describe what landed.

**Units 01–05 landed 2026-09-25. Three more units were added the same day**,
after the owner reviewed the open questions: §3.3 and §3.6 were decided (unit
06), and **§3.8 and §3.9 were added to RFC-037 §3 in a single reopening**, which
was then closed again.

**§3.9 is the largest item in the milestone and was not on anyone's list.** All
eight public *model* structs are `#[non_exhaustive]`; none of the eight *options*
structs is, and every field is `pub` — so every option this crate ever adds is a
breaking change while every result field is additive. Found by checking whether
§3.3's removal was reversible.

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
- **`fuzz/` is a separate crate that no local gate compiles.** Any removal must
  sweep it and run `cargo check --manifest-path fuzz/Cargo.toml --bins` before
  the push — `.git-exclude/rules/003-where-a-removal-must-be-swept.md`. Unit 06
  did not, and `main` went red on CI's `fuzz-smoke` leg.

## What is deliberately not here

- **M9** — fuzzing, rule deviations, record corrections, measurement. Six of its
  seven units are invisible and need no release; they land on `main` as they are
  done. Its seventh is unit 04 above.
- **Anything found after 2026-09-25** joins M9 or a later milestone, not this
  one, unless the owner reopens RFC-037 §3.
