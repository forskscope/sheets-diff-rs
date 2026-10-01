//! Output formatters and view adapters (RFC-013, RFC-029).

pub mod text;
pub mod view;

#[cfg(feature = "serde")]
pub mod json;
