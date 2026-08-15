# Handoff 01 — Fixture integrity

**Governing RFC.** [RFC-034](../../proposed/034-build-assurance-and-fixture-integrity.md) §5.1–5.3
**Roadmap.** M1
**Sequence.** Second, after the `parallel` removal. Must precede CI (Handoff 02),
or the first CI run fails on a dirty tree.

## Purpose

Stop the test suite from rewriting the repository, and make the golden corpus
capable of failing.

## Background

`tests/gen.rs::generate_fixtures` is an ordinary `#[test]`. Every `cargo test`
regenerates 14 tracked `.xlsx` files, so a clean checkout becomes dirty simply
by running the suite — visible as fixture byte-churn in commits `9cfd6eb`,
`bd3acac`, `06d4ab3` and `7a32187`. The churn exists because `rust_xlsxwriter`
embeds a creation timestamp.

Separately, `grep` across `tests/`, `src/` and `benches/` finds **no reader of
`expected.json`**. The seven committed golden files are written and never
compared, so RFC-015 and RFC-030's regression protection does not exist.

## Applicable requirements

RFC-015 (fixtures and regression testing), RFC-030 (corpus management),
NF-005 (empty/blank/missing cases tested), §15.2 of the requirements
("prefer programmatic fixture generation", "test both structured result and
formatter output").

## Change scope

- New `examples/gen-fixtures.rs` — the generator
- Delete `tests/gen.rs`
- `tests/integration.rs` — new golden-assertion test; absorb the genuine
  regression assertions currently living inside `generate_fixtures`
- `tests/fixtures/generated/*/` — regenerated once, deterministically, and
  committed
- `tests/fixtures/corpus/README.md` — correct the instructions
- `Cargo.toml` — an `[[example]]` entry if required for feature gating

## Non-change scope

Do **not** change comparison behaviour, the public API, or the *content* of any
fixture scenario. The seven scenarios stay exactly as they are; only how they
are produced and checked changes. Do not add new scenarios in this handoff —
corpus expansion is separate work.

## Required implementation

1. **Move generation out of the harness.** Port `tests/gen.rs` to
   `examples/gen-fixtures.rs`, invoked as
   `cargo run --example gen-fixtures --features serde`.
2. **Make output byte-reproducible.** Pin the document timestamp on every
   generated workbook using `DocProperties::set_creation_datetime` with
   `Workbook::set_properties` (both present in `rust_xlsxwriter` 0.95). Use one
   fixed constant date for all fixtures. Verify by generating twice and
   confirming identical bytes.
3. **Add the golden assertion.** A test that, for each directory under
   `tests/fixtures/generated/`, compares `old.xlsx` with `new.xlsx`, serialises
   via `output::json::to_json_pretty`, and asserts equality with
   `expected.json`. Gate the golden comparison on `--features serde`; without
   the feature still perform the comparison and skip only the assertion.
4. **Add an explicit bless path.** `BLESS=1` rewrites `expected.json` instead of
   asserting. Never automatic, never the default.
5. **Preserve the real assertions.** `generate_fixtures` contains genuine checks
   — `address.a1 == "XFD1"`, `sheets_renamed == 1`, `values_changed == 2`,
   `cells_changed >= 20` for the cascade. These move into the integration suite
   and must keep running.
6. **Resolve the two `#[ignore]`d tests.** `large_workbook_completes_within_limit`
   and `large_workbook_limit_exceeded_cleanly` currently never execute, so
   §15.1's large-workbook requirement is unverified. Either shrink them to run
   by default, or gate them behind a named feature that CI runs explicitly.
   Silently ignored is not an acceptable end state.

## Required tests

- Golden assertion over all seven scenarios
- A test proving generation is idempotent, or a documented CI check that
  regeneration produces no diff
- The migrated assertions from item 5
- The large-workbook tests actually executing under some invocation

## Required documentation updates

`tests/fixtures/corpus/README.md` currently instructs
`cargo test -- generate_fixtures` and `cargo test --features serde -- bless_fixtures`.
The first will no longer exist; the second never did. Replace both with the real
commands.

## Acceptance criteria

1. `cargo test --features serde,chrono,cli` leaves `git status --porcelain` empty.
2. Running the generator twice produces byte-identical output.
3. Hand-editing any `expected.json` causes a test failure — **demonstrate this**,
   do not merely assert it.
4. `BLESS=1` regenerates goldens; without it they are read-only.
5. The assertions from item 5 still run and still pass.
6. The two large-workbook tests execute under a documented invocation.

## Prohibited shortcuts

- Do not delete the golden corpus to make the problem go away.
- Do not make blessing implicit, or on-mismatch-rewrite. A golden that silently
  updates itself is worse than no golden, because it manufactures confidence.
- Do not `#[ignore]` anything new.
- Do not adjust fixture *content* so goldens match. If a golden differs from the
  current implementation's output, that is a **finding** — report it rather than
  blessing over it. Given the known `DateTimeIso` and alignment defects, at
  least one such mismatch is plausible and would be valuable.

## Compatibility constraints

None. Test-harness-only; no public API or behaviour change.

## Security constraints

Fixtures must remain fully synthetic — no customer data, per the existing corpus
policy.

## Known risks

- Pinning the timestamp may not achieve byte-reproducibility if
  `rust_xlsxwriter` embeds other nondeterministic content. Fallback per RFC-034
  §9: treat committed `.xlsx` files as fixed inputs never regenerated in CI and
  assert only on `expected.json`. Report which path was taken.
- Blessing goldens for the first time may capture *current, possibly wrong*
  behaviour. That is acceptable — the corpus's job is to detect change — but any
  output that looks wrong must be reported, not silently frozen.

## Required evidence

- `git status --porcelain` after a full test run, showing empty
- Two generator runs with matching checksums
- A demonstration of the golden test failing on a hand-edited `expected.json`,
  and passing when reverted
- Full test output for all feature combinations

## Review request format

Per the development policy §9.2.
