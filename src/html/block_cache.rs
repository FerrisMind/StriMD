//! Compile-once cache mapping [`RenderBlock`] values to [`HtmlFragment`] trees for rendering.

use pulldown_cmark::{html, Event, Options, Parser};

use crate::core::block::{
    BlockContent, BlockStatus, CompiledMarkdown, RenderBlock,
};
use crate::core::document::Document;
use crate::html::fragment::HtmlFragment;

/// Fenced code payload for backends that render code blocks outside the DOM.
#[derive(Debug, Clone)]
pub(crate) struct CachedCodeBlock {
    #[allow(dead_code)]
    pub(crate) language: Option<String>,
    pub(crate) code: String,
}

/// One compiled block ready for DOM traversal.
pub(crate) enum CachedBlock {
    Fragment(HtmlFragment),
    Code(CachedCodeBlock),
    Empty,
}

/// Block-ordered render cache; entries are built once per block revision.
#[derive(Default)]
pub(crate) struct BlockRenderCache {
    entries: Vec<CachedBlock>,
    statuses: Vec<BlockStatus>,
    #[cfg(test)]
    compile_count: usize,
}

impl BlockRenderCache {
    #[must_use]
    pub fn from_blocks(blocks: &[RenderBlock]) -> Self {
        let mut cache = Self::default();
        cache.rebuild(blocks);
        cache
    }

