# sheets-diff

Structured diff engine for Microsoft Excel `.xlsx` workbooks.

See the [README](../../README.md) for a quick start, feature table, and design
notes.

## Installing the command-line tool

The crate ships a `sheets-diff` command. It is **not built by default**: the crate is a
library first, so a library consumer does not pay for `clap`. The command is behind the
`cli` feature, and `cargo install sheets-diff` without it installs **nothing** — cargo
warns that no binary is available and still exits 0. Ask for the feature:

```sh
cargo install sheets-diff --features cli
```

`cli` also enables `serde` and `chrono`, on purpose: the installed tool's formats
(`--format json` needs `serde`) and its values (`CellDateTime.iso` needs `chrono`) must not
depend on how it was compiled, so `cli` alone is the whole, correct binary. Exit codes and
`--format` are described in the [migration guide](migration/v1-to-v2.md#cli-exit-codes) and
the [API guide](api-guide.md#json--serde-feature-only).

## Contents

- **[API guide](api-guide.md)** — path, reader, and bytes input; the options
  builder and resource limits; text and JSON output; error handling.
- **[Comparison semantics](semantics.md)** — five worked, run-for-real
  scenarios: typed value change, formula change, sheet rename, inserted
  row (and why `AlignmentMode` changes the answer), warning handling.
- **[Non-goals and limitations](non-goals.md)** — what this engine
  deliberately does not attempt, what is limited and why (upstream,
  deferred, or unreachable by construction), and the RFCs that shipped in
  part.
- **[Migration from v1](migration/v1-to-v2.md)** — how to update existing code
  that used v1's `Diff::new` / string cell model.
- **[Migration from v2 to v3](migration/v2-to-v3.md)** — what 3.0.0 removed and
  what to write instead.
- **[Threat model](maintainers/threat-model.md)** — what this crate defends
  against, what it does not, and how each claim is checked.
- **[Performance](maintainers/performance.md)** — measured, not inferred:
  peak memory across a size ladder, where it goes, and cancellation
  latency, with the method and its limits stated alongside every number.
