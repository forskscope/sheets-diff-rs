# `sheets-diff` Roadmap

**Baseline:** 2.2.3
**Agreed:** 2026-08-15
**Status:** active planning baseline

This roadmap was agreed jointly by the project owner and the architect role
after an architecture review of 2.2.3. It supersedes no earlier roadmap — the
v2 planning package it follows from is now restored under [`rfcs/`](./rfcs/).

---

## 1. Objective

> Make `sheets-diff` a library ForskScope can re-enable and trust: verifiable
> builds, a defensible dependency and resource-safety posture, and correct
> answers where it currently returns silently wrong ones — without expanding
> scope.

## 1a. Objective status — met 2026-09-02

**ForskScope is adopting, on 2.5.0.** Their letter of 2026-09-02
(`.git-exclude/tmp/SEND-sheets-diff-2026-09-02.md`) confirms `.xlsx` comparison
is being re-enabled: they re-verified against 2.5.0 rather than carrying their
2.3.0 result forward — `calamine 0.36.1`, `quick-xml 0.41.0`, `zip 8.6.0`,
`cargo audit` clean over 32 crates — and record that the reason for the July
suspension is gone.

They also **retracted their earlier answer that cancellation was not
load-bearing**, which was true only while their `.xlsx` path was disabled.
Nothing had been deprioritised on it: their instruction was to ship it with the
milestone, and it is in 2.5.0.

The objective in §1 was *"make `sheets-diff` a library ForskScope can re-enable
and trust."* The re-enabling is scheduled and no longer waiting on anything of
ours.

**Open:** one design question from their §4 — whether to expose a result shaped
like their internal `SpreadsheetDiff`. Investigation and recommendation (decline,
on grounds that serve them) in
`.git-exclude/tmp/sheets-diff-to-forskscope-adoption-2026-09.md`. It is an
API-surface decision, so it is the owner's, and a yes would need an RFC first.

## 2. Situation

ForskScope — the only known consumer — has **disabled `.xlsx` comparison** and
removed the parser dependency, over `RUSTSEC-2026-0194` / `RUSTSEC-2026-0195`
in `quick-xml` 0.39.4, reachable through `calamine` 0.35. Re-enabling is a
dependency-policy change on their side requiring audit evidence, not a version
bump.

The v2 **design** is sound and externally validated; ForskScope explicitly
endorses the API boundary. The gap is **verification**: the project has no CI,
a test suite that rewrites its own fixtures, a golden corpus nothing reads, and
a published feature that has never compiled. Almost every finding is a claim
made without evidence rather than a decision made badly.

Consequently this roadmap is mostly *finish and prove*, and its RFC work is
mostly **amendment of existing RFCs** rather than new design.

## 3. Themes

| | Theme | Rationale |
|---|---|---|
| **T1** | Assurance — CI, feature matrix, fixture integrity, working goldens | Nothing else can be trusted until "done" carries evidence |
| **T2** | Supply chain & resource safety — MSRV, calamine 0.36, audit gates, bounds, threat model | The re-enablement blocker, plus the surface their gate cannot see |
| **T3** | Correctness — silent wrong answers | Integrity of the diff is a security property (§6) |
| **T4** | Traceability & docs — RFC-033, status verification, NF-024/026/027 | Closes the governance gap and unmet MUST requirements |

## 4. Milestones

### M1 — Trustworthy build *(no release)* — ✅ **CLOSED 2026-08-15**

