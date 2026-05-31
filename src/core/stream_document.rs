use std::sync::Arc;

use mdstream::adapters::pulldown::{PulldownAdapter, PulldownAdapterOptions};
use mdstream::{FootnotesMode, MdStream, Options as MdstreamOptions, ReferenceDefinitionsMode};

use crate::core::block::{BlockContent, BlockKind, BlockStatus, CompiledMarkdown, RenderBlock};
use crate::core::ids::BlockId;
use crate::options::ParseOptions;
use crate::parse::content::{block_content_from_events, events_contain_html, html_block_content};
use crate::profile::ParseProfile;

/// How pending blocks choose display vs raw source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingPolicy {
    PreferDisplay,
    RawOnly,
}

/// mdstream configuration for StriMD LLM chat streaming.
#[derive(Debug, Clone)]
pub struct StreamOptions {
    pub profile: ParseProfile,
    pub mdstream: MdstreamOptions,
    pub pending_policy: PendingPolicy,
}

impl StreamOptions {
    /// Chat-oriented streaming defaults: invalidate footnotes and late references.
    #[must_use]
    pub fn chat() -> Self {
        Self {
            profile: ParseProfile::ChatStream,
            mdstream: MdstreamOptions {
                footnotes: FootnotesMode::Invalidate,
                reference_definitions: ReferenceDefinitionsMode::Invalidate,
                ..MdstreamOptions::default()
            },
            pending_policy: PendingPolicy::PreferDisplay,
        }
    }
}

/// Incremental patch applied after a stream append or finalize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamPatch {
    AppendCommitted { blocks: Vec<BlockId> },
    ReplaceCommitted { id: BlockId },
    ReplacePending,
    ClearAndRebuild,
    Noop,
}

/// Result of `StreamDocument::append` or `finalize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamUpdate {
    pub patch: StreamPatch,
    pub invalidated: Vec<BlockId>,
    pub reset: bool,
}

/// Append-only streaming Markdown document backed by vendored mdstream.
#[derive(Debug)]
pub struct StreamDocument {
    stream: MdStream,
    adapter: PulldownAdapter,
    blocks: Vec<RenderBlock>,
    pending: Option<RenderBlock>,
    profile: ParseProfile,
    raw_html: crate::options::RawHtmlPolicy,
    next_id: u64,
}

impl StreamDocument {
    #[must_use]
    pub fn new(options: StreamOptions) -> Self {
        let parse_options = ParseOptions::for_profile(options.profile);
        let prefer_display = matches!(options.pending_policy, PendingPolicy::PreferDisplay);
        Self {
            stream: MdStream::new(options.mdstream),
            adapter: PulldownAdapter::new(PulldownAdapterOptions {
                pulldown: parse_options.pulldown,
                prefer_display_for_pending: prefer_display,
            }),
            blocks: Vec::new(),
            pending: None,
            profile: options.profile,
            raw_html: parse_options.raw_html,
            next_id: 1,
        }
    }

    /// Append a streamed chunk and return the resulting patch.
    pub fn append(&mut self, chunk: &str) -> StreamUpdate {
        let update = self.stream.append(chunk);
        self.apply_mdstream_update(update)
    }

    /// Finalize the stream (flush pending) and return the resulting patch.
    pub fn append_str(&mut self, chunk: &str) -> StreamUpdate {
        self.append(chunk)
    }

    /// Reset internal stream state.
    pub fn reset(&mut self) {
        self.stream.reset();
        self.adapter.clear();
        self.blocks.clear();
        self.pending = None;
        self.next_id = 1;
    }

    /// Committed blocks in order.
    pub fn blocks(&self) -> impl Iterator<Item = &RenderBlock> {
        self.blocks.iter()
    }

    /// Current pending block, if any.
    #[must_use]
    pub fn pending(&self) -> Option<&RenderBlock> {
        self.pending.as_ref()
    }

    /// Parse profile used by this stream.
    #[must_use]
    pub fn profile(&self) -> ParseProfile {
        self.profile
    }

    fn apply_mdstream_update(&mut self, update: mdstream::Update) -> StreamUpdate {
        if update.reset {
            self.adapter.clear();
            self.blocks.clear();
            self.pending = None;
            return StreamUpdate {
                patch: StreamPatch::ClearAndRebuild,
                invalidated: Vec::new(),
                reset: true,
            };
        }

        self.adapter.apply_update(&update);

        let mut appended = Vec::new();
        for block in &update.committed {
            let render = self.mdstream_block_to_render(block, BlockStatus::Committed);
            appended.push(render.id);
            self.blocks.push(render);
        }

        let invalidated: Vec<BlockId> = update
            .invalidated
            .iter()
            .map(|id| BlockId::new(id.0))
            .collect();

        for id in &update.invalidated {
            let md_id = *id;
            if let Some(events) = self.adapter.committed_events(md_id) {
                let source = Arc::<str>::from(
                    self.blocks
                        .iter()
                        .find(|b| b.id.0 == md_id.0)
                        .map(|b| b.source.as_ref())
                        .unwrap_or(""),
                );
                let kind = self
                    .blocks
                    .iter()
                    .find(|b| b.id.0 == md_id.0)
                    .map(|b| b.kind)
                    .unwrap_or(BlockKind::Unknown);
                let content = self.committed_content(source, events.to_vec(), kind);
                if let Some(existing) = self.blocks.iter_mut().find(|b| b.id.0 == md_id.0) {
                    existing.content = content;
                }
            }
        }

        let pending_changed = update.pending.is_some();
        self.pending = update.pending.as_ref().map(|pending| {
            self.mdstream_block_to_render(
                &mdstream::Block {
                    id: pending.id,
                    status: mdstream::BlockStatus::Pending,
                    kind: pending.kind,
                    raw: pending.raw.to_string(),
                    display: pending.display.clone(),
                },
                BlockStatus::Pending,
            )
        });

        let patch = if !appended.is_empty() {
            StreamPatch::AppendCommitted { blocks: appended }
        } else if pending_changed {
            StreamPatch::ReplacePending
        } else if !invalidated.is_empty() {
            StreamPatch::ReplaceCommitted {
                id: invalidated[0],
            }
        } else {
            StreamPatch::Noop
        };

        StreamUpdate {
            patch,
            invalidated,
            reset: false,
        }
    }

