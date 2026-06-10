use std::fmt;
use std::path::PathBuf;

#[cfg(feature = "serde_derive")]
use serde::{Deserialize, Serialize};

/// Identifies which workbook (old or new) an error refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub enum WorkbookSide {
    Old,
    New,
}

impl fmt::Display for WorkbookSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkbookSide::Old => write!(f, "old"),
            WorkbookSide::New => write!(f, "new"),
        }
    }
}

/// Structured error type for all fallible `Diff` constructors.
///
/// Each variant carries enough context for a GUI application to display
/// a precise error message identifying which side and which path or sheet
/// caused the failure.
#[derive(Debug)]
pub enum SheetsDiffError {
    /// The workbook file could not be opened or parsed.
    OpenWorkbook {
        side: WorkbookSide,
        path: PathBuf,
        source: calamine::XlsxError,
    },
    /// A workbook supplied as a reader could not be parsed.
    OpenReader {
        side: WorkbookSide,
        source: calamine::XlsxError,
    },
    /// Reading cell values from a named worksheet failed.
    ReadSheetValues {
        side: WorkbookSide,
        sheet: String,
        source: calamine::XlsxError,
    },
    /// Reading cell formulas from a named worksheet failed.
    ReadSheetFormulas {
        side: WorkbookSide,
        sheet: String,
        source: calamine::XlsxError,
    },
}

impl fmt::Display for SheetsDiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SheetsDiffError::OpenWorkbook { side, path, source } => write!(
                f,
                "cannot open {} workbook '{}': {}",
                side,
                path.display(),
                source,
            ),
            SheetsDiffError::OpenReader { side, source } => {
                write!(f, "cannot parse {} workbook from reader: {}", side, source)
            }
            SheetsDiffError::ReadSheetValues { side, sheet, source } => write!(
                f,
                "cannot read values from {} workbook sheet '{}': {}",
                side, sheet, source,
            ),
            SheetsDiffError::ReadSheetFormulas { side, sheet, source } => write!(
                f,
                "cannot read formulas from {} workbook sheet '{}': {}",
                side, sheet, source,
            ),
        }
    }
}

impl std::error::Error for SheetsDiffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SheetsDiffError::OpenWorkbook { source, .. } => Some(source),
            SheetsDiffError::OpenReader { source, .. } => Some(source),
            SheetsDiffError::ReadSheetValues { source, .. } => Some(source),
            SheetsDiffError::ReadSheetFormulas { source, .. } => Some(source),
        }
    }
}
