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
**Units 01–02 are complete and merged** (`main` at `059ad6f`, CI 17/17): D0's
contingency is discharged, MSRV is 1.88, and `quick-xml` 0.41 has replaced
0.39.4 — both advisories cleared. RFC-035 was accepted 2026-08-16 and units
03–06 are live. **This is not yet the M2 exit criterion:** our own resource
bounds (unit 04) are outstanding, and until they land ForskScope's gate would
pass while `align.rs` can still exhaust memory (R1).


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

**Both halves met, plus the integrity work §6 folded in.** All six units
approved; PR #10 carries 03–06 at 18/18. `RUSTSEC-2026-0194`/`-0195` are out of
the tree, `align.rs` can no longer exhaust memory, and the engine no longer
reports "identical" for cells that differ. **R1 is closed on every axis it
named.** Remaining owner decisions: merge PR #10, release 2.3.0, then notify
ForskScope — in that order, since the notification is only honest once the
release exists.

### M3 — T4 and remaining defects *(unplanned; needs a joint planning session)*

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
