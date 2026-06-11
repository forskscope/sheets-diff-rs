//! Output formatters — thin adapters over `WorkbookDiff` (RFC-013).

pub mod text;

#[cfg(feature = "serde")]
pub mod json;
