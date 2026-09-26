## ✨ Contributing

We’re happy to receive feedback, bug reports, and questions via GitHub Issues.  
Pull requests are also welcome — though please note that we may not always be able to accept them.

This project is maintained as a labor of love. We welcome community participation, but:

- Issues that are respectful and constructive are appreciated.
- Pull requests are reviewed, but acceptance is not guaranteed.
- We do not engage in long debates or vision disagreements.
- If you have a different direction in mind, please fork freely, provided proper licensing is respected.

Thanks for understanding the scope and spirit of the project.

## Checks CI runs

Every push and pull request runs the jobs below (`.github/workflows/ci.yaml`,
RFC-034 Handoff 02, RFC-035 Handoff 03, M9 unit 00). Reproduce each locally
before opening a PR:

| Job | What it checks | Reproduce locally |
|---|---|---|
| `test` | Every feature combination builds and tests on Linux and Windows: `--no-default-features`, `--features serde`, `--features chrono`, `--features cli` (`cli` implies `serde` and `chrono`, so it also covers what `--features serde,chrono,cli` would) | `cargo test <flag>` for each combination above |
| `msrv` | The crate builds at the declared MSRV (`rust-version` in `Cargo.toml`), not just a newer default toolchain: `--all-features` check, doctests, and a minimal `--no-default-features` build | `rustup toolchain install 1.88.0 && rustup run 1.88.0 cargo check --all-features && rustup run 1.88.0 cargo build --no-default-features` |
| `lint` | Formatting, Clippy and rustdoc are gates, not advice — no `#[allow(...)]` to silence a finding; fix it | `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings` and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` |
| `tree` | The test suite must not rewrite tracked files (the defect Handoff 01 fixed) | `cargo test --features serde,chrono,cli && git status --porcelain` (expect no output) |
| `deps` | The dependency tree is gated on advisories, bans (no network-capable crates — NF-015), licenses, and sources (`deny.toml`, RFC-035 §5.5) | `cargo install cargo-deny --locked && cargo deny check` |
| `fuzz-smoke` | Each `fuzz/` target runs a bounded number of iterations without crashing | see `fuzz/README.md` |
| `doctests-nightly` | The `compile_fail,EXXXX` doctests fail **with the code they name**. Stable rustdoc only checks that they fail, for any reason, so this job is not a duplicate of the stable doctest run | `cargo +nightly test --doc --all-features` |
| `public-api` | No public item is removed or changed since the last release tag unless the major version in `Cargo.toml` has moved; additions pass | `cargo install cargo-public-api --locked && cargo +nightly public-api --all-features diff <last-tag>..HEAD --deny removed --deny changed` (it checks out the tag in place, so commit or stash first) |
| `install` | `cargo install --path . --features cli --locked` produces a binary whose `--format json` fills `CellDateTime.iso` | `cargo install --path . --features cli --locked --root <dir>`, then run `<dir>/bin/sheets-diff` on `tests/fixtures/generated/date_column/{old,new}.xlsx --format json` |
| `package` | **Release tags and manual runs only.** `cargo publish --dry-run` packages cleanly and the file list has no `.git-exclude/` entry | `cargo package --list --locked` and `cargo publish --dry-run --locked` |

`lint`, `tree`, and `deps` are the cheapest to run before pushing. If
`cargo fmt --all` or `cargo clippy --fix` would change behaviour rather than
only style, prefer a manual fix — automatic fixers can silently paper over a
real bug. Likewise, if `cargo deny check` fails, the fix is almost never a
version bump to make it pass quietly — read what it found and report it.
