# Handoff 03 — Supply-chain gates

**Governing RFC.** [RFC-035](../../accepted/035-resource-safety-and-supply-chain-governance.md) §5.5
**Roadmap.** M2, decision D3
**Sequence.** After unit 02 (merged). Independent of unit 04 — either order.

## Purpose

Make the dependency tree a checked property rather than a claim, so the next
advisory fails a build instead of sitting unnoticed for two months.

## Background

`RUSTSEC-2026-0194` and `RUSTSEC-2026-0195` landed against `quick-xml` 0.39.4
and nobody here noticed. The consumer noticed, audited, and disabled `.xlsx`
comparison. That asymmetry is the problem this unit fixes: ForskScope runs a CI
dependency-path gate, `cargo audit`, and a documented re-enablement procedure.
This project has none of those.

Unit 02 cleared the specific advisories. This unit makes the *next* one visible
on the day it lands.

## Applicable requirements

NF-015 (no network), NF-016 (no telemetry), NF-019, RFC-026 (dependency
governance), RFC-035 §5.5.

## Change scope

- New `deny.toml`
- `.github/workflows/ci.yaml` — a `deps` job
- `.github/CONTRIBUTING.md` — how to reproduce the gate locally

## Non-change scope

Do **not** change any dependency version to make the gate pass. If the gate
finds something, that is a finding to report — the whole point is that it tells
us things we did not know. Do not touch `src/`, the public API, or the fixture
corpus.

## Required implementation

### 1. `deny.toml`, four sections

Per RFC-035 §5.5:

- **advisories** — deny. This is the section that would have caught the
  `quick-xml` chain.
- **bans** — deny network-capable crates outright (`reqwest`, `hyper`,
  `ureq`, `curl`, `tokio` with net features, and similar). This converts NF-015
  from prose into a build-time property, which is the single most valuable thing
  in this file. Also deny duplicate versions where practical, and report rather
  than silently allow what you cannot.
- **licenses** — an allowlist consistent with Apache-2.0 distribution.
- **sources** — crates.io only.

Write each section with a comment saying *why*, not just *what*. A future
maintainer reading a bare ban list cannot tell which entries are load-bearing.

### 2. The dev-dependency decision — yours to make and justify

`RUSTSEC-2026-0204` affects `crossbeam-epoch`, reachable only as
`criterion → rayon → rayon-core → crossbeam-deque → crossbeam-epoch`, all
dev-only. It is in the tree today and unit 02 deliberately left it.

Decide whether dev-dependency advisories block the gate, and **write the
reasoning into `deny.toml` as a comment**. Both answers are defensible:

- *Block* — a compromised dev dependency runs on maintainer machines and in CI
  with repository credentials in scope; "dev-only" is not "harmless".
- *Do not block* — dev dependencies ship in no artefact a consumer receives, and
  a noisy gate that maintainers learn to ignore is worse than a quiet one.

If you choose not to block, the advisory must still be *visible* — an
unignorable report, not silence. If you choose to block, you must resolve
`RUSTSEC-2026-0204` in this unit, which likely means a `criterion` bump; check
feasibility before committing to that path.

State which you chose and why in the review request. This is a judgement call I
am delegating deliberately, not an oversight.

### 3. The `deps` CI job

Runs `cargo deny check` on every build. Pin the action or install a pinned
version — an unpinned tool in a supply-chain gate is self-defeating.

Consider whether it belongs as its own job or folded into `lint`. Own job is
preferred: a red `deps` should be distinguishable at a glance from a formatting
failure, because the response is entirely different.

### 4. Demonstrate the gate fires

M1's standard applies. Temporarily add a banned dependency (a network-capable
crate is the obvious choice), show `deps` going red, and revert. Capture the run
URL. A gate never observed failing has not been shown to work.

## Required tests

No unit tests. The gate is the deliverable, and the demonstration above is its
test.

## Acceptance criteria

1. `deny.toml` exists with all four sections configured and commented.
2. `cargo deny check` passes locally and in CI on the current tree.
3. The dev-dependency decision is made, implemented, and its reasoning is in
   `deny.toml`.
4. A banned dependency causes `deps` to go red — **demonstrated**, with a run
   URL, and reverted.
5. All other jobs stay green; no dependency version changed to make the gate pass.
6. `CONTRIBUTING.md` documents the local reproduction command.

## Prohibited shortcuts

- Do not add allowlist entries to silence a finding you have not understood.
  Every exception carries a comment naming what it is and why it is acceptable.
- Do not use `continue-on-error`.
- Do not change a dependency version to go green. Report instead.
- Do not skip the demonstration.

## Compatibility constraints

None — no shipped artefact changes.

## Security constraints

This unit *is* a security control. Two specifics:

- The ban list is the mechanism that makes "this library never accesses the
  network" checkable. Treat it as load-bearing, not decoration.
- Pin whatever tooling the job installs.

## Known risks

- `cargo deny` may surface findings across the tree that nobody has looked at
  before — the same effect `--all-features` clippy had in M1, where the expected
  two findings turned out to be nine. Budget for surprises and **report them**
  rather than widening the allowlist to fit.
- License findings on transitive dependencies can be tedious. Resolve honestly;
  an allowlist entry added without reading the licence is worse than no gate.

## Required evidence

- `deny.toml` and the workflow diff
- `cargo deny check` output on the current tree
- The red `deps` run URL from the demonstration, and its revert
- Confirmation that no dependency version changed
- A statement of the dev-dependency decision and its reasoning

## Review request format

Per development policy §9.2, plus an explicit statement of the §2 decision.
