# Handoff 03 — `--format json`, which RFC-013 specified and nothing wired

**Governing RFCs.** RFC-013 (CLI and output formatters), RFC-014 (JSON output)
**Roadmap.** M8 — unit 03
**Sequence.** After unit 01 (landed). Parallel with 02 and 04, but **read §"The
interaction with unit 02" before starting** — the two units meet at one flag.

## Purpose

RFC-013 specifies `sheets-diff --format json`. The library half exists. The CLI
has no such format. Wire it, and correct the record that says it is done.

## Background

`src/output/json.rs` provides `to_json` and `to_json_pretty` over a
`WorkbookDiff`, behind `#[cfg(feature = "serde")]`. Every public type in `model`
derives `Serialize`. The only missing piece is the CLI.

`src/main.rs`'s `OutputFormat` has two variants, `Summary` and `Unified`.

**RFC-013's record is wrong in three ways**, all of which this unit corrects:

1. Its Status line says *"Implemented (2.0.0–2.4.x) — verified 2026-08-16"* and
   records only the exit-code deferral. The missing JSON format is not mentioned.
   A verification pass looked at this RFC and did not notice a specified CLI
   format that does not exist.
2. Its §38 specifies `#[cfg(feature = "json")]` and a function named
   `render_json` returning `Result<String, serde_json::Error>`. There is no
   `json` feature — it is `serde` — and the functions are `to_json` /
   `to_json_pretty` returning `Result<String, String>`. The spec and the code
   drifted in name and in signature.
3. Its §101 records an open question — *"Should JSON output be stabilized in
   v2.0 or marked experimental?"* — which was never answered and which shipping
   a `--format json` answers by default. Answer it deliberately instead.

### The trap: `cli` does not enable `serde`

```toml
cli      = ["dep:clap"]
```

`to_json_pretty` exists only under `serde`. So a naive implementation gives you
a binary whose `--help` lists `json` in one build and not in another, at the
same version. **That is a new instance of this milestone's own theme** — a
surface a competent reader predicts wrongly — introduced by the milestone that
exists to remove them.

**Make `cli` imply `serde`.** A CLI whose advertised formats depend on how it
was compiled is not acceptable, and the cost is two dependencies in a binary
that already links `clap`.

## The interaction with unit 02

Unit 02 gives `--no-warnings` the meaning *"suppress the diagnostics section of
the output"*. If JSON ignores it, `--no-warnings --format json` is inert — the
same defect, newly minted, in this milestone.

**`--no-warnings` applies to whichever format is selected**: it suppresses the
diagnostics section in text and empties the `diagnostics` array in JSON. The
`summary` counters stay truthful in both, per unit 02's rule. Document this on
the flag.

If unit 02 has not landed when you start, implement `--format json` ignoring the
flag, say so in the review request, and leave the wiring to whichever unit lands
second. **Do not guess at unit 02's semantics from this paragraph** — read its
handoff.

## Change scope

- `Cargo.toml` — `cli` gains `serde`
- `src/main.rs` — `OutputFormat::Json`, its dispatch, its error path
- `tests/cli.rs`
- `rfcs/*/013-*.md` — Status, §38's names and signature, §101's answer
- `rfcs/*/014-*.md` — Status, if it also claims a CLI surface
- `docs/` — wherever `--format` is documented
- `CHANGELOG.md`

## Non-change scope

- **Do not change `to_json` / `to_json_pretty`**, their signatures, or the
  serialised shape of any model type. This unit wires an existing surface; it
  does not design one.
- Do not add `--format json-compact`, `--pretty`, or a `--output` file flag.
- Do not change exit codes. A JSON run exits exactly as the same comparison
  would under `--format summary`.
- Do not make JSON the default format.

## Required implementation

1. **`cli = ["dep:clap", "serde"]`** in `Cargo.toml`.
2. **`OutputFormat::Json`**, dispatching to `to_json_pretty`.
3. **Pretty, not compact.** This is a diff tool; its output gets stored, piped
   into `diff`, and read by people. Line-oriented output is worth more here than
   a saved newline, and compacting is one `jq -c` away.