    fn mdstream_block_to_render(
        &mut self,
        block: &mdstream::Block,
        status: BlockStatus,
    ) -> RenderBlock {
        let id = BlockId::new(block.id.0);
        self.next_id = self.next_id.max(block.id.0 + 1);
        let source_text = block.display_or_raw();
        let source = Arc::<str>::from(source_text);
        let kind = mdstream_kind_to_strimd(block.kind);
        let content = if block.kind == mdstream::BlockKind::HtmlBlock {
            html_block_content(source.clone(), self.raw_html)
        } else if let Some(events) = self.adapter.committed_events(block.id) {
            self.committed_content(source.clone(), events.to_vec(), kind)
        } else if block.kind == mdstream::BlockKind::CodeFence {
            BlockContent::Code {
                lang: block.code_fence_language().map(str::to_string),
                complete: status == BlockStatus::Committed,
            }
        } else {
            BlockContent::PendingMarkdown
        };

        RenderBlock {
            id,
            status,
            kind,
            source,
            content,
        }
    }

    fn committed_content(
        &self,
        source: Arc<str>,
        events: Vec<pulldown_cmark::Event<'static>>,
        kind: BlockKind,
    ) -> BlockContent {
        if kind == BlockKind::HtmlBlock || events_contain_html(&events) {
            block_content_from_events(&events, source, self.raw_html)
        } else {
            BlockContent::Markdown(CompiledMarkdown::new(source, events))
        }
    }
}

fn mdstream_kind_to_strimd(kind: mdstream::BlockKind) -> BlockKind {
    match kind {
        mdstream::BlockKind::Paragraph => BlockKind::Paragraph,
        mdstream::BlockKind::Heading => BlockKind::Heading,
        mdstream::BlockKind::ThematicBreak => BlockKind::ThematicBreak,
        mdstream::BlockKind::CodeFence => BlockKind::CodeFence,
        mdstream::BlockKind::List => BlockKind::List,
        mdstream::BlockKind::BlockQuote => BlockKind::BlockQuote,
        mdstream::BlockKind::Table => BlockKind::Table,
        mdstream::BlockKind::HtmlBlock => BlockKind::HtmlBlock,
        mdstream::BlockKind::MathBlock => BlockKind::MathBlock,
        mdstream::BlockKind::FootnoteDefinition => BlockKind::FootnoteDefinition,
        mdstream::BlockKind::Unknown => BlockKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_options_invalidate_footnotes_and_references() {
        let opts = StreamOptions::chat();
        assert_eq!(opts.mdstream.footnotes, FootnotesMode::Invalidate);
        assert_eq!(
            opts.mdstream.reference_definitions,
            ReferenceDefinitionsMode::Invalidate
        );
    }

    #[test]
    fn append_commits_blocks_incrementally() {
        let mut doc = StreamDocument::new(StreamOptions::chat());
        let update = doc.append("Hello ");
        assert!(!update.reset);
        let update = doc.append("world.");
        assert!(!update.reset);
        assert!(doc.blocks().count() >= 1 || doc.pending().is_some());
        let _ = update;
    }

    #[test]
    fn streamed_html_block_routes_to_fragment() {
        let mut doc = StreamDocument::new(StreamOptions::chat());
        doc.append("<details><summary>open</summary></details>\n\n");
        let block = doc
            .blocks()
            .find(|b| b.kind == BlockKind::HtmlBlock)
            .expect("html block");
        assert!(matches!(block.content, BlockContent::Html(_)));
    }

    #[test]
    fn streamed_inline_html_is_chunk_invariant() {
        let whole = {
            let mut doc = StreamDocument::new(StreamOptions::chat());
            doc.append("text <span>x</span> and more.\n\n");
            doc.blocks()
                .last()
                .or_else(|| doc.pending())
                .expect("block")
                .content
                .clone()
        };
        let chunked = {
            let mut doc = StreamDocument::new(StreamOptions::chat());
            doc.append("text <span>x</span>");
            doc.append(" and more.\n\n");
            doc.blocks()
                .last()
                .or_else(|| doc.pending())
                .expect("block")
                .content
                .clone()
        };
        assert_eq!(
            std::mem::discriminant(&whole),
            std::mem::discriminant(&chunked)
        );
    }
}