Governed by **RFC-034** ([implemented](./rfcs/done/034-build-assurance-and-fixture-integrity.md)).
All exit criteria met and evidenced; see
`.git-exclude/reviewed/034-handoff-02-ci-pipeline-deliberate-failures/`.
CI is green end to end
([run 31888729981](https://github.com/forskscope/sheets-diff-rs/actions/runs/31888729981),
17/17) and every guard has been observed **both green and red**.
[PR #7](https://github.com/forskscope/sheets-diff-rs/pull/7) is **merged**;
`main` is at `5ed0644` with CI green 17/17.

Exit criteria:

- CI green on Linux + one further OS (NF-023), across every feature combination
- `cargo clippy --all-targets` clean and gated
- MSRV verified by a job building at the declared floor
- `cargo test` leaves the working tree clean
- Golden fixtures are asserted against and can fail
- Previously `#[ignore]`d large-workbook tests either run or are gated explicitly

No release is cut until M1 is green. Publishing on an unverified build is what
produced the 2.2.0 phantom feature.

### M2 — 2.3.0, "trustworthy results and a defensible posture" — ✅ **COMPLETE 2026-08-16**

Governed by **RFC-035** ([accepted](./rfcs/accepted/035-resource-safety-and-supply-chain-governance.md))
for the new policy decisions, plus existing RFCs for the dependency migration
and the correctness defects. Execution queue:
[`rfcs/handoffs/035-…/README.md`](./rfcs/handoffs/035-resource-safety-and-supply-chain-governance/README.md).
All six units are approved. Units 01–02 are merged (`main` at `059ad6f`);
units 03–06 are on PR #10 at 18/18. Scope delivered:

- MSRV 1.85.0 → **1.88**, *then* `calamine` 0.35 → 0.36 (ordering is load-bearing, §5).
  **The bump is not a version-string edit.** Three files move together — the
  `msrv` job's drift guard enforces `Cargo.toml`, `env.MSRV` and the toolchain
  pin agreeing — and raising the floor to 1.88 newly surfaces **9 clippy
  findings** that do not exist at 1.85 — 8 × `collapsible_if` (`src/align.rs`,
  `src/diff.rs` ×3, `src/matcher.rs`, `src/meta.rs` ×2, `src/output/view.rs`)
  and 1 × `manual_is_multiple_of` (`benches/workbook_diff.rs:52`). Clippy gates
  suggestions on the declared MSRV. Independently reproduced: 0 findings at
  1.85.0, 9 at 1.88.0. The ninth is easy to miss because clippy's summary line
  counts the lib target only, while `lint` runs `--all-targets`. With `lint` now a hard gate these must be fixed, or deferred with a
  recorded reason, in the same change. Discovered during M1's deliberate-failure
  demonstrations.
- `deny.toml`, `cargo audit`, dependency-path assertions in CI
- Resource bounds: product-bounded alignment with positional fallback; input size bound
- `#![forbid(unsafe_code)]`
- Threat model (§6) and advisory-response policy
- **Integrity-affecting correctness defects** — `DateTimeIso`/`DurationIso`
  comparison, alignment coordinate collision, formula-range origin
- CHANGELOG corrections for 2.2.0 and 2.2.3

Exit: ForskScope's dependency gate passes **and** our own resource bounds hold.
The gate alone is not sufficient — see R1.

**Exit status, settled 2026-08-16 after their reply.** The second half is
demonstrated. The first half was **mis-specified by the architect** and is
recorded honestly rather than declared closed:

- **Advisory-based verification: passed, independently.** They resolved 2.3.0
  from a scratch project — `quick-xml` 0.41.0, `cargo audit` exit 0 — and
  confirmed the same inside their own tree. `quick-xml` 0.39.4 survives there
  only via `wayland-scanner`, a codegen path carrying no workbook XML, never
  through our chain.
- **Their dependency-path gate rejects `sheets-diff` by name.** That is their
  fail-closed policy from July, not a verdict on 2.3.0 — they removed the
  dependency outright rather than pinning it. Re-adoption is blocked by a switch
  they control.
- **Runtime verification: not performed.** The code path is still disabled on
  their side.

The criterion made our milestone's closure depend on a third party's business
decision, on their timeline, for their reasons. That was the wrong instrument.
What it was reaching for — *independent confirmation that the chain is cleared* —
is satisfied. Recorded as **met on the dimension it could measure**, with the
mis-specification named.

**Both halves met, plus the integrity work §6 folded in.** All six units
approved; PR #10 carries 03–06 at 18/18. `RUSTSEC-2026-0194`/`-0195` are out of
the tree, `align.rs` can no longer exhaust memory, and the engine no longer
reports "identical" for cells that differ. **R1 is closed on every axis it
named.** Remaining owner decisions: merge PR #10, release 2.3.0, then notify
ForskScope — in that order, since the notification is only honest once the
release exists.

### M3 — "Real files, and a record that is true" — ✅ **COMPLETE 2026-08-16** *(no release)*

*Status corrected 2026-08-17. This line said **OPEN** until then, and had done
since the milestone actually finished — verified now rather than assumed: the
corpus is 19 scenarios asserted by `tests/integration.rs`, `rfcs/done/033-…` is
reconstructed and present, and all 30 RFC statuses were verified in track D. See
the closing note at the end of this section.*

Agreed jointly 2026-08-16 from
`.git-exclude/tmp/m3-planning-proposal.md`. Everything here is gated by nothing
external. **No release** — nothing user-observable lands.

| Unit | Item | Governing RFC |
|---|---|---|
| A/01 | Coverage-dimension report — **approved**; produced RFC-036 and three findings | RFC-030 |
| A/02 | Generate the nine `rust_xlsxwriter`-producible scenarios | RFC-036 |
| A/03 | The two scenarios needing `patch_xlsx_xml` | RFC-036 |
| B | Reconstruct RFC-033 from the 20 sites citing it | — |
| D | Verify the 30 RFC statuses; correct the three already known wrong | RFC 000 |

Serial: A before B. **Exit:** the matrix runs in CI with every dimension covered
or explicitly deferred; RFC-033 exists; every RFC status is verified or
corrected.

### M4 — "The code contradicts itself" — ✅ **COMPLETE 2026-08-17** *(2.4.0)*

*Status corrected 2026-08-17. This line said **OPEN** until then. All four units
are merged and 2.4.0 is tagged and published.*

Agreed 2026-08-17 from `.git-exclude/tmp/m4-boundary-proposal.md`. M3's thirteen
findings plus the earlier M4 sketch were regrouped by **what each item changes
for a consumer**, which is what decides the release. Twelve schedulable items
across four milestones; two are upstream-blocked and one is the v3 decision.

Four units, one defect class: **a statement that sounds like a fact because it
sits next to code.** This project has been bitten by it three times — the
`parallel` feature that never compiled, `cells_compared`'s changelog claim, and
`meta.rs`'s comments. Unit 04 makes it five, and moves the class from comments
into the safety claims themselves.

| Unit | Item |
|---|---|
| 01 | Two doc-only truths: `meta.rs`'s comments describing a `WorkbookMetadataMode` that was never built, and the absence of any note that `Integer`/`Duration`/`Unsupported` are unreachable |
| 02 | `DiffMetrics.cells_compared` counts changed cells, not coordinates visited — and 2.2.3's changelog claimed it fixed exactly this |
| 03 | Exit code 3 for invalid/corrupt input is specified by RFC-013 and never emitted, plus the subprocess test that has never existed |
| 04 | The safety claims: `max_cells_compared` bounds diffs rather than coordinates (F-A), the threat model tells callers it bounds coordinates (F-E), and a disk error mid-read is classified as a corrupt file (F-F) |

**Unit 04 was added 2026-08-17, opened on the owner's ruling.** It was not in
the agreed set. Units 02 and 03 each surfaced it: reviewing the metric defect
exposed the same root cause in a resource limit, and reviewing the exit-code
mapping exposed the classifier feeding it. F-E is the one that decided the
priority — a threat model that promises a bound we do not provide is worse than
a limit that silently does not fire, because a consumer can read it and plan
around it.

**The owner ruled on its compatibility consequence.** `Limits::hardened()` sets
`max_cells_compared: Some(5_000_000)`; today that bounds diffs, so a hardened
caller comparing a large workbook with few differences never trips it, and
after unit 04 they will. A comparison that succeeded in 2.3.0 can return
`LimitExceeded` in 2.4.0. `Limits::default()` leaves the limit unset, so
default-configured callers are unaffected. Accepted deliberately: the limit
doing what it was always documented to do is the point of the fix.

**Released as 2.4.0, not 2.3.1 — a correction to the proposal.** I called this a
patch release. Unit 03 falsifies that: corrupt input currently exits 2, and
moving it to 3 changes an observable CLI contract. A consumer matching `2` for
"operational error" would see `3` for a subset. Exit codes are an interface, so
this is a minor at minimum.

**M5 shrinks slightly**: unit 03 carries the CLI subprocess test, which was one
of M5's three items, because implementing an exit code without testing it would
be the same defect class M4 exists to remove.

### M5 — "Nothing checks it" — ✅ **COMPLETE 2026-08-17** *(no release)*

Four units, one property: a rule this project states and nothing verifies.
Handoffs: [`rfcs/handoffs/m5-nothing-checks-it/`](rfcs/handoffs/m5-nothing-checks-it/README.md).

| Unit | Item |
|---|---|
| 01 | The stdout/stderr prohibition is enforced by nothing — four RFCs state it, no CI step checks it | ✅ |
| 02 | Source-path privacy is untested: non-UTF-8 handling and `display_name` semantics | ✅ |
| 03 | Encrypted-workbook detection has zero coverage, and no fixture exists | ✅ |
| 04 | Unit 01's gate can be waived with `#[allow]` — which its own error message advertises | ✅ |

**Unit 04 was added mid-milestone.** Unit 01's review found the gate stopped an
accident but not a decision: clippy's failure message prints the bypass
verbatim. `#![forbid(...)]` makes the override `error[E0453]`, and it fires in
every clippy invocation rather than only the scoped gate. My scoping failure —
unit 01 forbade touching `src/`, and the fix is a crate-level attribute.

**Nothing under `src/` changed in any of the four.** Every property held before
the milestone; now something objects if it stops. RFC-016 moved from *Partially
implemented* to *Implemented* (both deferrals closed), and RFC-032 likewise.

**Correction to this entry, 2026-08-17.** It previously attributed the
stdout/stderr rule to **NF-015**. That is wrong: NF-015 is *no network*, and it
has been a build-time property since M2 via `deny.toml` bans. The stdout
prohibition is RFC-016 §"Guarantee no stdout/stderr writes from library core",
restated by RFC-005, RFC-013 line 82 and RFC-032. Caught while writing the
handoffs; recorded rather than quietly corrected, since handing the dev team a
unit citing the wrong requirement is the defect class M4 just closed.

**A second finding from the same reading.**
`rfcs/done/1.2/006-regression-fixture-and-ci-hardening.md` claims v1.2 delivered
*"the CI stdout-hygiene check."* It did not — at tag `1.2.0` the only workflow
is `release-executable.yaml`, and `git grep stdout` across that whole tree
returns documentation only. An RFC in `done/` records a check that was never
built, and the rule has been unenforced since. Unit 01 annotates it, because
unit 01 is what makes the sentence true.

Source is currently clean and detection currently works: **M5 is prevention, not
remediation.**

### M6 — "The documentation MUSTs" — ✅ **COMPLETE 2026-08-17** *(2.4.1)*

Five units. Handoffs:
[`rfcs/handoffs/m6-documentation-musts/`](rfcs/handoffs/m6-documentation-musts/README.md).

**The requirements, quoted rather than paraphrased** — from
`.git-exclude/specs/sheets-diff-v2-requirements.md`:

| | Level | Text |
|---|---|---|
| NF-024 | **MUST** | Document the v2 API with examples for path, reader, bytes, options, and formatter usage. |
| NF-025 | **MUST** | Document migration from v1/v1.2 to v2. |
| NF-026 | **MUST** | Document non-goals and limitations clearly. |
| NF-027 | SHOULD | Document comparison semantics with examples: typed value change, formula change, sheet rename, inserted row, and warning handling. |

**Noted while opening this milestone: the requirements register lives outside
the tracked repository.** `NF-024`, `NF-026` and `NF-027` appear nowhere in
`rfcs/`, `docs/`, or any tracked file — only in this roadmap and in
`.git-exclude/specs/`. So the normative requirements this project is measured
against are not under version control, and every citation of an NF number in a
tracked file is unresolvable to anyone reading the repository alone. Not
scheduled here — it is a governance question for the owner, not a documentation
unit — but recorded rather than left to be rediscovered.

| Unit | Item |
|---|---|
| 01 | Make documentation checkable — the migration guide has 11 Rust blocks nothing compiles |
| 02 | The API guide (**NF-024**) — five example categories, none of which exist |
| 03 | Semantics and non-goals (**NF-026**, NF-027) — neither page exists |
| 04 | The undocumented public surface — F-D, F-G |
| 05 | Record corrections — F-I, F-L, and two RFC statuses M5 made stale |

**01 comes first and is not optional.** 02 and 03 write the examples NF-024 and
NF-027 name; without a harness they would ship unverified, which is the defect
class M4 and M5 spent eight units removing, manufactured at scale in the release
meant to fix the documentation. The mechanism is verified — a markdown file
pulled in via `#[doc = include_str!]` behind `#[cfg(doctest)]` turns its ```rust
blocks into doctests, and a broken block fails `cargo test --doc`.

**Exit: NF-024 and NF-026 met, NF-027 addressed, and then the v3 question.**

**All five units are merged** (`main` at `f15ce45`, 18/18, 24 doctests where
there were 3 before this milestone). **Both MUSTs are met for the first time
since v2.0.0.** NF-027's five examples do not merely compile — they execute
against committed corpus fixtures and assert on real output, so every
documented claim is re-checked on every push. The `msrv` job now compiles
doctests at the 1.88 floor.

Unit 03 found five contradictions between its inventory and the existing
record, two of them against the architect's own writing, including a count
(*"thirteen partially-implemented RFCs"*) that was eleven. All five were
verified and corrected; correcting two of them closed RFC-013 and RFC-015
outright, which moved the count to **nine** and made the new page's own table
wrong within the hour. Reconciled, with the movement left visible on the page.

**Both closing items are settled.**

1. **Released as 2.4.1.** Documentation and CI only — no API addition (the four
   doctest-harness structs are `#[cfg(doctest)]`-guarded and absent from every
   normal build), no behaviour change.
2. **The v3 question: not scheduled**, decided by the owner 2026-08-17 on
   `.git-exclude/tmp/v3-decision.md`. To be reconsidered after M7, when
   measurement may supply a real breaking requirement rather than an aesthetic
   one. The inventory that informs it lives at
   [`docs/src/non-goals.md`](docs/src/non-goals.md) and is now maintained as a
   consumer-facing page rather than as planning notes.

   **Still open from that paper, and not blocking:** whether the four
   available-but-uncompared object types (hyperlinks, merged regions, tables,
   pivot tables — calamine 0.36 exposes all four) should be scheduled, recorded
   as a permanent non-goal, or left until ForskScope asks; and whether
   `.git-exclude/specs/sheets-diff-v2-requirements.md` should be tracked, since
   every NF citation in `rfcs/` currently resolves to nothing for a reader of
   the repository alone.

Also **`DiffMetrics`'s undocumented fields** (F-D, raised in M4 unit 02's
review). M4 unit 02 documented `cells_compared`, leaving it the only documented
field of five. The gap that matters is `cells_read`: on the `sparse_range`
fixture it reads 5200 against 2 compared, because it counts every physically
visited cell including empties — a 2600× difference no consumer would predict
from the name. *(Historical: fixed in 3.0.0 by M10 unit 05, which made both
`cells_read` and `max_cells_read` count populated cells. `sparse_range` now
reads 4.)*

