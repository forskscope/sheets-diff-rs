//! Fatal error type for all fallible v2 entry points (RFC-005, RFC-033 §9).

use std::fmt;

use crate::model::{SheetRef, Side, SourceDescription};

// ---------------------------------------------------------------------------
// Open / read error kinds
// ---------------------------------------------------------------------------

/// Why a workbook could not be opened.
#[non_exhaustive]
#[derive(Debug)]
pub enum OpenErrorKind {
    NotFound,
    PermissionDenied,
    /// The bytes are not a valid ZIP / xlsx container.
    NotXlsx,
    /// Structurally valid ZIP, but xlsx internals are corrupt.
    Corrupt,
    /// File is locked or busy (OS-level).
    Locked,
    Other,
}

impl fmt::Display for OpenErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenErrorKind::NotFound => f.write_str("file not found"),
            OpenErrorKind::PermissionDenied => f.write_str("permission denied"),
            OpenErrorKind::NotXlsx => f.write_str("not an xlsx file"),
            OpenErrorKind::Corrupt => f.write_str("file is corrupt"),
            OpenErrorKind::Locked => f.write_str("file is locked"),
            OpenErrorKind::Other => f.write_str("open failed"),
        }
    }
}

/// Why a sheet could not be read.
#[non_exhaustive]
#[derive(Debug)]
pub enum ReadErrorKind {
    SheetNotFound,
    MalformedSheet,
    Other,
}

impl fmt::Display for ReadErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadErrorKind::SheetNotFound => f.write_str("sheet not found"),
            ReadErrorKind::MalformedSheet => f.write_str("sheet is malformed"),
            ReadErrorKind::Other => f.write_str("read failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Limit kind (RFC-012 / RFC-033 §10)
// ---------------------------------------------------------------------------

/// Which resource limit was exceeded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LimitKind {
    Sheets,
    CellsRead,
    CellsCompared,
    DiffsReturned,
}

impl fmt::Display for LimitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LimitKind::Sheets => f.write_str("max_sheets"),
            LimitKind::CellsRead => f.write_str("max_cells_read"),
            LimitKind::CellsCompared => f.write_str("max_cells_compared"),
            LimitKind::DiffsReturned => f.write_str("max_diffs_returned"),
        }
    }
}

// ---------------------------------------------------------------------------
// Boxed calamine error carrier
// ---------------------------------------------------------------------------

/// Opaque wrapper that owns the original `calamine::XlsxError` so that
/// `SheetsDiffError::source()` can return it without naming calamine in any
/// public signature (RFC-026).
pub struct CalamiLineError(pub(crate) calamine::XlsxError);

impl fmt::Debug for CalamiLineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "calamine error: {}", self.0)
    }
}
impl fmt::Display for CalamiLineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for CalamiLineError {}

// ---------------------------------------------------------------------------
// SheetsDiffError (RFC-033 §9)
// ---------------------------------------------------------------------------

/// Fatal error returned by every v2 entry point.
///
/// The `calamine` source error is preserved behind `std::error::Error::source()`
/// — it never appears in any public variant type.
#[non_exhaustive]
#[derive(Debug)]
pub enum SheetsDiffError {
    /// A workbook could not be opened or parsed.
    OpenWorkbook {
        side: Side,
        source: SourceDescription,
        kind: OpenErrorKind,
        /// Boxed calamine error; accessible via `Error::source()`.
        inner: Option<Box<CalamiLineError>>,
    },
    /// A specific sheet inside an opened workbook could not be read.
    ReadSheet {
        side: Side,
        sheet: SheetRef,
        kind: ReadErrorKind,
        inner: Option<Box<CalamiLineError>>,
    },
    /// The bytes/reader are a valid ZIP but not a recognised xlsx workbook.
    UnsupportedFormat { side: Side, detail: String },
    /// The workbook is password-protected (calamine `XlsxError::Password`).
    EncryptedWorkbook { side: Side },
    /// A `DiffOptions` combination is invalid; detected before any I/O.
    InvalidOptions { detail: String },
    /// The caller's cancellation predicate returned `true`.
    Cancelled,
    /// A configured `Limits` bound was reached.
    LimitExceeded { limit: LimitKind, observed: u64 },
    /// An internal programming error; indicates a bug in `sheets-diff`.
    Internal { detail: String },
}

