use std::fmt;
use std::io::{Read, Seek};
use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};
#[cfg(feature = "serde_derive")]
use serde::{Deserialize, Serialize};

use super::error::{SheetsDiffError, WorkbookSide};
use super::utils::{cell_pos_to_address, diff_range, filter_same_name_sheets};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Whether a cell diff concerns its displayed value or its formula.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub enum CellDiffKind {
    Value,
    Formula,
}

impl fmt::Display for CellDiffKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellDiffKind::Value => write!(f, "value"),
            CellDiffKind::Formula => write!(f, "formula"),
        }
    }
}

/// Top-level diff result between two `.xlsx` workbooks.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct Diff {
    pub old_filepath: String,
    pub new_filepath: String,
    pub sheet_diff: Vec<SheetDiff>,
    pub cell_diffs: Vec<SheetCellDiff>,
}

/// Records a sheet that was added or removed between the two workbooks.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct SheetDiff {
    pub old: Option<String>,
    pub new: Option<String>,
}

/// All cell-level diffs for a single worksheet.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct SheetCellDiff {
    pub sheet: String,
    pub cells: Vec<CellDiff>,
}

/// A single changed cell.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct CellDiff {
    /// 1-based row index.
    pub row: usize,
    /// 1-based column index.
    pub col: usize,
    /// Excel A1 address (e.g. `"XFD1048576"`).
    pub addr: String,
    pub kind: CellDiffKind,
    pub old: Option<String>,
    pub new: Option<String>,
}

// ---------------------------------------------------------------------------
// Diff constructors
// ---------------------------------------------------------------------------

impl Diff {
    /// Panicking convenience constructor.
    ///
    /// Opens both workbooks and computes their diff. Panics with a diagnostic
    /// message if either workbook cannot be opened or read.
    ///
    /// Existing callers of v1.1.4's `Diff::new` can continue to use this
    /// without any source changes.
    ///
    /// For production embedders and GUI applications, prefer [`Diff::try_new`]
    /// to receive a structured [`SheetsDiffError`] instead of a panic.
    pub fn new(old_filepath: &str, new_filepath: &str) -> Self {
        match Self::try_new(old_filepath, new_filepath) {
            Ok(diff) => diff,
            Err(err) => panic!("failed to diff workbooks: {err}"),
        }
    }

    /// Fallible path-based constructor.
    ///
    /// Accepts any value that can be treated as a [`Path`], including `&str`,
    /// `String`, and `PathBuf`. Returns a structured error for missing,
    /// corrupt, locked, or non-`.xlsx` inputs without panicking.
    pub fn try_new(
        old_filepath: impl AsRef<Path>,
        new_filepath: impl AsRef<Path>,
    ) -> Result<Self, SheetsDiffError> {
        let old_path = old_filepath.as_ref();
        let new_path = new_filepath.as_ref();

        let mut old_workbook: Xlsx<_> =
            open_workbook(old_path).map_err(|source| SheetsDiffError::OpenWorkbook {
                side: WorkbookSide::Old,
                path: old_path.to_path_buf(),
                source,
            })?;

        let mut new_workbook: Xlsx<_> =
            open_workbook(new_path).map_err(|source| SheetsDiffError::OpenWorkbook {
                side: WorkbookSide::New,
                path: new_path.to_path_buf(),
                source,
            })?;

        let old_label = old_path.to_string_lossy().into_owned();
        let new_label = new_path.to_string_lossy().into_owned();

        Self::try_from_workbooks(old_label, new_label, &mut old_workbook, &mut new_workbook)
    }

    /// Fallible reader-based constructor with explicit display names.
    ///
    /// Accepts any `Read + Seek` stream (e.g. [`std::io::Cursor`]). The
    /// `old_name` / `new_name` strings are stored in the returned
    /// `Diff.old_filepath` / `Diff.new_filepath` fields and appear in diff
    /// output — supply meaningful labels (filenames, Git object hashes, etc.).
    ///
    /// This constructor lets GUI and VCS tools avoid double I/O and lossy
    /// path-to-string conversions.
    pub fn try_from_named_readers<R1, R2>(
        old_name: impl Into<String>,
        old_reader: R1,
        new_name: impl Into<String>,
        new_reader: R2,
    ) -> Result<Self, SheetsDiffError>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        let mut old_workbook = Xlsx::new(old_reader).map_err(|source| {
            SheetsDiffError::OpenReader {
                side: WorkbookSide::Old,
                source,
            }
        })?;

