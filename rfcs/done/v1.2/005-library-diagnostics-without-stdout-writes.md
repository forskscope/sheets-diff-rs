# RFC 005 — Library Diagnostics Without Stdout Writes

**Status.** Implemented (v1.2.0)
**Tracks.** Library hygiene and GUI integration.
**Touches.** `src/core/diff.rs`, `src/main.rs`.

## Summary

Removed all `println!` calls from library code. Sheet read failures are
now returned through `SheetsDiffError` in the fallible API. `Diff::new`
panics (via `try_new(...).expect(...)`) instead of printing to stdout.
The CLI prints to stderr at the executable boundary.

## Problem removed

```rust
// v1.1.4 — library writing to stdout:
} else {
    println!("Failed to read sheet: {}", sheet);
}
```

## Strict fallible behavior

```rust
// v1.2.0 — structured errors instead:
let old_range = old_workbook
    .worksheet_range(sheet)
    .map_err(|source| SheetsDiffError::ReadSheetValues {
        side: WorkbookSide::Old,
        sheet: sheet.clone(),
        source,
    })?;
```

## CI hygiene check

```bash
! grep -R "println!\|eprintln!\|dbg!" src/core src/lib.rs
```

This check passes on v1.2.0.

## CLI boundary

`src/main.rs` is the only place that prints — to stderr on error and to
stdout for normal diff output. This is appropriate for an executable.

## Open questions

None.
