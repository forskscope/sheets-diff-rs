# RFC-026 — Feature Flags and Dependency Governance

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0 decision  
**Related:** RFC-004, RFC-014, RFC-016, RFC-025, RFC-028

## 1. Summary

Define the feature-flag matrix and dependency governance policy for `sheets-diff`
v2. A spreadsheet diff crate should remain easy to embed in GUI applications,
CLI tools, and libraries without pulling unnecessary dependencies.

## 2. Motivation

v2 may add optional serialization, chrono integration, parallelism, benchmark
support, fuzzing, and richer formatting. If all dependencies are enabled by
default, the crate becomes heavier and harder to audit. If feature flags are not
planned, public API availability becomes confusing.

## 3. Goals

- Keep the default feature set small and useful.
- Gate optional heavy dependencies.
- Make feature-dependent API explicit and documented.
- Pin or bound dependencies carefully, especially `calamine`.
- Avoid surprise network, telemetry, or system dependencies.

## 4. Non-goals

- Supporting every historical Rust version.
- Making every feature combination equally important.
- Avoiding all optional dependencies.

## 5. Proposed features

```toml
[features]
default = ["xlsx"]
xlsx = []
serde = ["dep:serde"]
chrono = ["dep:chrono"]
parallel = ["dep:rayon"]
json-cli = ["serde", "dep:serde_json"]
unstable-formatting = []
unstable-objects = []
```

`xlsx` is the main supported format. If future formats are added, they should be
explicit features.

## 6. Dependency policy

- Use exact compatible ranges for 0.x dependencies instead of overly broad `0`.
- Review dependency MSRV before adoption.
- Avoid dependencies that perform I/O outside explicit input paths.
- Optional features must not affect default behavior unless enabled.
- Public types from dependencies should not leak unless intentionally accepted.

For example, prefer wrapping `calamine` types inside `sheets-diff` public types.
This protects app developers from upstream dependency churn.

## 7. MSRV policy

Declare an MSRV in `Cargo.toml` and documentation. Changing MSRV is allowed only
in a minor release and must be listed in release notes.

```toml
rust-version = "1.xx"
```

Choose the actual version during implementation based on dependency needs.

## 8. Feature-dependent APIs

If a public method exists only with a feature, document it clearly:

```rust
#[cfg(feature = "serde")]
impl Serialize for WorkbookDiff { ... }
```

Avoid making core enum variants feature-dependent if that complicates matching.
Prefer optional fields or diagnostics.

## 9. Build matrix

CI must test at least:

- default features;
- no default features plus `xlsx` if applicable;
- `serde`;
- `parallel`;
- all features;
- MSRV with default features.

## 10. Acceptance criteria

- `cargo tree` for default features is documented in release notes.
- `calamine` dependency range is intentionally bounded.
- `serde` is optional.
- Parallelism is optional.
- Feature combinations used by docs examples compile in CI.
