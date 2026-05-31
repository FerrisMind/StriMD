//! Iced GUI backend (implementation detail: enable via default features, not `no_iced`).

mod code_fence;
mod legacy_document;
mod dom;
pub mod renderer;
mod state;
mod structs;
mod style;
mod widgets;

pub use code_fence::{fence_markdown_for_codeblock, iced_markdown_items_for_codeblock};
pub use legacy_document::{CodeBlock, MarkDocument, MarkSegment};
pub use state::MarkState;
pub use structs::{ImageInfo, MarkWidget, RubyMode, UpdateMsg};
pub use style::Style;

#[cfg(feature = "static")]
pub use crate::core::document::markdown_to_html;