impl fmt::Display for SheetsDiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SheetsDiffError::OpenWorkbook {
                side, source, kind, ..
            } => {
                let name = source.display_name.as_deref().unwrap_or("<unknown>");
                write!(f, "cannot open {side} workbook '{name}': {kind}")
            }
            SheetsDiffError::ReadSheet {
                side, sheet, kind, ..
            } => {
                write!(
                    f,
                    "cannot read sheet '{}' from {side} workbook: {kind}",
                    sheet.name
                )
            }
            SheetsDiffError::UnsupportedFormat { side, detail } => {
                write!(
                    f,
                    "{side} workbook is not a supported xlsx format: {detail}"
                )
            }
            SheetsDiffError::EncryptedWorkbook { side } => {
                write!(f, "{side} workbook is password-protected")
            }
            SheetsDiffError::InvalidOptions { detail } => {
                write!(f, "invalid options: {detail}")
            }
            SheetsDiffError::Cancelled => f.write_str("comparison was cancelled"),
            SheetsDiffError::LimitExceeded { limit, observed } => {
                write!(f, "limit '{limit}' exceeded (observed {observed})")
            }
            SheetsDiffError::Internal { detail } => {
                write!(f, "internal error: {detail}")
            }
        }
    }
}

impl std::error::Error for SheetsDiffError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SheetsDiffError::OpenWorkbook { inner, .. } => {
                inner.as_deref().map(|e| e as &dyn std::error::Error)
            }
            SheetsDiffError::ReadSheet { inner, .. } => {
                inner.as_deref().map(|e| e as &dyn std::error::Error)
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers (crate-internal)
// ---------------------------------------------------------------------------

impl SheetsDiffError {
    pub(crate) fn open_workbook(
        side: Side,
        source: SourceDescription,
        calamine_err: calamine::XlsxError,
    ) -> Self {
        let kind = classify_open_error(&calamine_err);
        SheetsDiffError::OpenWorkbook {
            side,
            source,
            kind,
            inner: Some(Box::new(CalamiLineError(calamine_err))),
        }
    }

    pub(crate) fn read_sheet(
        side: Side,
        sheet: SheetRef,
        calamine_err: calamine::XlsxError,
    ) -> Self {
        let kind = classify_read_error(&calamine_err);
        SheetsDiffError::ReadSheet {
            side,
            sheet,
            kind,
            inner: Some(Box::new(CalamiLineError(calamine_err))),
        }
    }
}

fn classify_open_error(e: &calamine::XlsxError) -> OpenErrorKind {
    use calamine::XlsxError;
    match e {
        XlsxError::Password => OpenErrorKind::NotXlsx, // reclassified below via EncryptedWorkbook
        XlsxError::FileNotFound(_) => OpenErrorKind::NotFound,
        XlsxError::Io(io) => match io.kind() {
            std::io::ErrorKind::NotFound => OpenErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied => OpenErrorKind::PermissionDenied,
            _ => OpenErrorKind::Other,
        },
        XlsxError::Zip(_) => OpenErrorKind::NotXlsx,
        _ => OpenErrorKind::Corrupt,
    }
}

fn classify_read_error(e: &calamine::XlsxError) -> ReadErrorKind {
    use calamine::XlsxError;
    match e {
        XlsxError::WorksheetNotFound(_) => ReadErrorKind::SheetNotFound,
        _ => ReadErrorKind::MalformedSheet,
    }
}

/// Convert a calamine open error, detecting `Password` to produce the
/// dedicated `EncryptedWorkbook` variant.
pub(crate) fn from_open_error(
    side: Side,
    source: SourceDescription,
    e: calamine::XlsxError,
) -> SheetsDiffError {
    if matches!(e, calamine::XlsxError::Password) {
        SheetsDiffError::EncryptedWorkbook { side }
    } else {
        SheetsDiffError::open_workbook(side, source, e)
    }
}