Also **`ReadErrorKind`'s undocumented variants** (F-G, raised in M4 unit 04's
review). No variant carries a doc comment, and after M4 `Other` is a public
variant that nothing can produce — the reader is `Xlsx<Cursor<Vec<u8>>>`, so the
`XlsxError::Io` that would map to it cannot arise at read time — yet it has its
own exit-code arm. That is unit 01's defect class exactly, and unit 01 already
established the wording for it. Retained deliberately as a conservative default;
the point is that nothing says so.

Also **the corpus count in 2.4.0's changelog is wrong** (F-I, found 2026-08-17
while drafting the ForskScope message). The M3 entry reads *"the fixture corpus
grew from 7 to 18 scenarios"*. Seven scenarios predate M3 and twelve were added
in `2679870`, so it grew from 7 to **19**. The entry was wrong when written and
shipped in 2.4.0 — the truth-telling release. **Annotate, do not rewrite**, per
the convention this file already applies three times over. Worth noting that it
was caught by needing to state the number to someone outside the project, which
is a check nothing in CI performs.

### M7 — "Measure, then change" — ✅ **COMPLETE 2026-08-17** *(2.5.0, published)*

Handoffs: [`rfcs/handoffs/m7-measure-then-change/`](rfcs/handoffs/m7-measure-then-change/README.md).

| Unit | Item | Status |
|---|---|---|
| 01 | Measure — peak allocation, its attribution, dense-vs-sparse cost, cancellation latency | Ready; **gates units 03+** |
| 02 | The v1.2-vs-v2 comparison (RFC-027) | Ready; independent, gates nothing |
| 03 | Cancellation that fires — RFC-012's requirement met for the first time | ✅ |
| 04 | Delete the alignment clone — +33% of peak, now 0.0% | ✅ |

