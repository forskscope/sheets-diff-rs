# Handoff 02 — Source-path privacy

**Governing RFC.** RFC-016 (security, privacy and no side effects)
**Roadmap.** M5
**Sequence.** Any. Independent of 01 and 03.

## Purpose

Test the privacy property `lib.rs` promises in prose and nothing verifies: a
caller's filesystem paths do not appear in a diff result, and a non-UTF-8 path
does not panic.

## Background

RFC-016 states the requirement twice — *"Avoid exposing absolute paths unless
caller opts in"* and the checklist item *"No absolute path in result unless
provided as display name"* — and its own Status line records the gap: *"no test
verifies source-path privacy (non-UTF-8/display-name handling is untested)."*

The implementation is in `src/open.rs`:

```rust
let display_name = path
    .file_name()
    .and_then(|n| n.to_str())
    .map(|s| s.to_owned());
```

So the design is: **file name only, never the directory; `None` rather than a
failure when the name is not UTF-8.**

`src/lib.rs` makes this a documented contract on `compare_paths`, in unusually
specific terms:

> the raw `Path` is passed to `std::fs::read` unchanged — there is **no internal
> `to_str()`/`unwrap()` on the path**, so non-UTF-8 paths (common on Linux) are
> fully supported and never cause a panic. The only UTF-8-dependent step is the
> cosmetic `SourceDescription.display_name`, which is set to `None` for a
> non-UTF-8 file name rather than failing.

Every clause of that is a testable claim. None of them is tested.

This matters more than a cosmetic field suggests, because of who consumes it:
ForskScope embeds this crate in a GUI, their threat model is *users open files
they did not author*, and a diff result that carries `/home/someone/clients/…`
into a rendered view or a log is a privacy leak with the library's name on it.

**The behaviour is believed correct.** This unit tests it. If a test fails, that
is a finding — stop and report rather than changing `src/`.

## Change scope

`tests/` — add a test file or extend an existing one, your choice; say which and
why. Plus `rfcs/done/016-security-privacy-and-no-side-effects-policy.md` (its
Status line, criterion 8) and `CHANGELOG.md` (criterion 9).

**No production code.** That is the constraint that matters here; "tests only"
is the spirit, and the two record files are bookkeeping this unit is responsible
for closing.

## Non-change scope

- **Nothing under `src/`.** If the property does not hold, report it.
- Do not change `display_name`'s semantics, or add an opt-in for full paths.
  RFC-016 §"Should source display names default to file names only…" is an open
  design question, not this unit's business.

## Required implementation

Tests establishing, at minimum:

1. **The directory does not survive.** Compare two fixtures via
   `compare_paths` using a path with at least one parent directory. Assert the
   parent path does not appear in `display_name` — and, more usefully, that it
   appears **nowhere in the whole result**, including rendered output.
2. **A non-UTF-8 file name yields `None` and does not panic**, and the
   comparison still succeeds. This is the clause `lib.rs` is most specific
   about.
3. **Error paths do not leak either.** `src/error.rs:152` formats
   `display_name.as_deref().unwrap_or("<unknown>")`. Trigger a failure on a
   nested path — a missing file is enough — and assert the rendered error
   carries the file name or `<unknown>`, never the directory.
4. **Byte and reader inputs carry no path at all** unless the caller supplied a
   display name.

## Required tests

The four above are the tests. Two things about how to write them:

**Assert on absence carefully.** A test that checks
`!display_name.contains("/tmp/…")` passes trivially if `display_name` is `None`
for an unrelated reason. Assert the positive shape too — that it *is* the file
name — so the test cannot pass by the value being empty.

**Item 2 is Unix-only.** Constructing a non-UTF-8 file name needs
`std::os::unix::ffi::OsStrExt`, and CI runs Windows. Gate it `#[cfg(unix)]`.
**Say in the review request what the Windows leg therefore does not cover** —
that is a real coverage boundary, and this milestone is about knowing where
those are rather than discovering them later.

## Acceptance criteria

1. A test proves a parent directory does not reach `display_name`, and asserts
   the positive value rather than only an absence.
2. A test proves the directory does not appear anywhere in the result, rendered
   output included.
3. A `#[cfg(unix)]` test proves a non-UTF-8 file name gives `display_name ==
   None`, does not panic, and still produces a successful comparison.
4. A test proves an error rendered from a nested path does not carry the
   directory.
5. A test proves byte/reader inputs carry no path.
6. The review request states what the Windows leg does not cover.
7. Nothing under `src/` changed.
8. RFC-016's Status line records this deferral closed. If unit 01 has not landed
   yet, leave its half open — do not mark the line wholly resolved.
9. CHANGELOG under `### Added` — coverage added, nothing fixed.
10. Gates green, full matrix.

## Prohibited shortcuts

- Do not assert only that `display_name` is `Some(_)`. The property is *which*
  string it is.
- Do not test the privacy property by reading `display_name` alone. The claim is
  about the result, and rendered output is where a leak would actually surface.
- Do not `unwrap()` your way past a non-UTF-8 construction failure in the test
  and leave it silently skipped.

## Known risks

- Creating a file with a non-UTF-8 name can fail on some filesystems. If it
  does, the test must fail or skip **loudly** — a silently-skipped privacy test
  is the milestone's own defect class.
- Rendered-output assertions are brittle if they pin whole strings. Assert that
  the directory substring is absent and the file name present, not an exact
  rendering.

## Required evidence

- The tests and their output
- A statement of the Windows coverage boundary
- `git status` showing `tests/` only
- CI run link, both platforms

## Review request format

Per development policy §9.2, plus the Windows-coverage statement.
