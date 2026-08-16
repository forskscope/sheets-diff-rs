//! Workbook-level metadata comparison: defined names, sheet visibility (RFC-021).
//!
//! calamine 0.36 exposes:
//!   - `defined_names() -> &[(String, String)]`  — flat (name, target) pairs, no scope
//!   - `sheets_metadata() -> &[Sheet]`            — name, typ, visible
//!
//! Defined-name scope is unavailable; all names match on (normalized_name) only,
//! and a `DefinedNameScopeUnknown` diagnostic is attached to every diff that
//! would need scope to be precise.

use std::collections::BTreeMap;

use calamine::Reader;

use crate::model::{
    Diagnostic, DiagnosticKind, DiagnosticLocation, DiffStage, Severity, WorkbookChange,
};
use crate::open::OpenedWorkbook;
use crate::options::DiffOptions;

// ---------------------------------------------------------------------------
// WorkbookChange variants (RFC-021, RFC-033 §12 reserved field)
// ---------------------------------------------------------------------------
// These are defined in model.rs as a non-exhaustive placeholder; here we
// give them content by constructing concrete instances.

/// Construct `WorkbookChange` values for defined-name diffs and sheet
/// visibility changes.  Metadata comparison always runs unconditionally —
/// `_opts` is unused, and there is no mode to disable or configure it with.
/// RFC-021 designed a `WorkbookMetadataMode` for this; it was never built
/// (RFC-021's Status field records this as deferred).
pub fn compare_workbook_metadata(
    old_wb: &mut OpenedWorkbook,
    new_wb: &mut OpenedWorkbook,
    _opts: &DiffOptions,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<WorkbookChange> {
    // WorkbookChange is a zero-field placeholder; we populate diagnostics
    // instead. Concrete variants are deferred with no version attached —
    // this function is the seam for when they land.

    diff_defined_names(old_wb, new_wb, diagnostics);
    diff_sheet_visibility(old_wb, new_wb, diagnostics);

    // Return empty until WorkbookChange gains concrete variants.
    Vec::new()
}

// ---------------------------------------------------------------------------
// Defined-name diffing
// ---------------------------------------------------------------------------

fn diff_defined_names(
    old_wb: &mut OpenedWorkbook,
    new_wb: &mut OpenedWorkbook,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let old_names: BTreeMap<String, String> = old_wb
        .reader
        .defined_names()
        .iter()
        .map(|(n, t)| (n.to_lowercase(), t.clone()))
        .collect();

    let new_names: BTreeMap<String, String> = new_wb
        .reader
        .defined_names()
        .iter()
        .map(|(n, t)| (n.to_lowercase(), t.clone()))
        .collect();

    // Emit the scope-unknown diagnostic once if any names are present.
    if !old_names.is_empty() || !new_names.is_empty() {
        diagnostics.push(Diagnostic {
            severity: Severity::Info,
            kind: DiagnosticKind::DefinedNameScopeUnknown,
            location: DiagnosticLocation {
                stage: DiffStage::Metadata,
                sheet_order: None,
                sheet_name: None,
                address: None,
            },
            message: "defined-name scope is unavailable in calamine 0.36; \
                      names are matched by normalized name only"
                .into(),
        });
    }

    // Added names
    for (name, target) in &new_names {
        if !old_names.contains_key(name) {
            diagnostics.push(Diagnostic {
                severity: Severity::Info,
                kind: DiagnosticKind::UnsupportedWorkbookMetadata {
                    category: format!("defined_name_added:{name}"),
                },
                location: DiagnosticLocation {
                    stage: DiffStage::Metadata,
                    sheet_order: None,
                    sheet_name: None,
                    address: None,
                },
                message: format!("defined name added: '{name}' → '{target}'"),
            });
        }
    }

    // Removed names
    for (name, target) in &old_names {
        if !new_names.contains_key(name) {
            diagnostics.push(Diagnostic {
                severity: Severity::Info,
                kind: DiagnosticKind::UnsupportedWorkbookMetadata {
                    category: format!("defined_name_removed:{name}"),
                },
                location: DiagnosticLocation {
                    stage: DiffStage::Metadata,
                    sheet_order: None,
                    sheet_name: None,
                    address: None,
                },
                message: format!("defined name removed: '{name}' (was '{target}')"),
            });
        }
    }

    // Changed targets
    for (name, old_target) in &old_names {
        if let Some(new_target) = new_names.get(name)
            && old_target != new_target
        {
            diagnostics.push(Diagnostic {
                severity: Severity::Info,
                kind: DiagnosticKind::UnsupportedWorkbookMetadata {
                    category: format!("defined_name_changed:{name}"),
                },
                location: DiagnosticLocation {
                    stage: DiffStage::Metadata,
                    sheet_order: None,
                    sheet_name: None,
                    address: None,
                },
                message: format!(
                    "defined name '{name}' target changed: '{old_target}' → '{new_target}'"
                ),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Sheet visibility diffing
// ---------------------------------------------------------------------------

fn diff_sheet_visibility(
    old_wb: &mut OpenedWorkbook,
    new_wb: &mut OpenedWorkbook,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let old_vis: BTreeMap<String, String> = old_wb
        .reader
        .sheets_metadata()
        .iter()
        .map(|s| (s.name.clone(), format!("{:?}", s.visible)))
        .collect();

    let new_vis: BTreeMap<String, String> = new_wb
        .reader
        .sheets_metadata()
        .iter()
        .map(|s| (s.name.clone(), format!("{:?}", s.visible)))
        .collect();

    for (name, old_v) in &old_vis {
        if let Some(new_v) = new_vis.get(name)
            && old_v != new_v
        {
            diagnostics.push(Diagnostic {
                severity: Severity::Info,
                kind: DiagnosticKind::UnsupportedWorkbookMetadata {
                    category: format!("sheet_visibility_changed:{name}"),
                },
                location: DiagnosticLocation {
                    stage: DiffStage::Metadata,
                    sheet_order: None,
                    sheet_name: Some(name.clone()),
                    address: None,
                },
                message: format!("sheet '{name}' visibility: {old_v} → {new_v}"),
            });
        }
    }
}