**All four merged** (`main` at `7aa4c94`, 18/18). **Two candidates declined on
their measurements** and recorded with their numbers: RFC-024 §7's density
choice (+12.4%) and `compare_bytes`'s copy (+2.6–4.8%).

**What the measurement changed.** Three of unit 01's four answers differed from
what reading the code suggested, and the record was wrong in two places that
mattered:

- `compare_bytes` does **not** double peak memory — 2.6–4.8%. The threat model
  said doubling, we told ForskScope twice, and it was inferred from reading the
  code. Corrected.
- Cancellation was **not** a latency problem. It was structurally unobservable
  on any single-sheet workbook — the ordinary shape of a spreadsheet — and both
  `Cancellation`'s doc comment and RFC-024's Status had described it as
  granularity for four releases.
- The alignment clone was the only large win, and it was **deletable rather
  than reducible**: alignment only ever calls `display_string()` on the values
  it is handed. Delta measured at 32.7–33.9%, now **0.0%** at both scales, with
  `Positional` peak byte-identical as the control.

One of three built, and not the one intuition or the threat model pointed at.
That is the milestone succeeding.

**Units 03 onward are deliberately unwritten.** Three of M7's four candidate
items share the property that we know a cost exists and not what it is:
`compare_bytes`'s copy is called a doubling in the threat model on the strength
of reading the code; `cell_map_to_align` clones every `CellValue` in both
sheets and nobody knows what fraction of peak that is; and cancellation's
polling granularity is called a gap by RFC-024's status while RFC-012's own
goal — *"cancellation checks at major pipeline stages"* — arguably describes
what we already do. That last disagreement cannot be settled by reading either
document, and is settled in one measurement by timing how long a caller waits.

Unit 01 changes no library code and optimises nothing. Its report's candidates
section is what units 03+ get scoped from. The mechanism is verified: a
tracking global allocator works in a bench target, since `#![forbid(unsafe_code)]`
binds the library crate and not `benches/`.

Large-workbook memory; cancellation granularity (polled per sheet pair, not per
cell batch as RFC-012 specifies); the shared display address; and **the
v1.2-vs-v2 benchmark comparison (RFC-027), moved here from M6** — it was grouped
as documentation debt, but it requires building v1.2 and running comparable
benchmarks, which is measurement, not writing. These share the
property that **their scope cannot honestly be written until something is
measured**, so they are grouped to keep that discipline in one place.

### M8 — "The surface promises what the engine does not" — ✅ **COMPLETE 2026-09-25** *(2.6.0)*

2.5.1 is published, so this is open. Handoffs: [`rfcs/handoffs/m8-the-surface/`](rfcs/handoffs/m8-the-surface/README.md).

From dev-team task 001's readiness review, plus the `cells_read` question
ForskScope left open. **One theme:** every item is a public surface a competent
reader predicts wrongly — the owner's second design principle, applied to the
things this crate advertises.

Needs a **minor** release: A1 changes a CLI exit contract (the same reason M4
shipped as 2.4.0 rather than 2.3.1) and A6 changes a public metric.

| Unit | Item | Why it is not a patch |
|---|---|---|
| 00 ✅ | **R1 — `Limits::hardened()` promises "a guarantee that no workbook — hostile or merely huge — can demand unbounded time or memory."** The threat model names two exceptions: a zip bomb within the size bound is capped only by `zip`'s own decompression, and styled blank records cost time and are not counted by `max_cells_read`. True about the struct — all six fields are set — and false about the world. **The second time this function has overpromised in this exact way**; M4's F-E was the same move one clause in. Found by the 2.5.1 release sweep. | Documentation |
| 01 ✅ | **A1 — a reorder reports no difference, and a rename renders none.** Reorder: exit `0`, `--format unified` emits only its two header lines. **Pure rename: exit `1` but unified renders nothing either** — the exit code and the renderer contradict each other in one invocation. Found while scoping; `render_unified`'s guard drops any sheet without cell changes unless it is `Added`/`Removed`, so its own `[renamed: …]` branch can only run when the sheet *also* has cell changes. Both reproduced. | Exit-code contract |
| 02 ✅ | **A2 + A3 — two inert options.** `--no-warnings` is in `--help` and never read; `DiagnosticOptions::min_severity` is public, documented, and never read. **Plus O3, found scoping the handoff 2026-09-24 and the most consequential of the three: sheet-level diagnostics reach no output at all.** `derive_summary` (`src/model.rs:864`) counts only the workbook-level vector, `render_unified` renders only that vector, and `metrics.diagnostics_emitted` (`src/diff.rs:302`) sums both — so the metric and the summary disagree. The casualties are `AlignmentBoundExceeded` (the sheet silently fell back to positional comparison) and `DuplicateAlignmentKey` (rows may have been paired wrongly), **both `Severity::Warning`, both invisible in every CLI output.** A warning whose job is to say the diff may be wrong, that nothing prints, is unit 01's failure in a second place. | Behaviour appears where there was none; **default diagnostic counts change** |
| 03 ✅ | **A5 — no `--format json`**, though RFC-013 specifies it and `src/output/json.rs` already provides `to_json`/`to_json_pretty`. RFC-013's Status says `Implemented` and records only the exit-code deferral. | New CLI surface |
| ~~04~~ | **WITHDRAWN to RFC-037 (v3) 2026-09-25.** **A4 — four `DiagnosticKind` variants nothing constructs** (`FormulaCachedValueUnverified`, `UnsupportedCellValue`, `DateTimeNotNormalized`, `LimitTruncatedCells`), each live in the stable `code()` table callers are told to match on. `LimitTruncatedCells` cannot occur: limits return `Err`. **Plus O1, and O1 is worse than reported: `SheetMatchReason` has three variants and only one is ever constructed.** `ExactName` is structurally impossible — the enum appears only inside `Renamed`/`RenamedAndMoved`. `ContentSimilarity` is never produced because the matcher never inspects content. `IndexAndContent` is produced for *every* rename and is inaccurate at all three sites, worst at `matcher.rs:151` where the pair is formed by elimination — neither index nor content. Found by the implementer during unit 01, folded here 2026-09-24. | Documentation only |
| 06 | **The CLI you can actually install — two defects that compound, found reviewing unit 03.** `cargo install sheets-diff` installs **nothing** (`default = []`, the bin needs `cli`), and **neither `README.md` nor `docs/` contains the string `cargo install`** — a tool documented since 2.0.0 with no stated way to get it. Meanwhile `cli` enables `serde` but not `chrono`, so the installable binary emits `"iso": null` for every datetime. The command users will be told to run is exactly the build that is wrong. **Must ship in 2.6.0 with `--format json`, not after it.** | Feature set of the shipped binary |
| 07 | **O-D — the builder omits two options.** `DiffOptions` documents `builder()` as the construction entry point; `DiffOptionsBuilder` has 19 setters and none for `diagnostics.min_severity` (which unit 02 just gave behaviour) or `matching.alignment` (whose warnings unit 02 just made visible) — while its sibling `matching.sheet_matching` has one. | Additive API |
| ~~05~~ | **WITHDRAWN to RFC-037 (v3) 2026-09-25**, done as M10 unit 05. **A6 — `cells_read` meant bounding-box area**, reporting 5,200 against 2 compared cells on `sparse_range`. | Public metric; **3 of 80 goldens moved**, not every one |