    #[must_use]
    pub fn from_document(document: &Document) -> Self {
        Self::from_blocks(document.blocks())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Block lifecycle (committed vs pending); used by stream sync and tests.
    #[allow(dead_code)]
    #[must_use]
    pub fn status(&self, index: usize) -> Option<BlockStatus> {
        self.statuses.get(index).copied()
    }

    pub fn entry(&self, index: usize) -> Option<&CachedBlock> {
        self.entries.get(index)
    }

    pub(crate) fn entries(&self) -> &[CachedBlock] {
        &self.entries
    }

    pub fn rebuild(&mut self, blocks: &[RenderBlock]) {
        self.entries.clear();
        self.statuses.clear();
        for block in blocks {
            self.statuses.push(block.status);
            let compiled = self.compile_block(block);
            self.entries.push(compiled);
        }
    }

    #[cfg(feature = "stream")]
    pub fn sync_from_stream(&mut self, stream: &crate::core::StreamDocument) {
        let mut blocks: Vec<RenderBlock> = stream.blocks().cloned().collect();
        if let Some(pending) = stream.pending() {
            blocks.push(pending.clone());
        }
        self.rebuild(&blocks);
    }

    fn compile_block(&mut self, block: &RenderBlock) -> CachedBlock {
        #[cfg(test)]
        {
            self.compile_count += 1;
        }
        match &block.content {
            BlockContent::Markdown(compiled) => {
                CachedBlock::Fragment(markdown_to_fragment(compiled))
            }
            BlockContent::Html(fragment) => CachedBlock::Fragment(fragment.clone()),
            BlockContent::Code { lang, .. } => {
                CachedBlock::Code(CachedCodeBlock {
                    language: lang.clone(),
                    code: block.source.trim_end_matches('\n').to_string(),
                })
            }
            BlockContent::PendingMarkdown => {
                CachedBlock::Fragment(pending_markdown_to_fragment(&block.source))
            }
            BlockContent::Unsupported { .. } => CachedBlock::Empty,
            #[cfg(feature = "_legacy_comrak")]
            BlockContent::LegacyHtml(html) => CachedBlock::Fragment(HtmlFragment::from_html(html)),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn compile_count(&self) -> usize {
        self.compile_count
    }
}

fn markdown_to_fragment(compiled: &CompiledMarkdown) -> HtmlFragment {
    let mut html_buf = String::new();
    html::push_html(&mut html_buf, compiled.events().iter().cloned());
    if html_buf.is_empty() {
        HtmlFragment::from_html("<p></p>")
    } else {
        HtmlFragment::from_html(&html_buf)
    }
}

fn pending_markdown_to_fragment(source: &str) -> HtmlFragment {
    let events: Vec<Event<'static>> = Parser::new_ext(source, Options::all())
        .map(|event| event.into_static())
        .collect();
    let mut html_buf = String::new();
    html::push_html(&mut html_buf, events.iter().cloned());
    if html_buf.is_empty() {
        HtmlFragment::from_html(&format!("<p>{}</p>", escape_html_text(source)))
    } else {
        HtmlFragment::from_html(&html_buf)
    }
}

fn escape_html_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::core::block::BlockKind;
    use crate::core::ids::BlockId;
    use crate::options::ParseOptions;
    use crate::parse::pulldown;
    use crate::profile::ParseProfile;

    #[test]
    fn committed_markdown_compiles_once_per_rebuild() {
        let blocks = pulldown::parse_blocks(
            "# Title\n\nBody.",
            ParseProfile::GitHubPreview,
            &ParseOptions::default(),
        )
        .expect("parse");
        let mut cache = BlockRenderCache::from_blocks(&blocks);
        assert_eq!(cache.compile_count(), blocks.len());
        let first = match cache.entry(0) {
            Some(CachedBlock::Fragment(fragment)) => fragment as *const HtmlFragment,
            _ => panic!("expected fragment entry"),
        };
        assert!(std::ptr::eq(
            first,
            match cache.entry(0) {
                Some(CachedBlock::Fragment(fragment)) => fragment as *const HtmlFragment,
                _ => panic!("expected fragment"),
            }
        ));
        cache.rebuild(&blocks);
        assert_eq!(cache.compile_count(), blocks.len() * 2);
    }

    #[test]
    fn readme_snippet_block_cache_has_dom_children() {
        use crate::core::document::Document;
        use crate::profile::ParseProfile;

        let text = "Hello from **markdown** and <b>HTML</b>!";
        let doc = Document::parse(text, ParseProfile::GitHubPreview).expect("parse");
        assert!(!doc.blocks().is_empty());
        let cache = BlockRenderCache::from_blocks(doc.blocks());
        let any_children = (0..cache.len()).any(|i| {
            matches!(cache.entry(i), Some(CachedBlock::Fragment(fragment)) if {
                !fragment.roots().is_empty()
            })
        });
        assert!(any_children, "expected non-empty fragment in block cache");
    }

    #[test]
    fn html_fragment_block_compiles_to_dom() {
        let blocks = pulldown::parse_blocks(
            "<details><summary>x</summary></details>",
            ParseProfile::GitHubPreview,
            &ParseOptions::default(),
        )
        .expect("parse");
        assert!(blocks
            .iter()
            .any(|b| matches!(b.content, BlockContent::Html(_))));
        let cache = BlockRenderCache::from_blocks(&blocks);
        assert!(cache
            .entries()
            .iter()
            .any(|entry| matches!(entry, CachedBlock::Fragment(_))));
    }

    #[cfg(feature = "stream")]
    #[test]
    fn sync_from_stream_includes_pending_block() {
        use crate::core::{StreamDocument, StreamOptions};

        let mut stream = StreamDocument::new(StreamOptions::chat());
        stream.append("Hello ");
        let mut cache = BlockRenderCache::default();
        cache.sync_from_stream(&stream);
        assert!(cache.len() > 0);
        if let Some(pending) = stream.pending() {
            assert_eq!(cache.status(cache.len() - 1), Some(BlockStatus::Pending));
            assert_eq!(cache.status(cache.len() - 1), Some(pending.status));
        }
    }

    #[test]
    fn render_block_markdown_uses_compiled_events() {
        let source = "**bold**";
        let blocks = vec![RenderBlock {
            id: BlockId::new(1),
            status: BlockStatus::Committed,
            kind: BlockKind::Paragraph,
            source: Arc::from(source),
            content: BlockContent::Markdown(CompiledMarkdown::new(
                Arc::from(source),
                Parser::new(source)
                    .map(|e| e.into_static())
                    .collect(),
            )),
        }];
        let cache = BlockRenderCache::from_blocks(&blocks);
        let CachedBlock::Fragment(fragment) = cache.entry(0).expect("entry") else {
            panic!("expected fragment");
        };
        assert!(!fragment.roots().is_empty());
    }
}
