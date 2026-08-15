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

Every push and pull request runs five jobs (`.github/workflows/ci.yaml`,
RFC-034 Handoff 02). Reproduce each locally before opening a PR:

| Job | What it checks | Reproduce locally |
|---|---|---|
| `test` | Every feature combination builds and tests on Linux and Windows: `--no-default-features`, `--features serde`, `--features chrono`, `--features cli`, `--features serde,chrono,cli` | `cargo test <flag>` for each combination above |
| `msrv` | The crate builds at the declared MSRV (`rust-version` in `Cargo.toml`), not just a newer default toolchain | `rustup toolchain install 1.85.0 && rustup run 1.85.0 cargo check --all-features` |
| `lint` | Formatting and Clippy are gates, not advice — no `#[allow(...)]` to silence a finding; fix it | `cargo fmt --all --check` and `cargo clippy --all-targets --all-features -- -D warnings` |
| `tree` | The test suite must not rewrite tracked files (the defect Handoff 01 fixed) | `cargo test --features serde,chrono,cli && git status --porcelain` (expect no output) |
| `fuzz-smoke` | Each `fuzz/` target runs a bounded number of iterations without crashing | see `fuzz/README.md` |

`lint` and `tree` are the cheapest to run before pushing. If `cargo fmt --all`
or `cargo clippy --fix` would change behaviour rather than only style, prefer
a manual fix — automatic fixers can silently paper over a real bug.
