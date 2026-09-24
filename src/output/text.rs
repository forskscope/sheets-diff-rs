//! Human-readable text output renderer (RFC-013).
//!
//! All functions here are pure formatters over `WorkbookDiff`. They do not
//! perform comparison logic and must never write to stdout/stderr directly.

use std::fmt::Write as FmtWrite;

use crate::model::{
    CellDiff, Diagnostic, Severity, SheetChange, SheetDiff, SheetRef, WorkbookDiff,
};

// ---------------------------------------------------------------------------
// Summary report
// ---------------------------------------------------------------------------

/// Render a compact human-readable summary of a `WorkbookDiff`.
pub fn render_summary(diff: &WorkbookDiff) -> String {
    let mut out = String::new();
    let s = &diff.summary;

    let old_name = diff.old.source.display_name.as_deref().unwrap_or("old");
    let new_name = diff.new.source.display_name.as_deref().unwrap_or("new");
    writeln!(out, "sheets-diff: {old_name}  →  {new_name}").unwrap();
    writeln!(
        out,
        "  sheets : {} added, {} removed, {} renamed, {} moved, {} changed",
        s.sheets_added, s.sheets_removed, s.sheets_renamed, s.sheets_moved, s.sheets_changed
    )
    .unwrap();
    writeln!(
        out,
        "  cells  : {} changed ({} value, {} formula)",
        s.cells_changed, s.values_changed, s.formulas_changed
    )
    .unwrap();

    if s.diagnostics.errors + s.diagnostics.warnings > 0 {
        writeln!(
            out,
            "  diagnostics: {} error(s), {} warning(s)",
            s.diagnostics.errors, s.diagnostics.warnings
        )
        .unwrap();
    }

    for sd in &diff.sheets {
        write_sheet_line(&mut out, sd);
    }

    out
}

fn write_sheet_line(out: &mut String, sd: &SheetDiff) {
    let label = sheet_label(sd);
    let tag = match &sd.change {
        SheetChange::Added => " [added]",
        SheetChange::Removed => " [removed]",
        SheetChange::Renamed { .. } => " [renamed]",
        SheetChange::RenamedAndMoved { .. } => " [renamed+moved]",
        SheetChange::Moved => " [moved]",
        SheetChange::Unchanged => return,
        SheetChange::Modified => "",
    };
    let cells = sd.summary.cells_changed;
    if cells > 0 || !tag.is_empty() {
        writeln!(out, "  sheet '{label}'{tag}: {cells} cell(s) changed").unwrap();
    }
}

// ---------------------------------------------------------------------------
// Unified-style diff
// ---------------------------------------------------------------------------

/// Render a unified-style diff showing per-cell old/new values.
pub fn render_unified(diff: &WorkbookDiff) -> String {
    let mut out = String::new();

    let old_name = diff.old.source.display_name.as_deref().unwrap_or("old");
    let new_name = diff.new.source.display_name.as_deref().unwrap_or("new");
    writeln!(out, "--- {old_name}").unwrap();
    writeln!(out, "+++ {new_name}").unwrap();

    for sd in &diff.sheets {
        // A sheet is left out only when nothing about it differs. `Unchanged` is
        // the one variant that means that; every other variant — `Added`,
        // `Removed`, `Moved`, `Renamed`, `RenamedAndMoved`, and any added later —
        // is a difference and is rendered, whether or not a cell changed. (This
        // guard used to admit only `Added` and `Removed`, which dropped a sheet
        // that was merely reordered or renamed and left the renamed-sheet branch
        // below reachable only when a cell had also changed.)
        if matches!(sd.change, SheetChange::Unchanged) && sd.cell_diffs.is_empty() {
            continue;
        }

        let label = sheet_label(sd);
        writeln!(out, "@@ sheet: {label} @@").unwrap();

        let old_name = sd.old_sheet.as_ref().map_or("?", |s| s.name.as_str());
        let new_name = sd.new_sheet.as_ref().map_or("?", |s| s.name.as_str());
        // Positions are 1-based tab positions, as a user counts them; the model's
        // `SheetRef::index` is 0-based.
        let position = |s: &Option<SheetRef>| {
            s.as_ref()
                .map_or("?".to_string(), |r| (r.index + 1).to_string())
        };
        let (old_pos, new_pos) = (position(&sd.old_sheet), position(&sd.new_sheet));

        match &sd.change {
            SheetChange::Added => {
                writeln!(out, "+[sheet added]").unwrap();
            }
            SheetChange::Removed => {
                writeln!(out, "-[sheet removed]").unwrap();
            }
            SheetChange::Renamed { .. } => {
                writeln!(out, " [renamed: '{old_name}' → '{new_name}']").unwrap();
            }
            SheetChange::RenamedAndMoved { .. } => {
                writeln!(
                    out,
                    " [renamed: '{old_name}' → '{new_name}'; moved: position {old_pos} → {new_pos}]"
                )
                .unwrap();
            }
            SheetChange::Moved => {
                writeln!(out, " [moved: position {old_pos} → {new_pos}]").unwrap();
            }
            // `Modified`, or an `Unchanged` sheet that has cell diffs: the cell
            // lines below are the whole story.
            _ => {}
        }

        for cd in &sd.cell_diffs {
            write_cell_diff(&mut out, cd);
        }
    }

    write_diagnostics_section(&mut out, &diff.diagnostics);
    out
}

fn write_cell_diff(out: &mut String, cd: &CellDiff) {
    let addr = &cd.address.a1;
    if let Some(vc) = &cd.value {
        writeln!(out, "-{addr}\t{}", vc.old.display_string()).unwrap();
        writeln!(out, "+{addr}\t{}", vc.new.display_string()).unwrap();
    }
    if let Some(fc) = &cd.formula {
        let old_f = fc.old.as_ref().map(|t| t.raw.as_str()).unwrap_or("");
        let new_f = fc.new.as_ref().map(|t| t.raw.as_str()).unwrap_or("");
        if !old_f.is_empty() || fc.old.is_none() && fc.new.is_some() {
            // formula removed or changed
            writeln!(out, "-{addr}~\t{old_f}").unwrap();
        }
        if !new_f.is_empty() || fc.new.is_none() && fc.old.is_some() {
            writeln!(out, "+{addr}~\t{new_f}").unwrap();
        }
    }
}

fn write_diagnostics_section(out: &mut String, diagnostics: &[Diagnostic]) {
    let shown: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity >= Severity::Warning)
        .collect();
    if shown.is_empty() {
        return;
    }
    writeln!(out, "\n# Diagnostics").unwrap();
    for d in shown {
        let prefix = match d.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
        };
        writeln!(out, "  [{prefix}] {} — {}", d.kind.code(), d.message).unwrap();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sheet_label(sd: &SheetDiff) -> String {
    sd.new_sheet
        .as_ref()
        .or(sd.old_sheet.as_ref())
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "?".into())
}
