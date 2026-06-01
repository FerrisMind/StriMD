pub mod block;
pub mod document;
pub mod error;
pub mod ids;

#[cfg(feature = "stream")]
pub mod stream_document;

pub use block::{BlockContent, BlockKind, BlockStatus, CompiledMarkdown, RenderBlock};
pub use document::Document;
#[cfg(feature = "static")]
pub use document::markdown_to_html;
pub use error::{HtmlFragmentError, ParseError, RenderError, UnsupportedReason};
pub use ids::BlockId;

pub use crate::parse::{ParseBackend, ParseDiagnostics};

#[cfg(feature = "stream")]
pub use stream_document::{
    PendingPolicy, StreamDocument, StreamOptions, StreamPatch, StreamUpdate,
};
