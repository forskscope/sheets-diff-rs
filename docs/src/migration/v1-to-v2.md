# Migrating from v1 to v2

v2 deliberately breaks the v1 public API. The data model, error type, and
entry points have all changed. This guide maps every common v1 pattern to its
v2 equivalent.

---

## Quick reference

| v1 | v2 |
|---|---|
| `Diff::new(old, new)` | `compare_paths(old, new)?` |
| `Diff::try_new(old, new)?` | `compare_paths(old, new)?` |
| `diff.sheet_diff` | `diff.sheets` (Vec<SheetDiff>) |
| `diff.cell_diffs` | `diff.sheets[i].cell_diffs` |
| `SheetDiff { old: Option<String>, new: Option<String> }` | `SheetDiff { change: SheetChange, old_sheet, new_sheet, … }` |
| `CellDiff { old: Option<String>, new: Option<String>, kind: CellDiffKind }` | `CellDiff { value: Option<ValueChange>, formula: Option<FormulaChange>, … }` |
| `CellDiffKind::Value / Formula` (two entries per address) | One `CellDiff` per address; `value` and `formula` are independent sub-fields |
| `println!` warnings from the library | `WorkbookDiff.diagnostics: Vec<Diagnostic>` |
| panic on bad input | `Err(SheetsDiffError::…)` |
| `unified_diff()` | `sheets_diff::output::text::render_unified(&diff)` |

---

## Entry points

```rust
// v1 — panicking constructor, path strings
let diff = sheets_diff::core::diff::Diff::new("old.xlsx", "new.xlsx");

// v1 — fallible constructor (v1.2+)
let diff = sheets_diff::core::diff::Diff::try_new("old.xlsx", "new.xlsx")?;

// v2 — fallible always; accepts any AsRef<Path>
use sheets_diff::compare_paths;
let diff = compare_paths("old.xlsx", "new.xlsx")?;
```

v2 also accepts byte slices and readers — useful when you already hold the
file contents:

```rust
use sheets_diff::{compare_bytes, compare_readers};

let diff = compare_bytes(&old_bytes, &new_bytes)?;
let diff = compare_readers(old_file, new_file)?;
```

---

## Sheet changes

v1 represented sheet-level changes as:

```rust
// v1
pub struct SheetDiff {
    pub old: Option<String>,   // sheet name on old side, or None if added
    pub new: Option<String>,   // sheet name on new side, or None if removed
}
```

v2 uses explicit variants:

```rust
// v2
for sheet in &diff.sheets {
    match &sheet.change {
        SheetChange::Added   => { /* new_sheet is Some */ }
        SheetChange::Removed => { /* old_sheet is Some */ }
        SheetChange::Renamed { confidence, .. } => {
            let from = sheet.old_sheet.as_ref().unwrap();
            let to   = sheet.new_sheet.as_ref().unwrap();
            println!("renamed '{}' → '{}' ({confidence:?})", from.name, to.name);
        }
        SheetChange::Modified | SheetChange::Unchanged => { /* same name */ }
        _ => {}
    }
}
```

---

## Cell changes

### v1: two separate entries per address

v1 emitted up to two `CellDiff` entries for the same address — one with
`kind = Value` and one with `kind = Formula`:

```rust
// v1
for cell in &diff.cell_diffs {
    println!("{} {:?}: {:?} → {:?}",
        cell.addr, cell.kind, cell.old, cell.new);
}
```

### v2: one entry per address, two sub-fields

v2 merges both into a single `CellDiff`:

```rust
// v2
for sheet in &diff.sheets {
    for cell in &sheet.cell_diffs {
        let addr = &cell.address.a1;

        if let Some(vc) = &cell.value {
            println!("{addr}: {} → {}", vc.old.display_string(), vc.new.display_string());
        }
        if let Some(fc) = &cell.formula {
            let old_f = fc.old.as_ref().map(|t| t.raw.as_str()).unwrap_or("(none)");
            let new_f = fc.new.as_ref().map(|t| t.raw.as_str()).unwrap_or("(none)");
            println!("{addr}~: {old_f} → {new_f}");
        }
    }
}
```

---

## Cell values: strings → typed enum

v1 stored `old: Option<String>` and `new: Option<String>`.

v2 preserves the spreadsheet type:

```rust
use sheets_diff::CellValue;

// v2 — getting a display string (equivalent to v1's string)
if let Some(vc) = &cell.value {
    let old_str = vc.old.display_string();
    let new_str = vc.new.display_string();
}

// v2 — checking the type
match &vc.new {
    CellValue::Text(s)    => { /* string cell */ }
    CellValue::Integer(i) => { /* integer — note: Text("100") ≠ Integer(100) */ }
    CellValue::Number(f)  => { /* float */ }
    CellValue::Bool(b)    => { /* boolean */ }
    CellValue::DateTime(dt) => { /* date/time serial */ }
    CellValue::Error(e)   => { /* formula error, e.g. #REF! */ }
    CellValue::Empty      => { /* explicitly empty */ }
    _ => {}
}
```

**Important:** `Text("100")` and `Integer(100)` are considered *different* in
v2. If you relied on v1's string equality to compare numeric cells, you may
now see additional diffs. Use `ValueDifferenceKind` to filter by reason, or
set `TypeMismatchPolicy::CompareDisplayString` in `DiffOptions` to restore
display-string comparison.

---

## Errors and diagnostics

v1 panicked on many bad inputs. v2 returns structured errors:

```rust
use sheets_diff::SheetsDiffError;

match compare_paths("missing.xlsx", "other.xlsx") {
    Err(SheetsDiffError::OpenWorkbook { side, kind, .. }) => {
        eprintln!("could not open {side} workbook: {kind}");
    }
    Err(SheetsDiffError::EncryptedWorkbook { side }) => {
        eprintln!("{side} workbook is password-protected");
    }
    Err(e) => eprintln!("error: {e}"),
    Ok(diff) => { /* … */ }
}
```

v1 wrote warnings to stdout/stderr. v2 attaches them to the result:

```rust
for d in &diff.diagnostics {
    eprintln!("[{}] {}", d.kind.code(), d.message);
}
// Also available per-sheet:
for sheet in &diff.sheets {
    for d in &sheet.diagnostics { /* … */ }
}
```

---

## Text output

```rust
// v1
let text = diff.unified_diff();

// v2
use sheets_diff::output::text::{render_summary, render_unified};
let summary = render_summary(&diff);   // compact overview
let unified = render_unified(&diff);   // unified-style per-cell diff
```

---

## CLI exit codes

| Condition | v1 | v2 |
|---|---|---|
| No differences | 0 | 0 |
| Differences found | — (always 0) | 1 |
| Operational error | panic / unhandled | 2 |

---

## Flattening v2 output into a v1-style list

If your application expects a flat list of `(addr, old_str, new_str)` triples:

```rust
struct FlatChange {
    sheet: String,
    addr:  String,
    old:   String,
    new:   String,
}

fn flatten(diff: &sheets_diff::WorkbookDiff) -> Vec<FlatChange> {
    let mut out = Vec::new();
    for sheet in &diff.sheets {
        let name = sheet.new_sheet.as_ref()
            .or(sheet.old_sheet.as_ref())
            .map(|s| s.name.as_str())
            .unwrap_or("?")
            .to_owned();
        for cell in &sheet.cell_diffs {
            if let Some(vc) = &cell.value {
                out.push(FlatChange {
                    sheet: name.clone(),
                    addr:  cell.address.a1.clone(),
                    old:   vc.old.display_string(),
                    new:   vc.new.display_string(),
                });
            }
        }
    }
    out
}
```
