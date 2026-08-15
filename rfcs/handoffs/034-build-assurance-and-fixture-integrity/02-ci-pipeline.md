# Handoff 02 — CI pipeline

**Governing RFC.** [RFC-034](../../done/034-build-assurance-and-fixture-integrity.md) §5.4–5.5
**Roadmap.** M1
**Sequence.** Third. Requires the `parallel` removal and Handoff 01 to be merged
first, or the pipeline is red on arrival.

## Purpose

Make every claim this project publishes machine-checked, and make NF-023 true.

## Background

`.github/workflows/` contains only an empty `scripts/` directory. The previous
workflow was deleted in `a06fb0e` and never replaced. In the absence of CI the
project shipped a feature that had never compiled (`parallel`, 2.2.0), a failing
clippy gate, a test suite that dirtied the tree, and an MSRV nothing verified.

## Applicable requirements

NF-020 (Linux, Windows, macOS support), NF-023 (CI covering Linux plus one
further OS), NF-021 (platform-native paths), RFC-026 (dependency governance),
RFC-028 (fuzzing), RFC-031 (stability and MSRV).

## Change scope

- New `.github/workflows/ci.yaml`
- Remove the empty `.github/workflows/scripts/` directory if it remains
- `README.md` — add a CI status badge alongside the existing badges

## Non-change scope

Do **not** add release, publish, or tagging automation — explicitly out of scope
per RFC-034 §4. Do not add coverage tooling or thresholds. Do not modify library
source to make a job pass; a red job is a finding to report, not a signal to
edit `src/`.

## Required implementation

Jobs, per RFC-034 §5.4:

| Job | Definition |
|---|---|
| `test` | matrix `{ubuntu-latest, windows-latest}` × `{--no-default-features, --features serde, --features chrono, --features cli, --features serde,chrono,cli}`; `cargo build` then `cargo test` |
| `msrv` | pinned toolchain equal to `Cargo.toml`'s `rust-version`; `cargo check --all-features`; must not silently use a newer toolchain |
| `lint` | `cargo fmt --all --check` and `cargo clippy --all-targets --all-features -- -D warnings` |
| `tree` | `cargo test --features serde,chrono,cli`, then fail if `git status --porcelain` is non-empty |
| `fuzz-smoke` | each target under `fuzz/` for a bounded run count (`-runs=<n>`), nightly toolchain, non-blocking on infrastructure failure but blocking on a crash |

Notes:

- The **feature matrix is the highest-value job**. It is what would have caught
  the `parallel` break. Do not reduce it to default-features-only for speed.
- The `tree` job is the permanent guard against Handoff 01's regression class.
- `--all-features` in `lint`/`msrv` is only valid once `parallel` is gone; until
  then it fails. That is the intended sequencing.
- Pin action versions by tag. Do not use floating `@main`.

## Required tests

The pipeline is the deliverable. It must be **demonstrated to fail correctly**,
not merely observed passing:

1. Temporarily reference a non-existent enum variant behind one feature; confirm
   the matrix goes red on exactly that combination; revert.
2. Temporarily hand-edit an `expected.json`; confirm `test` goes red; revert.
3. Temporarily make the generator write during a test; confirm `tree` goes red;
   revert.
4. Temporarily introduce a clippy warning; confirm `lint` goes red; revert.

A pipeline never observed failing has not been shown to work.

## Required documentation updates

- README CI badge
- `.github/CONTRIBUTING.md` — state which checks run and how to reproduce them
  locally

## Acceptance criteria

1. All jobs green on `main` at the merge commit.
2. Every feature combination in the matrix is built and tested on both platforms.
3. `msrv` demonstrably uses the declared floor — show the resolved `rustc --version`
   in the log.
4. Each of the four deliberate-failure checks produced a red run, with links.
5. No library source changed to make a job pass.

## Prohibited shortcuts

- Do not use `continue-on-error` to make a red job appear green.
- Do not narrow the matrix to save minutes.
- Do not add `#![allow(...)]` or `#[allow(...)]` to silence clippy findings; fix
  them, or report them as findings if the fix is non-obvious. The two known ones
  are `approx_constant` in `src/normalize.rs:134-135` and
  `tests/integration.rs:812`, both trivially fixed by using a non-π constant.
- Do not skip Windows because something fails there. A Windows failure is a
  genuine portability finding under NF-020/NF-021 and must be reported.

## Compatibility constraints

None — no shipped artefact changes.

## Security constraints

- Workflows get read-only `GITHUB_TOKEN` permissions unless a job demonstrably
  needs more.
- No secrets are required by any job in this handoff. If one appears to be
  needed, stop and report.
- Pin third-party actions to a tag or SHA; an unpinned action is a supply-chain
  hole in the very pipeline meant to close them.

## Known risks

- Windows may surface pre-existing platform assumptions — a finding, not a
  reason to drop the job.
- `fuzz-smoke` needs a nightly toolchain and may be flaky on infrastructure
  grounds. Distinguish infrastructure failure from a genuine crash; only the
  latter blocks.
- CI minutes grow with the matrix. Acceptable: this is the project's only
  evidence mechanism.

## Required evidence

- Links to green runs on both platforms across all feature combinations
- Links to the four deliberate-failure runs
- The `msrv` job log showing the resolved compiler version
- Confirmation that no file under `src/` changed in this handoff

## Review request format

Per the development policy §9.2, plus explicit confirmation that each
deliberate-failure check was performed and reverted.
