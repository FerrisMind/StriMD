#![doc = include_str!("../README.md")]

mod renderer;
mod state;
mod structs;
mod style;
mod widgets;

#[cfg(feature = "markdown")]
mod comrak;
#[cfg(feature = "markdown")]
mod document;

pub use state::MarkState;
pub use structs::{ImageInfo, MarkWidget, RubyMode, UpdateMsg};
pub use style::Style;

#[cfg(feature = "markdown")]
pub use comrak::markdown_to_html;
#[cfg(feature = "markdown")]
pub use document::{CodeBlock, MarkDocument, MarkSegment};
