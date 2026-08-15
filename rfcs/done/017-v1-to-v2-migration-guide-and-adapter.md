# RFC-017: v1 to v2 Migration Guide and Adapter

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.0.0 docs  
**Created:** 2026-06-11  
**Category:** Migration  

## 1. Summary

Provide practical migration guidance and helper patterns for existing v1 consumers.

## 2. Motivation

v2 deliberately breaks the public data model. Existing users need a clear path from `Diff::new` and string cell values to the new structured APIs without guessing how to map concepts.

## 3. Goals

- Document equivalent v2 calls for common v1 use cases.
- Show how to flatten v2 output into v1-like app models.
- Show how to get display strings from typed values.
- Explain duplicate-address policy changes.
- Explain CLI and exit-code changes.

## 4. Non-goals

- Do not guarantee source-level compatibility.
- Do not keep the v1 model as the canonical v2 model.
- Do not hide breaking changes behind confusing aliases.

## 5. External design

Migration examples:

```rust
// v1
let diff = sheets_diff::core::diff::Diff::new(old, new);

// v2
let diff = sheets_diff::compare_paths(old, new)?;
```

Flattening example:

```rust
for sheet in &diff.sheets {
    for cell in &sheet.cell_diffs {
        let addr = &cell.address.a1;
        if let Some(change) = &cell.value_change {
            let old = change.old.display_text();
            let new = change.new.display_text();
            // map to app-owned row
        }
    }
}
```

Catch-unwind removal:

```rust
match sheets_diff::compare_paths(old, new) {
    Ok(diff) => { /* render */ }
    Err(err) => { /* show user-friendly error */ }
}
```

## 6. Internal design

The migration guide should be a document, not a compatibility layer. However, optional helper functions may be useful:

```rust
pub fn flatten_cell_changes(diff: &WorkbookDiff) -> impl Iterator<Item = FlatCellChange<'_>>;
```

Such helpers must be clearly secondary to the structured model.

## 7. Data lifecycle

1. Existing user identifies v1 API usage.
2. Guide maps it to v2 API.
3. User adapts result traversal.
4. User decides whether to use display strings or typed values.
5. User updates error handling.

## 8. Error, diagnostic, and edge-case behavior

Migration docs should warn that v2 may show renamed sheets differently than v1 and may merge value/formula changes into one address-level diff.

## 9. Testing and acceptance criteria

Acceptance criteria:

- Migration guide exists before v2.0.0.
- Guide covers constructors, errors, values, sheet changes, cell changes, CLI, and JSON.
- Examples compile as doc tests where possible.
- ForskScope-style adapter example is included without depending on ForskScope code.

## 10. Migration and compatibility

This RFC is all about migration. v1 users who cannot accept breaking changes should stay on v1.2.x until ready.

## 11. Open questions

- Should a `legacy` module exist for one release cycle?
- Should the migration guide include an automated checklist for app developers?
