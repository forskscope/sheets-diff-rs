# API guide

Five ways in, one way to configure, two ways out. Every example on this
page compiles under the doctest harness `src/lib.rs` builds
(`docs/src/migration/v1-to-v2.md`'s guide explains the mechanism) — none of
them run against real files in CI (that would need a workbook fixture
doctests cannot depend on), so most are `no_run`: compiled, not executed.
That is the property this page exists to guarantee — every snippet below
is checked against the real public API on every push, not merely believed.

---

## Path input — `compare_paths`

The common case: two files on disk.

```rust,no_run
use sheets_diff::compare_paths;

let diff = compare_paths("old.xlsx", "new.xlsx")?;
println!("{} cell(s) changed", diff.summary.cells_changed);
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

Accepts any `AsRef<Path>`, including non-UTF-8 paths on Unix — the path is
never converted to a `String` internally, only the file *name* is, for the
cosmetic `display_name` field (`None` if that name isn't valid UTF-8; see
the [migration guide](migration/v1-to-v2.md) for the full non-panicking
contract).

**Cost:** reads the whole file into memory once (`std::fs::read`), then
holds it for the duration of the comparison. No streaming — a workbook
larger than available memory cannot be compared this way, or any other way
this crate currently offers.

---

## Reader input — `compare_readers`

Any `Read + Seek` — an open `File`, a `Cursor` over bytes you already hold
without wanting to name them as a path, or bytes recovered from a
non-filesystem source (an archive member, a network response already
buffered locally).

```rust,no_run
use sheets_diff::compare_readers;
use std::fs::File;

let old_file = File::open("old.xlsx").expect("open old.xlsx");
let new_file = File::open("new.xlsx").expect("open new.xlsx");
let diff = compare_readers(old_file, new_file)?;
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

`.xlsx` is ZIP-based and requires random access, which is why `Seek` is
required alongside `Read` — a pure forward-only stream cannot be compared.

**Cost:** the same as `compare_paths` once past the open step — the reader
is fully drained into an owned buffer (`read_to_end`) before any parsing
begins, so peak memory is one copy of the file's bytes, same as the path
route.

---

## Bytes input — `compare_bytes`

You already hold the bytes — the case this crate's own GUI-embedding
consumer (ForskScope) uses, handing over bytes it already read for its own
purposes rather than a path this crate would re-read.

```rust,no_run
use sheets_diff::compare_bytes;

# let old_bytes: Vec<u8> = Vec::new();
# let new_bytes: Vec<u8> = Vec::new();
let diff = compare_bytes(&old_bytes, &new_bytes)?;
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

**Cost — the one worth knowing before choosing this over the other two:**
`compare_bytes` copies its input (`to_vec()`) to get an owned buffer it can
build a `Cursor`-based reader over, so **peak memory is roughly double**
the input size while that copy exists — bytes you already hold, plus the
copy this call makes. `compare_paths` and `compare_readers` pay the same
one-copy cost as this call's *second* half, but never hold your original
buffer alongside it, because they read the bytes themselves rather than
receiving them already resident. Recorded as a residual risk in the
[threat model](maintainers/threat-model.md#residual-risks-worth-naming) —
this is a real, current cost, not a hypothetical one, and eliminating it
would need a borrowing reader (`Xlsx<Cursor<&[u8]>>`) that does not exist
today.

---

## Options — `DiffOptions::builder()`

Comparison modes, alignment, and resource limits, in one builder.

```rust,no_run
use sheets_diff::{DiffOptions, FormulaCompareMode, Limits, compare_paths_with_options};

let opts = DiffOptions::builder()
    .formula_compare(FormulaCompareMode::Ignore)
    .limits(Limits::hardened())
    .build()?;

let diff = compare_paths_with_options("old.xlsx", "new.xlsx", opts)?;
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

**Limits are the part easy to skip and expensive to skip.**
`DiffOptions::default()` (and the plain `compare_paths`/`compare_bytes`/
`compare_readers` used above) leaves every *linear* bound —
`max_sheets`, `max_cells_read`, `max_cells_compared`, `max_diffs_returned`
— unset, because their cost scales predictably with input the caller chose
to open, and bounding them by default would surprise code that has always
worked. That default is not a recommendation for untrusted input.
`Limits::hardened()`, set above, bounds every dimension — linear and
superlinear alike — for exactly that case: a workbook that arrived from
somewhere you don't control. Full reasoning and the specific numbers: the
[threat model](maintainers/threat-model.md#the-bounds-themselves-limits).

Every other builder method configures comparison *behaviour*, not
resource bounds — `.formula_compare`, `.format_compare`, `.number_compare`,
`.sheet_matching`, and others; see [`DiffOptionsBuilder`]'s own
documentation for the full list. `.limits(Limits { .. })` also accepts a
hand-built `Limits` value via struct-update syntax
(`Limits { max_sheets: Some(50), ..Limits::default() }`) for bounding one
dimension without adopting `hardened()`'s full preset.

[`DiffOptionsBuilder`]: https://docs.rs/sheets-diff/latest/sheets_diff/struct.DiffOptionsBuilder.html

---

## Formatters

Two text renderers, always available, and a JSON path behind the `serde`
feature.

### Text

```rust,no_run
use sheets_diff::compare_paths;
use sheets_diff::output::text::{render_summary, render_unified};

let diff = compare_paths("old.xlsx", "new.xlsx")?;
let summary = render_summary(&diff);   // compact overview
let unified = render_unified(&diff);   // unified-style per-cell diff
print!("{summary}");
# Ok::<(), sheets_diff::SheetsDiffError>(())
```

Both are pure formatters over an already-computed `WorkbookDiff` — neither
performs comparison work, and neither writes to stdout/stderr itself; they
return `String`, and printing (or not) is the caller's decision.

### JSON — `serde` feature only

```rust,no_run
# #[cfg(feature = "serde")]
# fn example() -> Result<(), sheets_diff::SheetsDiffError> {
use sheets_diff::compare_paths;
use sheets_diff::output::json::{to_json, to_json_pretty};

let diff = compare_paths("old.xlsx", "new.xlsx")?;
let compact = to_json(&diff).expect("serialisable result");
let pretty = to_json_pretty(&diff).expect("serialisable result");
# let _ = (compact, pretty);
# Ok(())
# }
```

Requires the `serde` feature (`sheets-diff = { version = "…", features =
["serde"] }`) — without it, `sheets_diff::output::json` does not exist,
and every public model type carries `Serialize` but not `Deserialize`
(round-tripping a `WorkbookDiff` back into this crate's types is not
supported; the JSON output is for consumption by other tools, not
reconstruction). `to_json`/`to_json_pretty` return `Result<String, String>`
— the error case is serialisation failure, which should not occur for a
well-formed result, but the `Result` is real and worth matching on rather
than `unwrap()`ing in production code.

---

## Error handling

Every entry point returns `Result<WorkbookDiff, SheetsDiffError>`. An
example that only `unwrap()`s — as every example above does, for brevity —
teaches the wrong habit for code embedding this crate somewhere a panic is
not acceptable (a GUI, a long-running service). `SheetsDiffError` is
`#[non_exhaustive]` with eight variants (`OpenWorkbook`, `ReadSheet`,
`UnsupportedFormat`, `EncryptedWorkbook`, `InvalidOptions`, `Cancelled`,
`LimitExceeded`, `Internal`) — matching them individually is optional; the
`Display` impl and `Error::source()` are always available, and
diagnostics carry a stable `code()` for programmatic matching (this
crate's own GUI-embedding consumer's adapter matches on it):

```rust,no_run
use sheets_diff::{SheetsDiffError, compare_paths};

match compare_paths("old.xlsx", "new.xlsx") {
    Ok(diff) => {
        for d in &diff.diagnostics {
            eprintln!("[{}] {}", d.kind.code(), d.message);
        }
    }
    Err(SheetsDiffError::LimitExceeded { limit, observed }) => {
        eprintln!("hit {limit} at {observed}");
    }
    Err(e) => eprintln!("comparison failed: {e}"),
}
```

`diagnostics` (both workbook-level, shown above, and per-sheet on each
`SheetDiff`) are warnings attached to an *successful* comparison — a
missing feature, a coverage gap, an ambiguous match — not failures. An
`Err` means the comparison did not produce a result at all.
