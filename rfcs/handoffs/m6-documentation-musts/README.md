# Handoffs — M6: the documentation MUSTs

Five units closing two **MUST** requirements unmet since v2.0.0, one **SHOULD**,
the gaps in a third MUST, and four corrections parked from M4 and M5.

**This milestone produces a release.** Nothing in it changes behaviour, but
documentation is what a consumer reads before adopting, and ForskScope is
deciding whether to re-enable us.

## The requirements, quoted rather than paraphrased

From `.git-exclude/specs/sheets-diff-v2-requirements.md` — note that this file
lives **outside the tracked repository**, which is why nothing in `rfcs/` or
`docs/` defines the NF numbers they cite:

| | Level | Text |
|---|---|---|
| NF-024 | **MUST** | Document the v2 API with examples for path, reader, bytes, options, and formatter usage. |
| NF-025 | **MUST** | Document migration from v1/v1.2 to v2. |
| NF-026 | **MUST** | Document non-goals and limitations clearly. |
| NF-027 | SHOULD | Document comparison semantics with examples: typed value change, formula change, sheet rename, inserted row, and warning handling. |

NF-024 and NF-027 both name **specific examples**. That is not decoration: it
means the requirement is not met by prose describing the API, and it is what
makes unit 01 a prerequisite rather than a nicety.

## The state being fixed

`docs/` is four files: a 13-line index, a 266-line migration guide, a 281-line
threat model, and a `SUMMARY.md`. There is **no API guide, no non-goals page,
and no comparison-semantics page.** NF-024 and NF-026 have never been met.

And the migration guide contains **11 Rust code blocks that nothing compiles.**
No `include_str!`, no `mdbook test`, no docs job in CI.

## Queue

| | Unit | Requirement | Kind |
|---|---|---|---|
| 01 | [Make documentation checkable](./01-documentation-checkable.md) | NF-025 | harness + existing guide |
| 02 | [The API guide](./02-api-guide.md) | **NF-024** | new page, compiled examples |
| 03 | [Semantics and non-goals](./03-semantics-and-non-goals.md) | **NF-026**, NF-027 | two new pages |
| 04 | [The undocumented public surface](./04-undocumented-surface.md) | F-D, F-G | doc comments |
| 05 | [Record corrections](./05-record-corrections.md) | F-I, F-L | annotations |

**01 must come first.** 02 and 03 write the examples NF-024 and NF-027 require,
and without 01 they would ship unverified — which is the defect class M4 and M5
spent eight units removing, manufactured at scale in the release meant to fix
the documentation. 04 and 05 are independent of everything.

## Why unit 01 exists, and why it is not optional

M5's whole finding was that a rule nothing checks will eventually be false. A
documented example nothing compiles is the same thing, and worse, because a
consumer will copy it.

The mechanism is verified, not assumed — I ran it before writing this:

```rust
#[doc = include_str!("../docs/src/migration/v1-to-v2.md")]
#[cfg(doctest)]
pub struct MigrationGuideDoctests;
```

Markdown ```rust blocks then become doctests. A deliberately broken block
**fails** `cargo test --doc`. `#[cfg(doctest)]` means the item exists only
during doctest runs, so it costs nothing at build or run time, and it rides
`cargo test`, which CI already runs across every feature combination on two
platforms. No new tooling, no new dependency, no docs job to add.

Whether that is the right mechanism is unit 01's decision to make and justify —
`mdbook test` is the obvious alternative, and there may be others.

## Fence convention

Set after unit 01, which established it on the migration guide. Units 02 and 03
inherit it:

| Fence | When | Compiled? |
|---|---|---|
| ` ```rust ` | v2 code that compiles and runs | yes, and runs |
| ` ```rust,no_run ` | v2 code needing a real file or workbook | **yes**, not executed |
| ` ```text ` | not Rust-to-be-compiled: v1 shapes, output samples, JSON | no |
| ` ```rust,ignore ` | **avoid** — if used, the prose must say why | no |

**`no_run` is almost always the right answer for "this cannot actually run
here", not `ignore`.** `no_run` still compiles, which is the property NF-024
needs. `ignore` compiles nothing and is indistinguishable from an example nobody
checked — the state unit 01 just fixed.

## Standing constraints

- **No behaviour change.** `src/` changes in this milestone are doc comments and
  the doctest harness only. If documenting something reveals a defect, **stop
  and report** — a documentation milestone that quietly fixes code is not
  documentation.
- **Document what is, not what should be.** Where behaviour is awkward, say so
  plainly. This project has spent two milestones removing statements that
  described an engine better than the one that exists; do not add more.
- **Every example must compile**, or be explicitly marked as non-compiling with
  a reason — per the fence convention above. The point is that the marking is
  deliberate, not accidental. *(Corrected 2026-08-17: this originally said v1
  code "should be `text` or `ignore`", which contradicts the convention unit 01
  established. `text` is the answer for v1 shapes; `ignore` is to be avoided.)*
- **The fixture corpus must not move.**
- Gates as always: fmt, clippy `-D warnings`, the scoped stdout gate, `deny`,
  MSRV 1.88, the full matrix.

## The milestone's exit

**M6 closes with the v3 question**, which is the owner's alone under §6.7. It is
deliberately last: writing NF-026's non-goals and limitations page forces an
inventory of everything the public model promises and the engine does not
deliver — three unreachable `CellValue` variants, four permanently-empty types,
one unreachable `ReadErrorKind`, eleven partially-implemented RFCs. That
inventory is the evidence the decision has been waiting for, and it is a
by-product of work NF-026 requires anyway.

*Corrected 2026-08-17: this said "thirteen partially-implemented RFCs". It is
**eleven** — `grep -l "Partially implemented" rfcs/done/*.md` returns 007, 013,
014, 015, 017, 019, 020, 021, 023, 024, 027. M5 closed 016 and 032 after the
sentence was written, and I did not re-derive the count. Caught by unit 03. The
second time this project has caught me asserting a count I had not checked.*

## Moved out of this milestone

The **v1.2-vs-v2 benchmark comparison** (RFC-027) was grouped here as
documentation debt. It is not documentation work — it requires building v1.2 and
running comparable benchmarks, which is measurement. **It moves to M7**, whose
discipline is measure-before-deciding and whose measurement unit is still owed.