**M8 now closes at unit 07.** Units 04 and 05 moved to
[RFC-037](rfcs/accepted/037-v3-scope.md) on 2026-09-25: both are fixed by
*removing or renaming* public items, which cargo's resolver makes a major-version
change, and the owner's decision was to stop deferring those behind a
documentation workaround. A major costs almost nothing while there is no
production use, and this is the cheapest it will ever be.

**Dispositions I am proposing, not questions:**

- **A1 is a defect, not a rule to write down.** The library already models a
  reorder — `SheetChange::Moved`, `DiffSummary::sheets_moved`, `[moved]` in the
  summary, `sheet_reordered` in the corpus. The engine says the workbooks differ
  and the CLI says "no differences found". ROADMAP §6 puts a missed difference at
  the same severity as a crash.
- **A2 and A3 are two mechanisms, not one. — corrected 2026-09-24.** The
  disposition above originally read *"make `--no-warnings` its CLI face"*;
  reading the code to write the handoff showed that wrong. `min_severity` is a
  **collection** filter and `--no-warnings` is a **display** control. Wiring the
  flag to the field would make `--no-warnings` change `DiffSummary`'s counters,
  so `render_summary` would report zero warnings for a workbook that had them —
  a flag that suppresses a display and thereby falsifies a count, which is a
  worse instance of what M8 exists to fix. Implement both, separately.
  A third severity control already has teeth and is not configurable:
  `src/output/text.rs:164` hardcodes `>= Warning`.
- **A4 and O1: document, do not remove.** `DiagnosticKind` is
  `#[non_exhaustive]`, but removing a variant still breaks a matcher, so removal
  is a v3 question. M4 unit 01 established the wording for unreachable variants;
  reuse it. **O1 is the same defect inverted** — A4 is a value never produced,
  O1 a value produced under a false name — and the reader cannot tell the two
  apart, so they belong in one unit. Renaming `IndexAndContent` is a v3 question
  for the same reason; the 2.x answer is a doc comment that says what the
  matcher actually does, and RFC-032's record corrected to match.
- **A5: build it.** The library half exists; this is wiring plus a record
  correction to RFC-013. **Watch the feature trap:** `cli = ["dep:clap"]` does
  not enable `serde`, and `to_json_pretty` is `serde`-gated, so the naive
  implementation gives a binary whose `--help` differs by build at the same
  version — a fresh instance of M8's own theme. `cli` must imply `serde`.
- **O3: fix it in unit 02**, not separately. Filtering `min_severity` without
  noticing that half the diagnostics never reach a renderer would be a half-fix,
  and the filter has to cover both vectors anyway.
- **A6: decide, then build.** Counting populated cells is what the name promises.

### f130 — "`RowKey` drops rows with a blank key" — ✅ **FIXED 2026-09-26** *(3.1.0, minor)*

**Ahead of M9.** Defect response, not a milestone unit. **Merged `b49dfd0`;
prep open at [`rfcs/handoffs/release-3.1.0/`](rfcs/handoffs/release-3.1.0/01-release-preparation.md).**
A minor, not a patch: the fix needs a new `DiagnosticKind` and `code()`'s own
documentation promises new variants in a minor. Handoff:
[`rfcs/handoffs/f130-rowkey-drops-blank-key-rows/`](rfcs/handoffs/f130-rowkey-drops-blank-key-rows/01-blank-key-rows-are-never-compared.md).

Reported by ForskScope 2026-09-26 and **reproduced here, worse than reported**.
With `AlignmentMode::RowKey`, a row with no cell in the key column never enters
`extract_row_keys`'s map, so `lcs_match` never sees it: it is in none of
`matched`, `removed` or `inserted`, and **its cells are never compared**. A
change in such a row is reported as no change.

And the result does not merely omit it — `alignment_summary` reports
`confidence: Exact` with no diagnostic, a positive claim of exact matching made
about rows that were not looked at.

Their shape is the ordinary one: 2,000 rows, a unique `ID` blank in ~5% of them
(subtotals, spacers, notes); 1,901 of 2,001 rows matched and 100 silently
absent. **Same class as M8's reorder defect** — the engine has the information,
the answer says there is no difference, and nothing objects. ROADMAP §6 rates
that with a crash.

**The fix is to stop lying, not to align better:** keyless rows become
`removed`/`inserted` so their cells reach the comparison, a `Warning` names the
per-side count, and `confidence` stops claiming `Exact`. Pairing keyless rows
positionally — ForskScope's suggestion, and better — is a quality change that
needs a design and follows separately.

### Alignment follow-ups — 🔄 **RECORDED 2026-09-26** *(no release yet)*

From ForskScope's measurements of 2026-09-26. Recorded so the answers given in
our reply are scheduled rather than promised.

| Item | State |
|---|---|
| **Alignment is not cancellable.** `grep -c check_cancel src/align.rs` → 0. 2.5.1 made the *read* cancellable and left this phase; a cancel requested 100 ms into a 1.2 s alignment is observed at 1,208 ms. **Accepted** in the reply; the LCS loop is the natural place. | To schedule |
| **Keyless rows compared positionally against their neighbours**, rather than reported as removed/inserted. ForskScope's suggestion; the quality half of f130, which ships correctness only. Needs a design: which neighbour, and what happens when the per-side counts differ. | To design |
| **O((m+n)·D) alignment** in place of O(m·n) LCS. Internal only — same `RowMapping` out, no API surface. Their figures: 3.7 bytes and 48 ns per `m × n` cell; `max_alignment_product` (25M) bites at ~5,000 rows; 10,000 rows is 4.9 s and 465 MB. **Direction accepted, RFC-gated, no date** — degenerates toward O(m·n) at large D, so the bound stays as a guard. | RFC first |
| **Row space on `CellDiff`.** A matched or removed row is numbered in the old sheet's space, an inserted one in the new; `CellDiff` carries an address and nothing saying which. Additive (`#[non_exhaustive]`), so a minor. **The naming is the hard part** — the field means *which file's row numbering this address uses*, not *which file the cell is in*. | RFC first |

### M9 — "Reaching the code, and a record that agrees with itself" — 🔄 **OPEN 2026-09-26** *(no release)*

Handoffs: [`rfcs/handoffs/m9-reaching-the-code/`](rfcs/handoffs/m9-reaching-the-code/README.md).
Authorized 2026-09-24; opened once 3.0.0 shipped and M10 closed.

**Unit 00 was added 2026-09-26** from the project-health audit
(`.git-exclude/decisions/004-where-verification-should-run.md`): `cargo doc`,
nightly doctests, `publish --dry-run`, `cargo public-api` and the install check
are **not in CI at all**, two matrix legs are the same build since `cli` began
implying `serde` and `chrono`, and nothing builds a minimal library at MSRV. It
runs first because it makes every later unit cheaper to verify.