        let mut new_workbook = Xlsx::new(new_reader).map_err(|source| {
            SheetsDiffError::OpenReader {
                side: WorkbookSide::New,
                source,
            }
        })?;

        Self::try_from_workbooks(
            old_name.into(),
            new_name.into(),
            &mut old_workbook,
            &mut new_workbook,
        )
    }

    /// Returns a clone of this diff (kept for v1.1.4 source compatibility).
    pub fn diff(&mut self) -> Diff {
        self.clone()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

impl Diff {
    /// Returns an empty `Diff` with the given display labels.
    fn empty(old_filepath: String, new_filepath: String) -> Self {
        Diff {
            old_filepath,
            new_filepath,
            sheet_diff: vec![],
            cell_diffs: vec![],
        }
    }

    /// Shared internal engine used by all public constructors.
    fn try_from_workbooks<R1, R2>(
        old_label: String,
        new_label: String,
        old_workbook: &mut Xlsx<R1>,
        new_workbook: &mut Xlsx<R2>,
    ) -> Result<Self, SheetsDiffError>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        let mut diff = Self::empty(old_label, new_label);
        diff.collect_diff_from_workbooks(old_workbook, new_workbook)?;
        diff.normalize_cell_diffs();
        Ok(diff)
    }

    /// Collects all sheet-level and cell-level diffs into `self`.
    fn collect_diff_from_workbooks<R1, R2>(
        &mut self,
        old_workbook: &mut Xlsx<R1>,
        new_workbook: &mut Xlsx<R2>,
    ) -> Result<(), SheetsDiffError>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        let old_sheets = old_workbook.sheet_names().to_owned();
        let new_sheets = new_workbook.sheet_names().to_owned();

        self.collect_sheet_diff(&old_sheets, &new_sheets);

        let same_name_sheets = filter_same_name_sheets(&old_sheets, &new_sheets);
        self.collect_cell_value_diff(old_workbook, new_workbook, &same_name_sheets)?;
        self.collect_cell_formula_diff(old_workbook, new_workbook, &same_name_sheets)?;

        Ok(())
    }

    /// Detects sheets that were added or removed.
    fn collect_sheet_diff(&mut self, old_sheets: &[String], new_sheets: &[String]) {
        if old_sheets == new_sheets {
            return;
        }

        for sheet in old_sheets {
            if !new_sheets.contains(sheet) {
                self.sheet_diff.push(SheetDiff {
                    old: Some(sheet.clone()),
                    new: None,
                });
            }
        }
        for sheet in new_sheets {
            if !old_sheets.contains(sheet) {
                self.sheet_diff.push(SheetDiff {
                    old: None,
                    new: Some(sheet.clone()),
                });
            }
        }
    }

    /// Collects changed cell values for all shared sheets.
    fn collect_cell_value_diff<R1, R2>(
        &mut self,
        old_workbook: &mut Xlsx<R1>,
        new_workbook: &mut Xlsx<R2>,
        same_name_sheets: &[String],
    ) -> Result<(), SheetsDiffError>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        for sheet in same_name_sheets {
            let old_range =
                old_workbook
                    .worksheet_range(sheet)
                    .map_err(|source| SheetsDiffError::ReadSheetValues {
                        side: WorkbookSide::Old,
                        sheet: sheet.clone(),
                        source,
                    })?;

            let new_range =
                new_workbook
                    .worksheet_range(sheet)
                    .map_err(|source| SheetsDiffError::ReadSheetValues {
                        side: WorkbookSide::New,
                        sheet: sheet.clone(),
                        source,
                    })?;

            let mut cell_diffs: Vec<CellDiff> = vec![];

            let (start_row, start_col, end_row, end_col) = diff_range(
                old_range.start(),
                new_range.start(),
                old_range.end(),
                new_range.end(),
            );

            for row in start_row..end_row {
                for col in start_col..end_col {
                    let old_cell = old_range.get_value((row, col)).unwrap_or(&Data::Empty);
                    let new_cell = new_range.get_value((row, col)).unwrap_or(&Data::Empty);

                    if old_cell != new_cell {
                        let row1 = (row + 1) as usize;
                        let col1 = (col + 1) as usize;
                        cell_diffs.push(CellDiff {
                            row: row1,
                            col: col1,
                            addr: cell_pos_to_address(row1, col1),
                            kind: CellDiffKind::Value,
                            old: if old_cell != &Data::Empty {
                                Some(old_cell.to_string())
                            } else {
                                None
                            },
                            new: if new_cell != &Data::Empty {
                                Some(new_cell.to_string())
                            } else {
                                None
                            },
                        });
                    }
                }
            }

            if !cell_diffs.is_empty() {
                self.cell_diffs.push(SheetCellDiff {
                    sheet: sheet.clone(),
                    cells: cell_diffs,
                });
            }
        }

        Ok(())
    }

    /// Collects changed cell formulas for all shared sheets.
    fn collect_cell_formula_diff<R1, R2>(
        &mut self,
        old_workbook: &mut Xlsx<R1>,
        new_workbook: &mut Xlsx<R2>,
        same_name_sheets: &[String],
    ) -> Result<(), SheetsDiffError>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        for sheet in same_name_sheets {
            let old_range = old_workbook
                .worksheet_formula(sheet)
                .map_err(|source| SheetsDiffError::ReadSheetFormulas {
                    side: WorkbookSide::Old,
                    sheet: sheet.clone(),
                    source,
                })?;

            let new_range = new_workbook
                .worksheet_formula(sheet)
                .map_err(|source| SheetsDiffError::ReadSheetFormulas {
                    side: WorkbookSide::New,
                    sheet: sheet.clone(),
                    source,
                })?;

            let mut cell_diffs: Vec<CellDiff> = vec![];

            let (start_row, start_col, end_row, end_col) = diff_range(
                old_range.start(),
                new_range.start(),
                old_range.end(),
                new_range.end(),
            );

            for row in start_row..end_row {
                for col in start_col..end_col {
                    let empty = String::new();
                    let old_cell = old_range.get_value((row, col)).unwrap_or(&empty);
                    let new_cell = new_range.get_value((row, col)).unwrap_or(&empty);

                    if old_cell != new_cell {
                        let row1 = (row + 1) as usize;
                        let col1 = (col + 1) as usize;
                        cell_diffs.push(CellDiff {
                            row: row1,
                            col: col1,
                            addr: cell_pos_to_address(row1, col1),
                            kind: CellDiffKind::Formula,
                            old: if old_cell.is_empty() {
                                None
                            } else {
                                Some(old_cell.to_string())
                            },
                            new: if new_cell.is_empty() {
                                None
                            } else {
                                Some(new_cell.to_string())
                            },
                        });
                    }
                }
            }

            if !cell_diffs.is_empty() {
                self.cell_diffs.push(SheetCellDiff {
                    sheet: sheet.clone(),
                    cells: cell_diffs,
                });
            }
        }

        Ok(())
    }

    /// Merges per-sheet cell diff batches and sorts by `(sheet, row, col, kind)`.
    ///
    /// Sorting by numeric `(row, col)` avoids lexical ordering bugs such as
    /// `A10` appearing before `A2`.
    fn normalize_cell_diffs(&mut self) {
        self.cell_diffs.sort_by(|a, b| a.sheet.cmp(&b.sheet));

        let mut merged: Vec<SheetCellDiff> = vec![];
        for entry in self.cell_diffs.drain(..) {
            match merged.iter_mut().find(|m| m.sheet == entry.sheet) {
                Some(existing) => existing.cells.extend(entry.cells),
                None => merged.push(entry),
            }
        }

        for sheet_diff in &mut merged {
            sheet_diff.cells.sort_by(|a, b| {
                a.row
                    .cmp(&b.row)
                    .then_with(|| a.col.cmp(&b.col))
                    .then_with(|| a.kind.cmp(&b.kind))
            });
        }

        self.cell_diffs = merged;
    }
}
