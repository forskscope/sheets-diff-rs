//! CLI binary for `sheets-diff` (RFC-013).
//!
//! This is a thin wrapper over the library API.  All comparison logic lives in
//! the library.  This file must not contain any comparison code.

#[cfg(not(feature = "cli"))]
compile_error!("The `cli` feature must be enabled to build the sheets-diff binary.");

use std::process;

use clap::{Parser, ValueEnum};

use sheets_diff::{
    DiffOptions, SheetsDiffError,
    output::text::{render_summary, render_unified},
};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "sheets-diff",
    about = "Structured diff engine for Excel .xlsx workbooks",
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
            // Exit code 2 = operational error
            process::exit(2);
        }
    }
}
