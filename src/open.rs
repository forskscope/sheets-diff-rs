//! Workbook opening: path, bytes, reader inputs (RFC-004, RFC-032).

use std::io::{Cursor, Read, Seek};
use std::path::Path;

use calamine::{Reader, Xlsx};

use crate::error::{LimitKind, SheetsDiffError, from_open_error};
use crate::model::{SheetRef, Side, SourceDescription, SourceKind};

// ---------------------------------------------------------------------------
// Opened workbook handle
// ---------------------------------------------------------------------------

/// An opened xlsx workbook, holding the calamine reader and sheet metadata.
pub struct OpenedWorkbook {
    pub reader: Xlsx<Cursor<Vec<u8>>>,
    pub sheets: Vec<SheetRef>,
    pub source: SourceDescription,
    /// Workbook-level date epoch flag (RFC-019 / D-02). Read once here via
    /// `Xlsx::has_1904_epoch()` rather than per cell — the flag is a workbook
    /// property, not a per-cell one.
    pub is_1904: bool,
}

// ---------------------------------------------------------------------------
// Open from path
// ---------------------------------------------------------------------------

/// Open a workbook from a filesystem path.
///
/// RFC-035 §5.4: when `max_input_bytes` is set, the file size is checked via
/// `fs::metadata` *before* `fs::read` — an oversized input is rejected
/// without ever being read into memory.
pub fn open_path(
    path: impl AsRef<Path>,
    side: Side,
    max_input_bytes: Option<u64>,
) -> Result<OpenedWorkbook, SheetsDiffError> {
    let path = path.as_ref();
    let display_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_owned());
    let source = SourceDescription {
        kind: SourceKind::Path,
        display_name,
    };

    if let Some(limit) = max_input_bytes {
        let len = std::fs::metadata(path)
            .map_err(|io_err| {
                let xlsx_err = calamine::XlsxError::Io(io_err);
                from_open_error(side, source.clone(), xlsx_err)
            })?
            .len();
        if len > limit {
            return Err(SheetsDiffError::LimitExceeded {
                limit: LimitKind::InputBytes,
                observed: len,
            });
        }
    }

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
///
/// RFC-035 §5.4: when `max_input_bytes` is set, it is checked against
/// `bytes.len()` before the owning copy (`to_vec()`) is made.
pub fn open_bytes(
    bytes: impl AsRef<[u8]>,
    side: Side,
    display_name: Option<String>,
    max_input_bytes: Option<u64>,
) -> Result<OpenedWorkbook, SheetsDiffError> {
    let source = SourceDescription {
        kind: SourceKind::Bytes,
        display_name,
    };
    let bytes = bytes.as_ref();
    if let Some(limit) = max_input_bytes {
        let len = bytes.len() as u64;
        if len > limit {
            return Err(SheetsDiffError::LimitExceeded {
                limit: LimitKind::InputBytes,
                observed: len,
            });
        }
    }
    open_bytes_inner(bytes.to_vec(), side, source)
}

// ---------------------------------------------------------------------------
// Open from reader
// ---------------------------------------------------------------------------

/// Open a workbook from an arbitrary `Read + Seek` source.
///
/// RFC-035 §5.4: when `max_input_bytes` is set, the `Seek` bound lets us
/// measure the source's length (seek to end, then back to start) *before*
/// `read_to_end`, so an oversized input is rejected without reading any of
/// its bytes into memory.
pub fn open_reader<R: Read + Seek>(
    mut reader: R,
    side: Side,
    display_name: Option<String>,
    max_input_bytes: Option<u64>,
) -> Result<OpenedWorkbook, SheetsDiffError> {
    let source = SourceDescription {
        kind: SourceKind::Reader,
        display_name,
    };

    if let Some(limit) = max_input_bytes {
        let len = (|| -> std::io::Result<u64> {
            let end = reader.seek(std::io::SeekFrom::End(0))?;
            reader.seek(std::io::SeekFrom::Start(0))?;
            Ok(end)
        })()
        .map_err(|io_err| {
            let xlsx_err = calamine::XlsxError::Io(io_err);
            from_open_error(side, source.clone(), xlsx_err)
        })?;
        if len > limit {
            return Err(SheetsDiffError::LimitExceeded {
                limit: LimitKind::InputBytes,
                observed: len,
            });
        }
    }

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

    let is_1904 = wb.has_1904_epoch();

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
        is_1904,
    })
}

fn open_workbook_from_cursor(
    cursor: Cursor<Vec<u8>>,
) -> Result<Xlsx<Cursor<Vec<u8>>>, calamine::XlsxError> {
    Xlsx::new(cursor)
}
