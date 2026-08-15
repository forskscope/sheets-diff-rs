# Handoff 02 — MSRV 1.88 and calamine 0.36

**Governing RFC.** RFC-026 (dependency governance), RFC-031 (stability policy)
**Roadmap.** M2, decision D0
**Sequence.** Second. Requires unit 01's spike, which is complete and approved.

## Purpose

Clear the dependency chain that caused ForskScope to disable `.xlsx`
comparison, by raising the MSRV to 1.88 and migrating to `calamine` 0.36.

## Background

`quick-xml` 0.39.4 carries `RUSTSEC-2026-0194` and `RUSTSEC-2026-0195`, both
denial-of-service on XML input, both fixed in 0.41. It is reachable through
`calamine` 0.35, and our consumer's users open files they did not author — so
the path is reachable from untrusted input, not theoretical.

Unit 01's spike ([review](../../../.git-exclude/reviewed/035-handoff-01-calamine-spike/README.md))
established, by building rather than reading:

- `calamine` 0.36.1 compiles against our usage with **zero code changes**
- the effective MSRV floor is **exactly 1.88** — 1.85 and 1.87 both fail, and
  `zip` 8.6 adds no constraint beyond calamine's own
- `quick-xml` resolves to 0.41.0 and 0.39.x leaves the tree entirely
- no golden and no test output changes

Roadmap decision D0 approved MSRV 1.88 contingent on that proof. The
contingency is discharged.

## Applicable requirements

NF-015 (no network), NF-019 (resource limits help against pathological
workbooks), NF-020/NF-023 (platform and CI coverage), RFC-026, RFC-031.

## Change scope

- `Cargo.toml` — `rust-version` 1.85.0 → **1.88.0**, `calamine` "0.35" → "0.36"
- `Cargo.lock` — the resulting resolution
- `.github/workflows/ci.yaml` — `env.MSRV` and the `dtolnay/rust-toolchain@` pin
- Nine clippy fixes (§Required implementation)
- `CHANGELOG.md` — an `[Unreleased]` entry
- `README.md` — the `calamine 0.35 pinned` design note is now wrong

## Non-change scope

Do **not** implement anything the spike merely *made possible*. Specifically,
leave alone: the `is_1904` plumbing, hyperlinks, merged regions, tables, pivot
tables, and shared-formula behaviour. Those are unit 05's or later, and mixing
capability work into a dependency migration destroys the property that makes
this unit safe — that behaviour is provably unchanged.

Do **not** touch comparison logic, the public API, or the fixture corpus.

## Required implementation

### 1. Order matters

Raise `rust-version` **before** bumping `calamine`. Edition 2024 uses the
MSRV-aware resolver, so bumping calamine while 1.85 is still declared can
quietly hold you on 0.35 and produce a green build with the advisory chain
intact. Verify with `cargo tree` which version actually resolved — do not infer
it from `Cargo.toml`.

### 2. Three files move together

`Cargo.toml`'s `rust-version`, the workflow's `env.MSRV`, and the
`dtolnay/rust-toolchain@` pin must all become 1.88.0. The `msrv` job's drift
guard enforces the first two against each other and the resolved toolchain
against the third, so a partial edit fails CI by design. That guard has never
fired in anger; this is its first real exercise.

### 3. Nine clippy fixes — not eight

The MSRV raise makes two lints newly eligible, because clippy will not suggest a
fix the declared floor cannot compile:

| Lint | Count | Locations |
|---|---:|---|
| `collapsible_if` | 8 | `src/align.rs`, `src/diff.rs` ×3, `src/matcher.rs`, `src/meta.rs` ×2, `src/output/view.rs` |
| `manual_is_multiple_of` | 1 | `benches/workbook_diff.rs:52` |

The ninth is easy to miss: clippy's summary line reports *"(lib) generated 8
warnings"* because the benches finding is in a different target. `lint` runs
`--all-targets`, so it is gated too. Verify with

```
cargo clippy --all-targets --all-features 2>&1 | grep -oE "index.html#[a-z_]+" | sort | uniq -c
```

and confirm the output is empty when you are done.

These are mechanical. `collapsible_if` becomes a `let`-chain (`if let … && …`),
which is precisely the readability gain 1.88 was chosen for. Fix them; do not
suppress them.