**No longer release-free.** M9 was scoped as fuzzing, rule deviations and record
corrections — none observable. Unit 06 (O-A) changes that: populating
`DiagnosticLocation` at the push sites changes `location.sheet_name` in
serialised output from `null` to a name, which `--format json` now publishes.
That is a minor, and it means M9's units are no longer uniformly invisible —
split the release-affecting ones out at planning time rather than discovering it
at the cut.

| Unit | Item |
|---|---|
| 01 | **D1 — the fuzz corpus cannot reach the sheet reader.** `fuzz_open_xlsx_bytes` is seeded with an empty file, random bytes and a truncated ZIP header; coverage-guided fuzzing must synthesise a valid archive *and* valid workbook XML before a line of read, normalise or compare code runs. **The f123 denial of service lived past that point.** No target sets `Limits` or an alignment mode. |
| 02 | **B1–B4 — deviations from `project-instructions-rust.md`**: inline `#[cfg(test)] mod tests` in five modules, `src/output/mod.rs` against the prescribed 2018 style, eight `#[allow(…)]` against CI's "no silencing" comment. Plus **B3**, which cannot be evaluated: the rule says to split `tests/` by line count and defines no threshold. |
| 03 | **C4, C6, C7, C8, C9, E, and F-2 — the remaining record corrections.** **F-2:** RFC-013 has more drift than M8 unit 03's three corrections covered — its §5 exit-code table says `2 = usage, 3 = input/open/read, 4 = cancelled or limit, 5 = internal` when `4` and `5` are never emitted, and it places the CLI at `src/bin/sheets-diff.rs` when it is `src/main.rs`. Found by the implementer 2026-09-25 and correctly left alone as out of scope. Original items: `fuzz/README.md` says CI does not run the fuzz targets (it does); RFC-035 and RFC-036 are still in `accepted/` while the README calls 035 delivered; `performance.md`'s figures were measured on the dense read PR #28 replaced and nothing says whether they were re-measured; handoff directories are not `NNN-slug/`; `README.md` says calamine is "pinned" where `Cargo.toml` has a caret range. |
| 04 | **O4 — `FormulaUnavailable` is pushed once per cell** (`src/diff.rs:747`), each carrying a cloned sheet-name `String` and a `CellAddress`, for every numeric cell with no formula. On a dense numeric sheet the diagnostics vector grows proportional to cells read, at a much larger per-item cost than a cell. Bounded by `max_cells_read`, so not a hole — but the constant factor is unmeasured and the threat model does not mention it. **Measure before deciding**; M7 established that this project does not act on a memory hypothesis it has not measured. Found 2026-09-24 while scoping M8 unit 02. |
| 05 | **O-B — the corpus cannot reach the warnings M8 unit 02 exists to surface.** Swept independently at review time: 19 scenarios × 4 alignment modes = **76 runs, zero sheet-level warnings**. No fixture trips `AlignmentBoundExceeded` or `DuplicateAlignmentKey`, and RFC-036's coverage matrix has no row for either. **This is D1's shape again** — a corpus that cannot reach the code it exists to guard — and it should be worked beside D1, not apart from it. Found by the implementer 2026-09-25. |
| ~~06~~ | **MOVED to RFC-037 (v3) 2026-09-25** — it changes serialised output, which a major covers. **O-A / F-4 — `DiagnosticLocation` is populated inconsistently.** `align.rs`'s two sites and most of `meta.rs`/`objects.rs`/`matcher.rs` leave `sheet_name` and `sheet_order` `None`; only `FormulaUnavailable` and two others fill them. M8 unit 02 worked around it by naming the sheet from the owning `SheetDiff`, which is right for the renderer — but **a JSON consumer now sees those diagnostics with `"sheet_name": null`**, inside the right sheet's array and unable to say so from the location alone. Fix the push sites. |
| 07 | **F-3 — the CLI applies no `Limits` at all**, and `--format json` builds the entire result as one in-memory `String`. Not new (every format has the shape), but JSON makes output size track result size exactly. **Measure before deciding**, with O4. |

**D1 is the substantive one.** This crate has fuzzing that cannot reach the code
where its worst defect was — the same shape as the golden corpus nothing read.

**O5, for unit 03's E bucket:** `src/output/view.rs` is a `pub mod` that nothing
inside the crate calls — the only in-crate mention is a doc comment in
`model.rs:479`. It is public API rather than dead code, and it is the one place
that reads `SheetDiff::diagnostics`, so it is *not* covered by M8's O3. Decide
deliberately whether it is a supported projection or an accident, and say which
in RFC-033. Found 2026-09-24 while scoping M8 unit 02.

### M10 — "The names become true" — 🔄 **OPEN 2026-09-25** *(3.0.0)*

**This is the next release, not M9.** M9 is authorized and unopened; six of its
seven units are invisible and need no release, so it may run in parallel at any
time. M10 carries 3.0.0.

Executes [RFC-037](rfcs/accepted/037-v3-scope.md), accepted 2026-09-25.
Handoffs: [`rfcs/handoffs/m10-the-names-become-true/`](rfcs/handoffs/m10-the-names-become-true/README.md).

M8 documented the lies it could not remove without a major. This milestone
removes them. **§3 of RFC-037 is a closed list** — nothing joins M10 without the
owner reopening it.

| Unit | Item | Nature |
|---|---|---|
| 01 | **§3.1 — `SheetMatchReason`.** Two of three variants never constructed, `ExactName` structurally impossible; the third is recorded on every rename and is wrong at all three sites, worst where the pair is formed by elimination. | Removes and replaces public variants |
| 02 | **§3.2 + §3.4 — values nothing produces.** Four `DiagnosticKind` variants, all live in the stable `code()` table callers are told to match on; plus `SourceKind::Unknown`. | Removals + documentation |
| 03 | **§3.7 — the builder's surface, settled once.** Adds `date_compare` and `max_cells_read`; removes `number_compare` (duplicate) and `build_with_matching` (discards `sheet_matching` silently). | Adds two, removes two |
| 04 | **M9 O-A — `DiagnosticLocation` is half-populated.** Moved here 2026-09-25: it changes `location.sheet_name` in serialised output, which `--format json` now publishes. | Changes serialised output |
| 05 ✅ | **§3.5 — `cells_read` meant bounding-box area**, reporting 5,200 against 2 compared cells on `sparse_range`; now 4. **The metric and `Limits::max_cells_read` were one accumulator**, so both changed: the bound now counts populated cells, which is what the cell map costs since streaming removed the dense allocation. It **loosens only** — a box contains every cell in it, so no workbook passed in 2.6.0 and fails now. | Public metric **and a security limit**; 3 of 80 goldens moved |
| 06 | **§3.3 + §3.6 — options a caller cannot usefully set.** Four comparison settings return `InvalidOptions` if selected; `comparison.format` is read only to be rejected, so its only usable value is its default; `AlignmentMode::HeaderColumn` is exactly `RowKey{[1]}` under a name promising column alignment the crate does not have. | Removes settings, a field, a mode |
| 07 | **§3.8 — values that never arrive.** `Severity::Error` is structurally impossible in RFC-005's two-tier model, and makes `render_summary` print an error count that can only be 0; plus `SheetsDiffError::UnsupportedFormat`, `::Internal`, `OpenErrorKind::Locked`. Found by the implementer outside unit 02's scope. | Removes a field from serialised output |
| 08 | **§3.9 — the options tree cannot be extended without a break.** All eight public *model* structs are `#[non_exhaustive]`; **none of the eight *options* structs is**, and every field is `pub`. So every option ever added is a break while every result field is additive. Found checking whether §3.3 was reversible. **The largest item in the milestone.** | `#[non_exhaustive]`; every later option additive |
| L | **§5.6 — the v2→v3 migration guide.** Every removal needs a row. | Last |