4. **JSON goes to stdout and nothing else does.** No banner, no trailing text.
   The scoped stdout gate exists for the library; this binary's stdout must be
   parseable in full.
5. **`to_json_pretty`'s `Err` maps to exit 2** with the message on stderr.
   Serialisation failing is an internal fault, not bad input.
6. **Exit code is unchanged by format.** `workbooks_differ` runs the same way.
7. **`--help` describes the format** in one line.
8. **RFC-013 corrections**, all three from §Background. For §101, the answer
   this unit implements is: the JSON shape is **stable within 2.x** — the model
   types are public and `#[non_exhaustive]`, new fields may appear in a minor
   release, and no existing field is renamed or removed within a major version.
   Say that in the RFC and in the user-facing docs. If you disagree with that
   answer, say so in the review request rather than implementing a different one.

## Required tests

- `--format json` on a differing pair: stdout **parses as JSON** (parse it, do
  not string-match), and exit is 1.
- `--format json` on an identical pair: parses, exit 0.
- `--format json` on a missing file: exit 2, stdout **empty**, message on stderr.
- `--format json` on a corrupt workbook: exit 3, stdout empty.
- The parsed JSON contains the sheet names and the summary counters, checked
  against the same comparison's `--format summary` output.
- A reorder-only pair — unit 01's case — reports its `moved` sheets in JSON too.

Each shown failing before the fix by a targeted removal.

## Acceptance criteria

1. `--format json` exists, is listed in `--help`, and emits parseable JSON.
2. `cli` enables `serde`; **`cargo build --features cli` alone produces a binary
   with `json` in `--help`**, demonstrated by running it.
3. Output is pretty-printed.
4. stdout carries JSON and nothing else, on every exit path including errors.
5. Exit codes identical to `--format summary` for the same inputs, shown as a
   table across all four exit codes.
6. Serialisation failure exits 2.
7. RFC-013's Status, §38 names and signature, and §101 answer all corrected.
8. `--no-warnings` interaction handled per §"The interaction with unit 02", or
   its absence explicitly reported.
9. Fixture corpus byte-identical.
10. CHANGELOG under `### Added`; gates green, including a build with **only**
    the `cli` feature.

## Prohibited shortcuts

- **Do not gate `OutputFormat::Json` behind `#[cfg(feature = "serde")]`.** That
  is the build-dependent `--help` this unit exists to avoid. Fix the feature,
  not the symptom.
- Do not implement JSON by hand-formatting strings in `main.rs`. RFC-013's first
  line says this file must contain no comparison logic; formatting the model is
  the same prohibition one step out.
- Do not print a warning or progress line to stdout in JSON mode "for
  friendliness".
- Do not silently fall back to `--format summary` if serialisation fails.

## Compatibility constraints

**Additive for the CLI.** No existing invocation changes behaviour.

**`cli` now enables `serde`** — additive for a library consumer (more trait
impls, never fewer), but it does change what `--features cli` pulls in. Name it
in the CHANGELOG under `### Changed`, not only in `### Added`; someone auditing
their dependency tree should not have to discover it.

## Known risks

- **`Result<String, String>`** is `to_json`'s error type, not
  `serde_json::Error`. Do not "fix" it here; RFC-013's §38 is being corrected to
  match the code, not the reverse. If you think the signature is wrong, that is
  a finding, not a task.
- `docs/src/` examples execute. A `--format` list that is now stale will surface
  as a failing doctest — that is the harness working.
- `serde` in the default feature set stays **off**. Only `cli` gains it.

## Required evidence

- Pre-fix failure transcripts
- A `cargo build --no-default-features --features cli` binary's `--help`
- The exit-code table across all four codes, both formats
- The parsed-JSON assertions, not string comparisons
- Corpus byte-comparison
- CI run link

## Review request format

Per development policy §9.2. Additionally: state whether you agree with the
§101 stability answer, and report the `--no-warnings` interaction explicitly
even if it is "unit 02 had not landed".
