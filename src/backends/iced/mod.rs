//! Iced GUI backend (implementation detail: enable via default features, not `no_iced`).

#[cfg(feature = "_legacy_comrak")]
mod legacy_document;
pub mod renderer;
mod state;
mod structs;
mod style;
mod widgets;

#[cfg(feature = "_legacy_comrak")]
pub use legacy_document::{CodeBlock, MarkDocument, MarkSegment};
pub use state::MarkState;
pub use structs::{ImageInfo, MarkWidget, RubyMode, UpdateMsg};
pub use style::Style;

#[cfg(feature = "_legacy_comrak")]
pub use crate::parse::comrak_migration::markdown_to_html;