**§3.4's six values were decided 2026-09-25** and recorded in the milestone
README, per RFC-037 criterion 1. **Only one is a removal** (`SourceKind::Unknown`).
`DisplaySource::ReaderProvided` / `::ApplicationProvided` turned out **not to be
dead**: `CellDisplay::new` is `pub` with a `pub source` field, so they are
vocabulary for callers who supply their own display text. The rule that
separates them: a value the *engine* would have to produce and never does is a
dead arm; a value a *caller* may hand us is vocabulary, and vocabulary may be
wider than today's usage.

**All five open questions were reviewed and settled 2026-09-25** against the
owner's design philosophy; the memo is
`.git-exclude/decisions/001-v3-open-questions.md`. RFC-037 §3 was reopened
**once**, gained §3.8 and §3.9 and the §3.3/§3.6 decisions, and was closed
again.

**One question remains open and is not a v3 item:** `hardened()`'s
`max_cells_read` value (5,000,000) was chosen when the field meant bounding-box
area. It should be re-derived from a stated memory budget — **measured, not
estimated**, per the standing rule since M7 — but changing a value is not
breaking, so it can land in any release.

**Still open, and unrelated to v3:** whether 2.x gets anything after 3.0.0.
The ForskScope letter goes out after the final stable release and will have to
**state** that answer rather than ask it.

### Release plan

| Release | Contents | State |
|---|---|---|
| **2.5.1** (patch) | The streaming read (merged, `8fe7c2c`) + f123 unit 01: the cancellation test that does not test, the threat-model surface, and the record sweep | **Dev team is on it.** Cut and publish on completion, then file the advisory |
| **2.6.0** (minor) ✅ | M8 — all six units (00, 01, 02, 03, 06, 07) merged. Units 04 and 05 withdrawn to v3. **This is the release ForskScope adopts.** | **PUBLISHED 2026-09-25**, crates.io, tag `2.6.0`, MSRV 1.88.0. `cargo install sheets-diff --features cli` verified live: 2.6.0, `json` in `--help`, `iso` populated, reorder exits 1. |
| **3.0.0** (major) | **M10 complete — ten units.** [RFC-037](rfcs/accepted/037-v3-scope.md) — a **closed list** of removals and renames: `SheetMatchReason`, four unreachable `DiagnosticKind` variants, four always-failing options, `cells_read`'s meaning, `AlignmentMode::HeaderColumn`. **No new features.** Requires a `v2-to-v3` migration guide. | **RFC-037 ACCEPTED 2026-09-25.** §3 is closed. Three questions in its §7 remain open and block the handoffs that depend on them. |
| — | **M9** — fuzzing, rule deviations, record corrections, measurement. Six of its seven units are invisible; they land on `main` as they are done and need no release. Its one observable unit (O-A) moves into v3. | No release; may run in parallel |

**There is no planned 2.7.0.** *(Corrected 2026-09-25.)* One was scheduled in
`01b0579` — the commit *before* RFC-037 — for "M9's observable half", and then
carried forward unexamined once the owner decided to go straight to a major. The
owner caught it. **3.0.0 followed 2.6.0 directly.**

Two reasons, and the second is the better one:

- **Nothing in it needed a minor.** Six of M9's seven units are invisible.
  The seventh (O-A) changes serialised output, which a major covers anyway.
  Unit 08's two builder setters are additive and can wait.
- **Unit 08 and RFC-037 §3.7 are one decision.** Unit 08 *adds*
  `date_compare` and `max_cells_read`; §3.7 *removes* `number_compare` and
  `build_with_matching`. Splitting them across two releases means settling the
  builder's naming rule twice — which the implementer flagged before I did.

A **patch** release stays available reactively if a defect is found in 2.6.x.
That is different from planning a minor with nothing in it.

**2.5.1 is published** (2026-09-24, crates.io, MSRV 1.88.0).

**The security advisory was decided against**, 2026-09-24, on download evidence
rather than on the defect's severity: v2 has **365 downloads across 12
releases** — this project's CI and ForskScope's testing — and v1's 10,975 sits
on a uniform ~585-per-release floor that is mirroring, not adoption. An advisory
protects users who can act on it; there are effectively none, any v1 user's only
remedy is a major-version migration, and the CHANGELOG and threat model already
state the affected range publicly and permanently.

**Revisit when ForskScope ships**, or if v2 downloads move materially, a second
consumer appears, or anyone reports being affected. Their adoption is scheduled,
so the first is foreseeable. Text, evidence and trigger:
`.git-exclude/security/advisory-draft-bounding-box-allocation.md`.

### Settled by the consumer, 2026-08-17

**`serde::Deserialize`: declined, not deferred.** ForskScope was asked directly
and said no — they have no read path for anything they serialise, and noted that
if they ever gained one they would need a *stable format* rather than a derive,
which is the larger commitment. Recorded in RFC-014's Status and on
[`docs/src/non-goals.md`](docs/src/non-goals.md) as **not planned**, with the
note that the decision rests on one data point and would be revisited on a
second. This closes a question open since M3.

**The cancellation fix does not get its own release.** Also ForskScope's call,
also asked directly: they have shipped nothing against any version of this
crate, `.xlsx` is still failing closed on their side, so there is no released
ForskScope in which a user can start a comparison, let alone cancel one. Their
instruction was *"do not prioritise a release for us — ship it with the rest of
the milestone."* It ships with M7.

They also confirmed **2.4.1 as the adoption target**, and reported their
acceptance matrix now blocked on an upstream dependency whose fix is merged but
unpublished — outside their control and ours. Their own blockers are closed.

### Not scheduled

- **`serde` `Deserialize`** — a public API expansion needing its own RFC, and
  ForskScope caches nothing, so ask before building.
- **Upstream-blocked** — `CellNumberFormat` (calamine keeps `mod formats`
  private through 0.36) and `WorkbookObjectChange` (no object content exposed).
- **The v3 question** — deferred to M6's close by default; see below.

### Superseded M4 sketch *(kept for the record)*

Gated by M3's findings and ForskScope's runtime report.

| Unit | Item |
|---|---|
| 01 | **F** — `cells_compared`. Isolated and first, so it stays cuttable as a standalone 2.3.1 |
| 02 | **H (doc)** — record that `CellValue::Duration` cannot occur through `.xlsx` |
| 03 | **G** — disambiguate the shared display address, **additively** on `CellDiff` (which is `#[non_exhaustive]`, so this is minor, not major) |
| 04 | **Large-workbook memory** — measure first, then act. Candidates: C's bytes-path borrow; `cell_map_to_align`'s full clone of every `CellValue`; RFC-024 §7's unbuilt Sparse/Dense choice |
| 05 | **E** — `docs/` per NF-024/026/027 |
| 06 | Any defect A surfaced |

