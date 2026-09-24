# Handoff 04 — Public values that describe what the engine does not do

**Governing RFCs.** RFC-033 (public model lexicon), RFC-018 (formula
comparison), RFC-006 (sheet matching), RFC-035 (resource bounds)
**Roadmap.** M8 — unit 04
**Sequence.** After unit 01 (landed). Parallel with 02 and 03.

## Purpose

Two findings, one shape. **A4**: four `DiagnosticKind` variants that nothing
constructs, each listed in the stable `code()` table as though it could occur.
**O1**: a `SheetMatchReason` whose name states a check the matcher never
performs, on every rename this crate reports.

A value that never appears and a value that appears under a false name are the
same defect for a reader. They belong in one unit.

O1 was found by the implementer while building unit 01's fixtures and folded
here on 2026-09-24. It is the second finding this milestone to arrive from
someone looking at something else.

## Background — A4

Four variants have **zero** construction sites anywhere in the repository —
verified across `src/`, `tests/`, `benches/` and `fuzz/`, including `#[cfg(test)]`
modules. Each appears only in its own declaration and its `code()` arm:

| Variant | Stable code | Why it cannot occur |
|---|---|---|
| `FormulaCachedValueUnverified` | `formula_cached_value_unverified` | Nothing constructs it. **RFC-018 §146 states the engine "may emit" it** — a record that is false. |
| `UnsupportedCellValue { detail }` | `unsupported_cell_value` | Its one historical use was the duplicate-alignment-key condition, and RFC-035 correctly moved that to `DuplicateAlignmentKey`. Nothing replaced it. |
| `DateTimeNotNormalized` | `datetime_not_normalized` | Nothing constructs it. |
| `LimitTruncatedCells { limit, observed }` | `limit_truncated_cells` | **Structurally impossible.** Its documented meaning is that a configured cell limit truncated the comparison; limits return `Err(LimitExceeded)`. There is no truncation path to report. |

`code()`'s own doc comment (`src/model.rs:624`) tells callers these strings are
**the stable programmatic surface** and to match on them rather than on the
enum. A caller who does exactly what we instruct will write four arms that can
never run, and nothing tells them.

## Background — O1

`SheetMatchReason` has three variants. **One is constructed. Two are not.**

```rust
pub enum SheetMatchReason {
    ExactName,          // constructed: nowhere
    IndexAndContent,    // constructed: matcher.rs:143, 151, 212
    ContentSimilarity,  // constructed: nowhere
}
```

`ExactName` is not merely unused — it is **structurally impossible**.
`SheetMatchReason` appears in exactly two places, `SheetChange::Renamed` and
`SheetChange::RenamedAndMoved`, and a sheet matched by exact name is not a
rename.

`ContentSimilarity` is never produced because **the matcher never inspects cell
content at all.**

And the variant that *is* produced is wrong at all three sites:

- **`matcher.rs:212`**, inside a function named `index_match`, pairs sheets on
  `n.index == old.index` and nothing else. Reason recorded: `IndexAndContent`.
- **`matcher.rs:143`**, `conservative_rename` with one unmatched sheet on each
  side and equal indices. Index only.
- **`matcher.rs:151`**, the same function with **unequal** indices — the two
  sheets are paired because they are the only ones left. Reason recorded:
  `IndexAndContent`. Here **neither** half of the name is true: no content was
  compared and no index matched. This is a match by elimination.

So every rename this crate reports carries a reason asserting a content check
that does not exist, and in one case an index match that also does not exist.

## Disposition — and the part that is the owner's call

**A4: document, do not remove.** `DiagnosticKind` is `#[non_exhaustive]`, but
removing a variant still breaks a caller matching on it. Removal is a v3
question.

**O1: document in 2.6.0; rename in v3.** This is my recommendation and the
owner has been asked to confirm it before this unit is worked. The alternative
— adding honest variants now (`IndexOnly`, and something for the elimination
case) and producing them instead — is compile-safe, because `#[non_exhaustive]`
already forces downstream wildcards. I am not recommending it for 2.6.0:

- A consumer matching `IndexAndContent` to detect renames would silently fall
  through to their wildcard. A silent behaviour change is a poor way to fix a
  naming defect.
- It makes consumers migrate twice — once now, once at v3 when the dead variants
  are actually removed.
- The reader's misunderstanding, which is the defect, is cured just as
  completely by a doc comment.

**Do not implement the rename in this unit.** If the owner decides otherwise,
this handoff will be reissued rather than amended — amending a handoff in flight
is a process error this project has already made once.

## Change scope

- `src/model.rs` — doc comments on the four `DiagnosticKind` variants and on all
  three `SheetMatchReason` variants
- `src/matcher.rs` — a comment at each of the three sites; and see §"One piece of
  cruft"
- `rfcs/done/018-*.md` — the false "may emit" sentence
- `rfcs/done/033-*.md` — the lexicon's variant listings
- `rfcs/*/006-*.md` — whatever it says about match reasons
- `docs/` — any user-facing diagnostic-code table
- `CHANGELOG.md`

## Non-change scope

- **No variant is added, removed, renamed or reordered.** Not one.
- **No behaviour changes.** Not which diagnostics are produced, not which reason
  the matcher records, not the matching algorithm.
