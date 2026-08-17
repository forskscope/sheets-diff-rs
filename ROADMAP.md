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

### M3 — "Real files, and a record that is true" — 🔄 **OPEN 2026-08-16** *(no release)*

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

### M4 — "The code contradicts itself" — 🔄 **OPEN 2026-08-17** *(2.4.0)*

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

### M6 — "The documentation MUSTs" — 🔄 **OPEN 2026-08-17** *(release)*

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

Also **`DiffMetrics`'s undocumented fields** (F-D, raised in M4 unit 02's
review). M4 unit 02 documented `cells_compared`, leaving it the only documented
field of five. The gap that matters is `cells_read`: on the `sparse_range`
fixture it reads 5200 against 2 compared, because it counts every physically
visited cell including empties — a 2600× difference no consumer would predict
from the name.

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

### M7 — "Measure, then change" *(release; scope set by measurement)*

Large-workbook memory; cancellation granularity (polled per sheet pair, not per
cell batch as RFC-012 specifies); the shared display address; and **the
v1.2-vs-v2 benchmark comparison (RFC-027), moved here from M6** — it was grouped
as documentation debt, but it requires building v1.2 and running comparable
benchmarks, which is measurement, not writing. These share the
property that **their scope cannot honestly be written until something is
measured**, so they are grouped to keep that discipline in one place.

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
