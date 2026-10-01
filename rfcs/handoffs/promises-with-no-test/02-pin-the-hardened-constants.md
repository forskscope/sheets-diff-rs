# Handoff 02 — Pin `hardened()`'s constants, so its documented boundary cannot drift

**Written:** 2026-10-01, from unit 01's survey. **Small.**

## Purpose

`Limits::hardened()`'s doc comment carries a table of row counts — ~357,000 rows per side at seven
columns, ~250,000 at ten, ~125,000 at twenty — and the worked example that a 500,000-row ledger
reads 7,000,014 cells and is refused. **All of it is arithmetic on one constant that no test pins.**

## Background

Unit 01's survey called this an ordinary missing test needing a half-million-row fixture, and
proposed one deliberately `#[ignore]`d scale test. **Both halves of that are wrong, and the gap is
smaller than it looks.**

**The mechanism is already pinned.** `tests/cells_read.rs:257`,
`the_count_is_cumulative_across_sheets_and_sides()`, asserts that `max_cells_read` counts populated
cells across both sheets *and sides* — which is the load-bearing claim, the one that surprised a
consumer, and the reason the per-side boundary is half the constant. It is pinned at small scale,
where it belongs.

**What is not pinned is the constant.** `tests/integration.rs:1206`,
`limits_hardened_bounds_every_dimension()`, asserts only `.is_some()` on each field. Change
`max_cells_read` from `5_000_000` to `50_000_000` and nothing fails — while `hardened()`'s doc
table, the threat model's boundary paragraph, and the 7,000,014-cell example all become wrong
silently.

**So the table is arithmetic over one pinned mechanism and one unpinned input.** Pin the input and
the table follows. No large fixture, and **no `#[ignore]`d test** — an ignored test is a check that
cannot fail, which is the anti-pattern M9 removed twice this quarter (`continue-on-error`, and a
mutating fuzz job whose red meant nothing).

## Change scope

- `tests/integration.rs` — extend `limits_hardened_bounds_every_dimension`, or add beside it.
- Nothing else. No `src/`, no doc edits.

## Non-change scope

- **Do not change any `hardened()` value.** RFC-035 chose them; this pins them.
- **Do not add a scale fixture**, ignored or otherwise.
- Do not re-pin the both-sides mechanism — `the_count_is_cumulative_across_sheets_and_sides` has it.
  **Read that test first** and say in the review request that you did.

## Required implementation

**1. Assert the values, not their presence.** Every `hardened()` field, by exact value. The existing
`.is_some()` assertions can stay or be replaced — your call, but a test named
*"bounds every dimension"* that now also fixes the numbers should say so in its name or its comment.

**2. Say what the assertion is for.** A comment naming what depends on these numbers: `hardened()`'s
own doc table, the threat model's boundary paragraph, and that the per-side figure is the constant
halved *because* the count is cumulative across sides. Someone changing a value should be told what
documentation they have just falsified, in the failure they see.

**3. Do the same for `Limits::default()`'s two bounded fields** if they are also only
presence-checked — `default_limits_bound_alignment_and_input_but_not_linear_paths` is next to it.
`DEFAULT_MAX_ALIGNMENT_PRODUCT` is a public constant whose own doc comment carries the ~0.25 s fill
figure and the ~95 MiB table size, so the same argument applies. Check before changing.

## Required tests

The assertions above. **Failing-first:** change one constant in `src/options.rs`, show the test
fails, restore `cmp`-identical. One demonstration is enough; the mechanism is identical for each
field.

## Acceptance criteria

1. Every `hardened()` field asserted by exact value.
2. `Limits::default()`'s bounded fields likewise, or a stated reason.
3. The comment from §2 present.
4. Failing-first demonstrated and restored.
5. `git diff src/` empty. Gates as always, plus rule 003.

## Prohibited shortcuts

- No `#[ignore]`. If a test cannot run in the ordinary gate, it is not the right test here.
- Do not assert a value you have not read out of `src/options.rs` at the time of writing.
- Do not "simplify" by asserting the whole struct with one `assert_eq!` on a `Debug` string — a
  format change would then fail it for the wrong reason.

## Known risks

**1. This test will need editing whenever a value legitimately changes**, which is the point:
changing a documented bound should require touching a test that names the documentation depending
on it. If that feels like friction, the friction is the feature.

**2. `max_cells_compared` is also 5,000,000** and is a different dimension from `max_cells_read`.
Assert both; do not collapse them because the numbers coincide today.

## Required evidence

Under `.git-exclude/review-request/promises-02-pin-the-hardened-constants/evidence/`:

1. The values as read from `src/options.rs`, and the test.
2. The failing-first run and `cmp` restore.
3. Whether `Limits::default()` needed the same, with what you found.
4. Confirmation you read `the_count_is_cumulative_across_sheets_and_sides` and did not duplicate it.
5. Gate sweep, one scratch dir, deleted. `git diff src/`.

## Review request format

`.git-exclude/review-request/promises-02-pin-the-hardened-constants/README.md`, with:

- What you pinned and what you deliberately did not.
- Anything else in `src/` whose documented consequence rests on an unpinned constant. This is the
  second survey; the first found four items and I would rather this one came back longer than
  shorter.