- Do not change `code()`'s returned strings.
- Do not "fix" the matcher to actually compare content. That is a real proposal
  and it is RFC-006's, not this unit's. If you have a view, write it in the
  review request.

## Required implementation

1. **Reuse M4 unit 01's established wording.** `src/model.rs:239` set the
   pattern for an unreachable public variant:

   > Cannot occur: *(reason specific to this variant)*. A match arm on this
   > variant is unreachable today; it is retained as *(what it is retained
   > for)*, not as a live case.

   Every one of the four A4 variants gets this form, with its **own** reason —
   the table in §Background gives four different reasons, and four copies of one
   sentence would be the same defect in new clothes.

2. **`LimitTruncatedCells` says why it is structurally impossible**, not merely
   unused: limits return `Err`, so there is no truncation to report.

3. **`code()`'s doc comment gains one sentence**: some codes in this table are
   not currently emitted, and each such variant says so on itself. A caller
   following the instruction to match on codes must be able to find that out
   from where the instruction is given.

4. **`SheetMatchReason`'s three variants each get a doc comment:**
   - `ExactName` — never produced, and structurally cannot be: this enum appears
     only inside `Renamed` and `RenamedAndMoved`.
   - `IndexAndContent` — **produced for every rename**, and **no cell content is
     compared**. State plainly that the name is inaccurate, that it means "paired
     by index, or by elimination when one sheet remains on each side", and that
     it will be renamed in the next major version.
   - `ContentSimilarity` — never produced; the matcher does not inspect content.

5. **The enum's own doc comment** ("The reason a non-exact sheet pair was
   formed") says what the matcher actually considers: name, index, and
   elimination. Not content.

6. **A comment at `matcher.rs:151`** specifically, because it is the worst of
   the three: this pair is formed by elimination, and the recorded reason is
   accurate about neither index nor content.

7. **RFC-018's "may emit `DiagnosticKind::FormulaCachedValueUnverified`"** is
   corrected to say it is specified and not implemented. Do not delete the
   sentence — the specification stands; the claim of emission does not.

8. **RFC-033's lexicon** marks which listed variants are live.

## One piece of cruft

`matcher.rs:154` is `let _ = confidence; // used inside change arms` — a dead
binding whose comment explains a use that does not exist. It is two lines from
work you are already doing. Remove it if and only if the build stays clean;
report it either way. This is the one exception to §Non-change scope, and it is
deliberate: leaving a false comment behind in the unit about false statements
would be absurd.

## Required tests

**There is no behaviour to test, and that is the point.** Instead:

- A test asserting `SheetMatchReason::ContentSimilarity` and `ExactName` are not
  produced by any matcher path exercised by the existing corpus, so the doc
  comment has something holding it true.
- A test asserting the corpus produces no diagnostic whose `code()` is one of
  the four dead strings.

Both are **guards on the documentation**, not on behaviour: if someone later
makes one of these live and forgets the doc comment, the test fails and points
at the sentence. Say so in a comment on each test, or the next reader will
delete them as tautologies.

If either test *fails* when you write it, stop — a variant in the table is
reachable and the finding is wrong. Report that instead; it would be the better
outcome.

## Acceptance criteria

1. All four A4 variants documented with the M4 wording and **four distinct
   reasons**.
2. `LimitTruncatedCells` explains structural impossibility.
3. `code()`'s doc comment points at the situation.
4. All three `SheetMatchReason` variants documented; `IndexAndContent` states
   plainly that no content is compared.
5. The enum-level doc comment lists what the matcher does consider.
6. `matcher.rs:151`'s elimination case commented.
7. RFC-018 corrected without deleting the specification.
8. RFC-033 and RFC-006 records agree with the code.
9. **Zero behaviour change**, shown by a byte-identical fixture corpus and an
   unchanged full test run.
10. Both documentation-guard tests present, passing, and commented as to why
    they exist.
11. CHANGELOG under `### Fixed` — a false statement corrected is a fix, not an
    addition. Gates green.

## Prohibited shortcuts

- **Do not remove any variant**, however dead. Removal is v3's.
- **Do not change what the matcher records** to make the name true. That is a
  behaviour change disguised as a doc fix.
- Do not write "reserved for future use" on `LimitTruncatedCells`. It is not
  reserved; it describes something that cannot happen given how limits work.
- Do not mark a variant as unreachable without checking it yourself. My count is
  four; verify it. Counts in my handoffs have been wrong five times, and each
  time the implementer caught it.

## Compatibility constraints

None. No public item's type, name or value changes. The CHANGELOG entry exists
so that a consumer matching on a dead code learns their arm is unreachable.

## Known risks

- `docs/src/` examples execute; a diagnostic-code table there may be asserted on.
- **`UnsupportedCellValue`'s history is documented in CHANGELOG:846 and
  RFC-035.** Do not contradict those; it was correctly *removed* from the
  duplicate-key path, and is dead because nothing took its place — not because
  the removal was wrong.

## Required evidence

- Your own per-variant construction-site search, with the command shown
- Corpus byte-comparison
- Full test run before and after
- The two guard tests' output
- CI run link

## Review request format

Per development policy §9.2. Additionally: report your own count of dead
`DiagnosticKind` variants even if it agrees with mine, and state whether you
think the matcher *should* compare content — that view belongs in RFC-006's next
revision and I would rather have it from whoever just read the code.