**Exit:** NF-024/026 met (both **MUST**, unmet for three releases); large-workbook
memory measured and acted on; **release 2.4.0**.

### The v3 question — reopened as a question, not a milestone

M5 was dissolved on the argument that `CellDiff` is `#[non_exhaustive]` so G is
additive, and one unreachable variant does not justify a major version. Both
still hold individually. What changed is the accumulation: **three** unreachable
`CellValue` variants, **four** permanently-empty types, and **thirteen** RFCs
whose designs shipped only in part.

The honest summary is that the public model describes a more capable engine than
the one that exists — not through neglect, mostly through upstream constraints
and deliberate deferrals, but the gap is real and invisible from the API.

**Not a recommendation for v3.** M4's doc note and M6's non-goals section may be
a sufficient answer, and a major version would make ForskScope migrate again
months after the last time. Deferred to M6's close, when the documentation
answer can be judged on its merits. Owner's alone under §6.7.

### Superseded M3 sketch *(kept for the record)*

**2.3.0 shipped 2026-08-16**; the ForskScope notification was sent the same day
and **answered the same day**
(`.git-exclude/tmp/sheets-diff-reply-2.3.0-2026-08.md`). Their reply reshapes
this milestone's inputs:

**Confirmed relevant — priority up.** `compare_bytes` doubling peak memory: they
confirmed their adapter *does* pass bytes, so this moves from speculative to
known-relevant.

**Confirmed irrelevant — deprioritise.** The MSRV move costs them nothing (their
workspace declares 1.91). Stored-diff reproduction does not affect them — they
cache nothing and compute every comparison fresh. `code()` stability held, so
their adapter's matching surface is untouched.

**New finding, ours not theirs.** Their §4 observes that the formula-attachment
defect was "silent and content-dependent". That points at a gap their reply did
not name and we had not either: **every one of our 15 fixtures is synthetic** —
14 generated by `rust_xlsxwriter`, one deliberately-corrupt non-zip. Not a single
Excel-authored workbook. That is precisely why D-04 hid for so long (real Excel
lays sheets out differently from `rust_xlsxwriter`) and why D-01's reachability
could not be tested without hand-patching XML. A real-world corpus is an M3
candidate we own.

**Timing.** They are mid-way through a platform-acceptance matrix keyed to
SHA-256 artifact hashes; adding a runtime dependency invalidates evidence already
collected, so re-enabling `.xlsx` is queued behind it. Adoption is weeks out, not
days. M3 has room.

RFC-033 reconstruction; per-RFC status verification; `docs/` per NF-024/026/027;
non-integrity defects. M2 deliberately deferred four items rather than forgetting
them, each recorded in the threat model's residual-risk section:

- `DiffMetrics.cells_compared` counts only changed cells, not coordinates visited
- Two correctly-computed diffs can share a display address
- The bytes path owns a copy where it could borrow, doubling peak memory
- `CellValue::Duration` is unreachable through `.xlsx` — a public API question

Release boundary and scope to be agreed with the owner before any handoff.

## 5. Sequencing constraints

These are forced, not preferences:

1. **T1 before any release.** Otherwise "done" is self-report, which §2.6 of the
   development policy prohibits as evidence.
2. **Fixture integrity before CI**, or the first CI run fails on a dirty tree.
3. **MSRV before calamine.** Edition 2024 uses the MSRV-aware resolver; bumping
   calamine while 1.85 is declared can silently resolve back to 0.35 and produce
   a green build with the advisory chain intact.
4. **Our own bounds before declaring safety.** ForskScope's gate inspects
   dependency paths; it cannot see an unbounded allocation in `align.rs`.
5. **RFC-033 before design-conformance review** — it is cited as the normative
   lexicon for the public model.

## 6. Integrity as a security property

ForskScope is a diff/**merge** workstation: a user acts on our output. A
silently missed difference means a user is shown "identical", accepts a merge,
and loses data. That is a data-loss path reachable from ordinary input with no
attacker involved.

False negatives are therefore treated as **integrity failures**, not quality
defects, and the integrity-affecting subset of T3 ships inside the security
release rather than after it.

## 7. Risk register

| | Risk | Mitigation |
|---|---|---|
| R1 | ~~Dependency fix reads as an all-clear while first-party resource risks persist~~ | **Closed 2026-08-16** — M2 delivered bounds and integrity fixes alongside the dependency clearance |
| R2 | ~~`calamine` 0.35→0.36 delta larger than expected~~ | **Closed 2026-08-16** — unit 01's spike proved zero API delta and a 1.88 floor |
| R3 | Status-verification pass finds further partials; scope grows | Timebox; record deferrals in `Status` fields rather than fixing all |
| R4 | Correctness fixes change output; consumer adapters see behaviour change | Goldens must exist (M1) before fixes land; document as behaviour changes |
| R5 | Single-maintainer capacity against a large queue | Scope reduction is the lever — see D2 |
| R6 | Unbounded `Limits::default()` leaves every caller unprotected | Bound superlinear paths by default; ship a `hardened()` preset for the rest |

## 8. Decisions of record

| | Decision | Date |
|---|---|---|
| D0 | MSRV 1.85.0 → 1.88, contingent on build verification. 1.91 declined: a library's floor is a consumer contract, and 1.88 already satisfies a 1.91 workspace. | 2026-08-15 |
| D1 | ForskScope re-enablement is the organizing goal — as anchor, not as definition of done. | 2026-08-15 |
| D2 | The `parallel` feature is **removed**. It parallelises comparison while parsing stays sequential, so it targets the wrong bottleneck; it imposes a global `rayon` pool on host applications; and it is a determinism risk. RFC-025 remains `accepted/`, amended with the parse-parallel design and a re-introduction gate requiring a measured parse/compare split. | 2026-08-15 |
| D3 | Adopt supply-chain gating equivalent to the consumer's, plus first-party hardening. | 2026-08-15 |
| D4 | A **sufficient** threat model is required, not a minimal one. | 2026-08-15 |

## 9. Non-goals reaffirmed

Confirmed by the consumer and unchanged: no merge or write capability, no GUI
binding beyond `output::view`, no formula evaluation, no style diffs beyond
what `calamine` exposes, no network or telemetry. See NG-001…NG-010 in the
requirements.

## 10. Open items

- **RFC-033** — cited as normative in 11 places in `src/`; no copy exists
  anywhere. Must be reconstructed from the code that references it.
- **Repository metadata conflict** — `Cargo.toml` declares
  `nabbisen/sheets-diff-rs`; the git remote and `SECURITY.md` both say
  `forskscope/sheets-diff-rs`. `repository` is what crates.io and docs.rs
  publish. Needs an owner decision before either is edited.
- **v2 RFC statuses are provisional** — placement in `done/` reflects that the
  v2 line shipped, not per-RFC verification. See `rfcs/README.md`.
