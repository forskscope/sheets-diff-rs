//! Workbook opening: path, bytes, reader inputs (RFC-004, RFC-032).

use std::io::{Cursor, Read, Seek};
use std::path::Path;

use calamine::{Reader, Xlsx};

use crate::error::{SheetsDiffError, from_open_error};
use crate::model::{SheetRef, Side, SourceDescription, SourceKind};

// ---------------------------------------------------------------------------
// Opened workbook handle
// ---------------------------------------------------------------------------

/// An opened xlsx workbook, holding the calamine reader and sheet metadata.
pub struct OpenedWorkbook {
    pub reader: Xlsx<Cursor<Vec<u8>>>,
    pub sheets: Vec<SheetRef>,
    pub source: SourceDescription,
}

// ---------------------------------------------------------------------------
// Open from path
// ---------------------------------------------------------------------------

/// Open a workbook from a filesystem path.
pub fn open_path(path: impl AsRef<Path>, side: Side) -> Result<OpenedWorkbook, SheetsDiffError> {
    let path = path.as_ref();
    let display_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_owned());
    let source = SourceDescription {
        kind: SourceKind::Path,
        display_name,
    };

    // Read file bytes first so we own them and can build a Cursor-based reader.
    let bytes = std::fs::read(path).map_err(|io_err| {
        let xlsx_err = calamine::XlsxError::Io(io_err);
        from_open_error(side, source.clone(), xlsx_err)
    })?;

    open_bytes_inner(bytes, side, source)
}

// ---------------------------------------------------------------------------
// Open from bytes
// ---------------------------------------------------------------------------

/// Open a workbook from a byte slice or owned bytes.
pub fn open_bytes(
    bytes: impl AsRef<[u8]>,
    side: Side,
    display_name: Option<String>,
) -> Result<OpenedWorkbook, SheetsDiffError> {
    let source = SourceDescription {
        kind: SourceKind::Bytes,
        display_name,
    };
    open_bytes_inner(bytes.as_ref().to_vec(), side, source)
}

// ---------------------------------------------------------------------------
// Open from reader
// ---------------------------------------------------------------------------

/// Open a workbook from an arbitrary `Read + Seek` source.
pub fn open_reader<R: Read + Seek>(
    mut reader: R,
    side: Side,
    display_name: Option<String>,
) -> Result<OpenedWorkbook, SheetsDiffError> {
    let source = SourceDescription {
        kind: SourceKind::Reader,
        display_name,
    };
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|io_err| {
        let xlsx_err = calamine::XlsxError::Io(io_err);
        from_open_error(side, source.clone(), xlsx_err)
    })?;
    open_bytes_inner(bytes, side, source)
}

// ---------------------------------------------------------------------------
// Common inner open
// ---------------------------------------------------------------------------

fn open_bytes_inner(
    bytes: Vec<u8>,
    side: Side,
    source: SourceDescription,
) -> Result<OpenedWorkbook, SheetsDiffError> {
    let cursor = Cursor::new(bytes);
    let wb: Xlsx<Cursor<Vec<u8>>> =
        open_workbook_from_cursor(cursor).map_err(|e| from_open_error(side, source.clone(), e))?;

    let sheet_meta = wb.sheets_metadata().to_vec();
    let sheets: Vec<SheetRef> = sheet_meta
        .into_iter()
        .enumerate()
        .map(|(index, meta)| SheetRef {
            name: meta.name.clone(),
            index,
        })
        .collect();

    Ok(OpenedWorkbook {
        reader: wb,
        sheets,
        source,
    })
}

fn open_workbook_from_cursor(
    cursor: Cursor<Vec<u8>>,
) -> Result<Xlsx<Cursor<Vec<u8>>>, calamine::XlsxError> {
    Xlsx::new(cursor)
}