### 4. Documentation that is now false

`README.md`'s design note says **"`calamine` 0.35 pinned. The `Data` enum
variant set is the grounding for all `CellValue` conversions."** Update the
version. The second sentence stays true — the spike confirmed `Data` is
byte-identical between 0.35 and 0.36.

Check for other stale `0.35` references across `src/` doc comments, `docs/`, and
the RFCs' prose. Several diagnostics and doc comments say "calamine 0.35 does
not expose …". Where that statement is still true of 0.36 — number formats, for
instance — update the version but keep the claim. Where the spike showed it is
now false, **report it rather than rewriting it**: those are unit 05/06 scope,
and a doc change that promises a capability nothing implements is worse than a
stale version number.

## Required tests

No new tests. The existing suite is the regression check, and the spike already
showed it passes unchanged. What must be demonstrated is that it *still* does
after the lint fixes, which the spike did not include.

## Acceptance criteria

1. `cargo tree -e normal -i quick-xml` shows **0.41.0 or later**, and no 0.39.x
   appears anywhere in the tree.
2. `cargo clippy --all-targets --all-features -- -D warnings` is clean.
3. All five feature combinations pass on both platforms in CI.
4. The `msrv` job passes at the new floor, and its log shows
   `resolved: rustc 1.88.0`.
5. **The fixture corpus and all seven goldens are byte-identical.** This is the
   evidence that a dependency migration changed no behaviour; if a golden moves,
   stop and report it as a finding.
6. `cargo test` still leaves the tree clean (`tree` job green).
7. No `src/` change other than the eight `collapsible_if` fixes.

> **Correction, 2026-08-16 (architect).** Criterion 7 as written above
> contradicts §4, which explicitly requires updating stale `0.35` references in
> `src/` doc comments. That is a drafting error in this handoff, not a
> requirement the implementer failed. **Criterion 7 means: no comparison-logic,
> public-API, or fixture change** — the same thing the non-change scope states
> in prose. Version-number corrections in comments and message strings are
> compliant. Recorded here rather than by editing the criterion, since
> rewriting a requirement after the fact to match an implementation is
> prohibited even when the requirement was at fault. See
> `.git-exclude/reviewed/035-handoff-02-msrv-calamine/README.md` §2.

## Prohibited shortcuts

- Do not bump `calamine` before `rust-version`; see §1.
- Do not suppress any lint with `#[allow]`. Fix or report.
- Do not bless a golden. A moved golden here means the parser's behaviour
  changed under us, which is a finding of real significance.
- Do not implement any newly-available calamine capability. See non-change scope.
- Do not update a doc claim to promise something no code does.

## Compatibility constraints

Raising the MSRV is a **minor**-version change under the ecosystem convention
this project follows. It is a real compatibility event for consumers on older
toolchains and belongs in the CHANGELOG prominently, not as a footnote. Note in
the entry that the driver is a security fix, so downstreams can weigh it.

## Security constraints

This unit exists to clear two advisories. Confirm they are gone from the
resolved tree rather than assuming the version bump implies it.

`RUSTSEC-2026-0204` (`crossbeam-epoch`, via `rayon` → `criterion`) will remain.
It is dev-only and out of scope here; unit 03 decides whether dev-dependency
advisories block the gate.

## Known risks

- **Windows.** The spike ran on Linux only. `zip` 7.2 → 8.6 is a major bump in
  the archive reader; if a Windows leg surfaces a difference, that is a genuine
  portability finding to report, not a reason to narrow the matrix.
- **The drift guard fires on a partial edit.** That is intended. If it fires,
  read the message — it names both values and both files.

## Required evidence

- `cargo tree -e normal -i quick-xml` before and after
- The clippy lint-kind count, empty after the fixes
- Full CI run links: all ten `test` legs, `msrv`, `lint`, `tree`, `fuzz-smoke`
- The `msrv` job log showing `resolved: rustc 1.88.0`
- `git status --porcelain -- tests/fixtures` empty, and the corpus hash
  unchanged from `c056b0fc…`
- The diff, confirming no `src/` change beyond the eight lint fixes

## Review request format

Per development policy §9.2, plus explicit confirmation of criterion 5 — that
every golden is byte-identical.
