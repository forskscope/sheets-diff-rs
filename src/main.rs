//! CLI binary for `sheets-diff` (RFC-013).
//!
//! This is a thin wrapper over the library API.  All comparison logic lives in
//! the library.  This file must not contain any comparison code.

#[cfg(not(feature = "cli"))]
compile_error!("The `cli` feature must be enabled to build the sheets-diff binary.");

use std::process;

use clap::{Parser, ValueEnum};

use sheets_diff::{
    DiffOptions, OpenErrorKind, ReadErrorKind, SheetsDiffError,
    output::text::{render_summary, render_unified},
};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "sheets-diff",
    about = "Structured diff engine for Excel .xlsx workbooks",
    after_help = "EXIT CODES:\n  \
                  0  no differences found\n  \
                  1  differences found\n  \
                  2  operational error (invalid options, a resource limit was \
                  hit, an environment issue such as a missing or unreadable \
                  file, or an internal bug)\n  \
                  3  invalid or corrupt input (the file at the given path is \
                  not a readable .xlsx workbook: wrong format, corrupt \
                  internals, or encrypted)",
    version
)]
struct Cli {
    /// Path to the old workbook.
    old: std::path::PathBuf,

    /// Path to the new workbook.
    new: std::path::PathBuf,

    /// Output format.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Summary)]
    format: OutputFormat,

    /// Do not compare formulas.
    #[arg(long)]
    no_formulas: bool,

    /// Suppress warnings in output.
    #[arg(long)]
    no_warnings: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// Compact summary (default).
    Summary,
    /// Unified diff.
    Unified,
}

// ---------------------------------------------------------------------------
// Exit-code mapping (RFC-013)
// ---------------------------------------------------------------------------

/// Maps a comparison failure to an exit code.
///
/// The line: **3** when something about the bytes at the given path make
/// them unusable as a workbook (wrong format, corrupt internals, encrypted);
/// **2** for everything else — reaching those bytes in the first place
/// (missing file, permissions, a lock held by another process), caller
/// misconfiguration, a resource limit, or an internal bug. `NotFound` /
/// `PermissionDenied` / `Locked` are about the environment around the file,
/// not the file's own content, so they stay environment errors (2) rather
/// than joining the corrupt-input bucket (3).
///
/// `_` arms exist because `SheetsDiffError` and `OpenErrorKind` are
/// `#[non_exhaustive]`: an unclassified future variant defaults to 2 rather
/// than being guessed into 3.
fn exit_code_for(err: &SheetsDiffError) -> i32 {
    match err {
        SheetsDiffError::OpenWorkbook { kind, .. } => match kind {
            OpenErrorKind::NotXlsx | OpenErrorKind::Corrupt => 3,
            OpenErrorKind::NotFound | OpenErrorKind::PermissionDenied | OpenErrorKind::Locked => 2,
            _ => 2,
        },
        // The workbook opened, but a sheet inside it couldn't be read.
        SheetsDiffError::ReadSheet { kind, .. } => match kind {
            // The workbook's own internal structure not holding up --
            // what "corrupt input" means once you're past the open step.
            //
            // Sound only because the CLI has no sheet-selection flag: every
            // sheet the workbook's own index promises gets read, so a
            // missing one is the workbook's inconsistency, not a caller's
            // request for a sheet that was never going to exist. If a
            // `--sheet` flag is ever added, `SheetNotFound` becomes caller
            // error for an unmatched selection and must move to 2.
            ReadErrorKind::SheetNotFound | ReadErrorKind::MalformedSheet => 3,
            // An I/O failure mid-read (disk, network filesystem) is not
            // evidence the workbook is corrupt -- conservative default,
            // same reasoning as `OpenErrorKind::Other` above.
            ReadErrorKind::Other => 2,
            _ => 2,
        },
        SheetsDiffError::UnsupportedFormat { .. } => 3,
        SheetsDiffError::EncryptedWorkbook { .. } => 3,
        SheetsDiffError::InvalidOptions { .. } => 2,
        SheetsDiffError::Cancelled => 2,
        SheetsDiffError::LimitExceeded { .. } => 2,
        SheetsDiffError::Internal { .. } => 2,
        _ => 2,
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let mut builder = DiffOptions::builder();
    if cli.no_formulas {
        builder = builder.formula_compare(sheets_diff::FormulaCompareMode::Ignore);
    }
    let opts = match builder.build() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("sheets-diff: invalid options: {e}");
            process::exit(2);
        }
    };

    match sheets_diff::compare_paths_with_options(&cli.old, &cli.new, opts) {
        Ok(diff) => {
            let output = match cli.format {
                OutputFormat::Summary => render_summary(&diff),
                OutputFormat::Unified => render_unified(&diff),
            };
            print!("{output}");

            // Exit code: 0 = no differences, 1 = differences found
            if diff.summary.cells_changed > 0
                || diff.summary.sheets_added > 0
                || diff.summary.sheets_removed > 0
                || diff.summary.sheets_renamed > 0
            {
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("sheets-diff: {e}");
            if let Some(src) = std::error::Error::source(&e) {
                eprintln!("  caused by: {src}");
            }
            process::exit(exit_code_for(&e));
        }
    }
}
