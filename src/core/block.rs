use std::sync::Arc;

use crate::core::ids::BlockId;
use crate::html::fragment::HtmlFragment;

/// Lifecycle of a block in static or streaming documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStatus {
    Committed,
    Pending,
}

/// High-level block classification for rendering and export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading,
    ThematicBreak,
    CodeFence,
    List,
    BlockQuote,
    Table,
    HtmlBlock,
    MathBlock,
    FootnoteDefinition,
    Unknown,
}

/// Backend-agnostic block payload.
#[derive(Debug, Clone)]
pub enum BlockContent {
    Markdown(CompiledMarkdown),
    PendingMarkdown,
    Code {
        lang: Option<String>,
        complete: bool,
    },
    Html(HtmlFragment),
    Unsupported {
        reason: crate::core::error::UnsupportedReason,
    },
    #[cfg(feature = "_legacy_comrak")]
    LegacyHtml(String),
}

/// One renderable document block.
#[derive(Debug, Clone)]
pub struct RenderBlock {
    pub id: BlockId,
    pub status: BlockStatus,
    pub kind: BlockKind,
    pub source: Arc<str>,
    pub content: BlockContent,
}

/// Opaque compiled Markdown events for internal backends.
#[derive(Debug, Clone)]
pub struct CompiledMarkdown {
    source: Arc<str>,
    events: Arc<[pulldown_cmark::Event<'static>]>,
}

impl CompiledMarkdown {
    pub(crate) fn new(source: Arc<str>, events: Vec<pulldown_cmark::Event<'static>>) -> Self {
        Self {
            source,
            events: events.into(),
        }
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn events(&self) -> &[pulldown_cmark::Event<'static>] {
        &self.events
    }
}
