# RFC 004 — Reader-Based and Path-Safe Input Constructors

**Status.** Implemented (1.2.0)
**Tracks.** App-owned I/O and non-UTF-8 path safety.
**Touches.** `src/core/diff.rs`, `tests/constructors.rs`.

## Summary

Added `Diff::try_new(impl AsRef<Path>, ...)` for path-safe construction
without `to_str().unwrap()`, and `Diff::try_from_named_readers(name, R, name, R)`
for construction from already-opened `Read + Seek` streams.

## Delivered API

```rust
impl Diff {
    /// Path-safe fallible constructor. Accepts &str, String, PathBuf, etc.
    pub fn try_new(
        old_filepath: impl AsRef<std::path::Path>,
        new_filepath: impl AsRef<std::path::Path>,
    ) -> Result<Self, SheetsDiffError>;

    /// Reader-based fallible constructor.
    /// old_name / new_name populate Diff.old_filepath / Diff.new_filepath.
    pub fn try_from_named_readers<R1, R2>(
        old_name: impl Into<String>,
        old_reader: R1,
        new_name: impl Into<String>,
        new_reader: R2,
    ) -> Result<Self, SheetsDiffError>
    where
        R1: std::io::Read + std::io::Seek,
        R2: std::io::Read + std::io::Seek;
}
```

## Example usage

```rust
// GUI app with owned bytes
use std::io::Cursor;
let diff = Diff::try_from_named_readers(
    "old.xlsx", Cursor::new(old_bytes),
    "new.xlsx", Cursor::new(new_bytes),
)?;

// VCS checkout without lossy path conversion
let diff = Diff::try_new(old_path_buf, new_path_buf)?;
```

## Internal design

Both constructors share `Diff::try_from_workbooks`, the private engine
that receives already-opened `Xlsx<R>` values and their display labels.
No diff logic is duplicated.

## Open questions

None.
