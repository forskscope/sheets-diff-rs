//! Non-cell workbook object detection and coverage diagnostics (RFC-023).
//!
//! calamine 0.35 does not expose object content (charts, images, comments,
//! tables, pivot tables, hyperlinks, or data validation) through its public
//! API. What it does expose is:
//!   - `Sheet.typ: SheetType` — distinguishes WorkSheet, ChartSheet, MacroSheet, Vba
//!   - `Sheet.visible: SheetVisible`
//!
//! The policy for v2.2 is `WarnIfPresent` for non-worksheet sheet types and
//! a single coverage diagnostic explaining what is NOT compared. This prevents
//! a misleading "no differences" result when meaningful objects are present.

use calamine::{Reader, SheetType};

use crate::model::{Diagnostic, DiagnosticKind, DiagnosticLocation, DiffStage, Severity};
use crate::open::OpenedWorkbook;

// ---------------------------------------------------------------------------
// ObjectCompareMode (RFC-023 §6)
// ---------------------------------------------------------------------------

/// Controls how the presence of non-cell objects is handled.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ObjectCompareMode {
    /// Ignore objects entirely — no diagnostics.
    Ignore,
    /// Emit a coverage warning when non-worksheet sheets or any object
    /// categories that cannot be compared are detected. Default.
    #[default]
    WarnIfPresent,
    /// Compare what is available; emit diagnostics for the rest.
    /// In v2.2 this behaves identically to `WarnIfPresent` because no
    /// object content API is available in calamine 0.35.
    CompareAvailable,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Detect non-cell objects on both workbook sides and emit coverage diagnostics.
pub fn report_object_coverage(
    old_wb: &mut OpenedWorkbook,
    new_wb: &mut OpenedWorkbook,
    mode: ObjectCompareMode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if mode == ObjectCompareMode::Ignore {
        return;
    }

    detect_non_worksheet_sheets(old_wb, diagnostics);
    detect_non_worksheet_sheets(new_wb, diagnostics);

    // Emit a single blanket coverage note so consumers know what was NOT compared.
    emit_coverage_note(diagnostics);
}

// ---------------------------------------------------------------------------
// Non-worksheet sheet detection
// ---------------------------------------------------------------------------

fn detect_non_worksheet_sheets(wb: &mut OpenedWorkbook, diagnostics: &mut Vec<Diagnostic>) {
    for (index, sheet) in wb.reader.sheets_metadata().iter().enumerate() {
        let kind = match sheet.typ {
            SheetType::ChartSheet => Some("chart sheet"),
            SheetType::MacroSheet => Some("macro sheet"),
            SheetType::Vba => Some("VBA module"),
            SheetType::DialogSheet => Some("dialog sheet"),
            SheetType::WorkSheet => None, // ordinary — no warning needed
        };
        if let Some(kind_label) = kind {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                kind: DiagnosticKind::UnsupportedWorkbookFeature {
                    feature: kind_label.to_owned(),
                },
                location: DiagnosticLocation {
                    stage: DiffStage::Metadata,
                    sheet_order: Some(index),
                    sheet_name: Some(sheet.name.clone()),
                    address: None,
                },
                message: format!(
                    "sheet '{}' is a {} — content not compared \
                     (calamine 0.35 does not expose {} data)",
                    sheet.name, kind_label, kind_label
                ),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Blanket coverage note
// ---------------------------------------------------------------------------

fn emit_coverage_note(diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.push(Diagnostic {
        severity: Severity::Info,
        kind: DiagnosticKind::UnsupportedWorkbookFeature {
            feature: "non-cell objects".to_owned(),
        },
        location: DiagnosticLocation {
            stage: DiffStage::Metadata,
            sheet_order: None,
            sheet_name: None,
            address: None,
        },
        message: "charts, images, comments, hyperlinks, tables, pivot tables, \
                  data validation, and conditional formatting are not compared \
                  in this version (calamine 0.35 does not expose object content)"
            .into(),
    });
}
