# Handoff 06 — The CLI you can actually install

**Governing RFCs.** RFC-013 (CLI), RFC-014 (serde / report schema)
**Roadmap.** M8 — unit 06
**Sequence.** After unit 03 (landed 2026-09-25). **Must ship in 2.6.0**, in the
same release as `--format json`, not after it.

## Purpose

This crate has shipped a command-line tool since 2.0.0, documents its flags, its
exit-code contract and now a JSON format — and never says how to obtain it. The
obvious command produces a warning and no binary. The binary a user *can* build
emits a documented JSON field that is always `null`.

Both are the milestone's theme at its outermost layer: the surface promises a
tool the reader cannot get, in a shape the reader cannot get it in.

## Background

### Defect 1 — `cargo install` installs nothing

```
$ cargo install --path .
warning: none of the package's binaries are available for install using the
         selected features
  bin "sheets-diff" requires the features: `cli`
Consider enabling some of the needed features by passing, e.g., `--features="cli"`
```

`default = []` and the `[[bin]]` has `required-features = ["cli"]`. That
combination is correct for a library-first crate — it is the *silence* around it
that is the defect. **`grep -r "cargo install" README.md docs/` returns nothing.**

A user who reads the README, sees a documented CLI with an exit-code contract,
and types the one command every Rust user types, gets a warning and no tool.

### Defect 2 — the installable binary's JSON is wrong by default

Found by the implementer during unit 03 (F-1) and reported rather than worked
around, correctly, because my unit 03 handoff said *"Only `cli` gains `serde`."*
**That instruction was the defect.**

`cli` enables `serde` but not `chrono`, and `CellDateTime.iso` is populated only
under `chrono`. Same workbook, `date_column` fixture, `--format json`:

```
--- cli only                     +++ serde,chrono,cli
-   "iso": null,                 +   "iso": "2024-06-15T00:00:00",
```

The two compound. The install command users will be told to run is
`cargo install sheets-diff --features cli` — **exactly the build that emits
`"iso": null`.** Left alone, 2.6.0 publishes a JSON format whose documented
timestamp field is null for everyone who installs the tool the documented way.

## Change scope

- `Cargo.toml` — the `cli` feature
- `README.md` — installation
- `docs/` — installation, and the `iso` statement in the API guide
- `rfcs/done/013-*.md`, `rfcs/done/014-*.md` — the §11 stability exception
- `.github/workflows/` — see implementation item 5
- `CHANGELOG.md`

## Non-change scope

- **Do not change `default`.** It stays `[]`. A library consumer must not get
  `clap`.
- Do not change `required-features` on the `[[bin]]`.
- Do not add a new feature name (`full`, `bin`, `all`). One more name to explain
  is not an improvement on one sentence of documentation.
- Do not change any JSON field, or `CellDateTime`'s shape.
- Do not touch `--format json`'s implementation. It is correct; its inputs are
  not.

## Required implementation

1. **`cli = ["dep:clap", "serde", "chrono"]`**, with the comment above the
   feature block saying why both are there: the binary's advertised formats, and
   its advertised *values*, must not depend on how it was compiled.
2. **Installation documented in `README.md`**, near the top, as the literal
   command: `cargo install sheets-diff --features cli`. State that the binary is
   not built by default because the crate is a library first, so a consumer does
   not pay for `clap`.
3. **The same in `docs/`**, wherever the CLI is introduced.
4. **The `iso` statement becomes true of the installed tool.** RFC-013 §11's
   stability answer and the API guide currently carry unit 03's written
   exception. Rewrite it: the installed CLI always populates `iso`; a **library**
   consumer without the `chrono` feature still sees `null`. Both halves are true
   and the second still needs saying — do not delete the exception, narrow it.
5. **Keep one CI leg honest about a minimal build.** Unit 03's F-5 is right:
   the `cli`-only leg was what would have caught the `serde` trap, and it now
   compiles `serde` and will compile `chrono`. Ensure at least one leg still
   builds the library with `--no-default-features` and no `cli`, so a minimal
   build stays exercised. If such a leg already exists, say so and change
   nothing.

## Required tests

- **A test that the documented install command's feature set produces a binary
  that populates `iso`.** The natural form is a CLI test on the `date_column`
  fixture asserting the parsed JSON's `iso` is a non-null string. It fails today
  under `--no-default-features --features cli`.
- **`--help` remains byte-identical** between a `--features cli` build and an
  all-features build. Unit 03 established this; it must not regress now that the
  feature list is longer.

Show each failing first by a targeted change.

## Acceptance criteria

1. `cli` enables `clap`, `serde` and `chrono`.
2. `cargo build --no-default-features --features cli` produces a binary whose
   `--format json` populates `iso`, **demonstrated by running it** and showing
   the before/after line.
3. `--help` byte-identical between the cli-only and all-features builds.
4. `README.md` and `docs/` give the install command; `grep -r "cargo install"`
   now finds it.
5. The `iso` exception narrowed to the library case in RFC-013 §11 and the API
   guide, not deleted.
6. A minimal (`--no-default-features`, no `cli`) build is still exercised by CI,
   or its absence reported.
7. `Cargo.lock` change, if any, stated and explained.
8. Fixture corpus byte-identical. **Goldens may move** if any golden carries an
   `iso` — check, and say either way.
9. CHANGELOG: `### Changed` for the feature, `### Added` for the docs. Gates
   green, including both build variants.

## Prohibited shortcuts

- **Do not make `cli` default.** It puts `clap` in every library build.
- Do not "fix" `iso` by removing the field or defaulting it to a string without
  `chrono`. The field's contract is a real timestamp or nothing.
- Do not document `cargo install sheets-diff` without `--features cli` and hope.
  Run the command you write down.

## Compatibility constraints

**`cli` gains two dependencies.** Additive for anyone enabling `cli` — more
trait impls and a populated field, never fewer. It does change what
`--features cli` pulls in, and `chrono` brings `calamine/chrono` with it. Name
it in `### Changed`, as unit 03 did for `serde`.

**`iso` becomes populated in the installed CLI's JSON output.** For a consumer
who parsed that output and relied on `null`, that is a change. It is the
correction of a defect and it happens **before** `--format json` is ever
published, so no released behaviour changes. Say that explicitly in the
CHANGELOG — "fixed before release" is a different claim from "fixed".

## Known risks

- `chrono` in the binary raises the build's dependency surface. `cargo deny`
  must stay green; it already covers `chrono` under the existing feature.
- Doctests and executing docs in `docs/src/` may assert on `iso: null`.
- **MSRV.** `chrono` is already an optional dependency built in the
  all-features MSRV leg, so 1.88 should hold. Verify rather than assume.

## Required evidence

- The `cargo install --path .` warning before, and the working documented
  command after
- The `iso` line from a cli-only build, before and after
- `--help` `cmp` between the two builds
- Corpus byte-comparison and the golden check
- Both build variants' gates, and MSRV
- CI run link

## Review request format

Per development policy §9.2. Additionally: report whether any golden carried an
`iso`, and state what a minimal library build still exercises in CI.
