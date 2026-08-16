# RFC-004: Input Sources and Workbook Opening

**Status.** Implemented (2.0.0–2.2.3) — verified 2026-08-16 against the implementation.
**Target:** v2.0.0  
**Created:** 2026-06-11  
**Category:** I/O API  

## 1. Summary

Introduce caller-controlled input sources for paths, byte buffers, and `Read + Seek` readers.

## 2. Motivation

Local-first applications often already own file bytes or have non-UTF-8 paths. v1 requires `&str` paths and reopens files internally, which creates lossy path conversion and redundant I/O. v2 should let callers provide the source they already have.

## 3. Goals

- Support `AsRef<Path>` path inputs.
- Support borrowed and owned bytes.
- Support `Read + Seek` readers.
- Keep source display names caller-controlled.
- Map open/read failures into structured errors.

## 4. Non-goals

- Do not implement network fetching.
- Do not store input bytes after comparison unless explicitly needed.
- Do not require non-UTF-8 paths to be converted to strings.

## 5. External design

Public API examples:

```rust
pub fn compare_paths(
    old: impl AsRef<Path>,
    new: impl AsRef<Path>,
) -> Result<WorkbookDiff, SheetsDiffError>;

pub fn compare_bytes(
    old: impl AsRef<[u8]>,
    new: impl AsRef<[u8]>,
) -> Result<WorkbookDiff, SheetsDiffError>;

pub fn compare_readers<R1, R2>(
    old: R1,
    new: R2,
) -> Result<WorkbookDiff, SheetsDiffError>
where
    R1: Read + Seek,
    R2: Read + Seek;
```

Option-based API:

```rust
pub enum WorkbookSource<'a> {
    Path(&'a Path),
    Bytes(&'a [u8]),
    Reader(Box<dyn ReadSeek + 'a>),
}
```

Because stable Rust cannot directly use multiple non-auto traits in trait objects without an alias workaround, define an internal or public helper trait:

```rust
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}
```

## 6. Internal design

Internal source handling:

```rust
struct SourceSpec<'a> {
    side: Side,
    source: WorkbookSource<'a>,
    display_name: Option<String>,
}

struct OpenedWorkbook<R> {
    side: Side,
    workbook: calamine::Xlsx<R>,
    description: SourceDescription,
}
```

For bytes, wrap with `std::io::Cursor`. For paths, use `File` and `BufReader<File>`. For readers, pass through to `calamine::Xlsx::new`.

## 7. Data lifecycle

1. Caller provides paths, bytes, or readers.
2. API converts them to `SourceSpec`.
3. Source is opened into a calamine workbook.
4. Opening errors become `SheetsDiffError::OpenWorkbook`.
5. Source metadata is reduced to safe `SourceDescription`.
6. The workbook is passed to normalization.

## 8. Error, diagnostic, and edge-case behavior

Path errors include side and path. Bytes/reader errors include side and optional display name, but no fabricated path.

Password-protected or unsupported workbooks should be returned as structured open/read errors. The library must not try to bypass protection.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Path API works with `PathBuf` and non-UTF-8 paths where the platform supports them.
- Bytes API works with in-memory `.xlsx` fixtures.
- Reader API works with `Cursor<Vec<u8>>` and `BufReader<File>`.
- Missing and corrupt inputs return structured errors.
- No API requires `path.display().to_string()`.

## 10. Migration and compatibility

v1 callers using `Diff::new(&old_str, &new_str)` migrate to `compare_paths(old_path, new_path)`. Apps that already read files can migrate to `compare_bytes` to avoid double I/O.

## 11. Open questions

- Should `WorkbookSource` be public in v2.0 or kept internal behind helper functions?
- Should async readers be explicitly out of scope, or reserved for a future feature?
