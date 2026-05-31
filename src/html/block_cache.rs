//! Compile-once cache mapping [`RenderBlock`] values to [`HtmlFragment`] trees for rendering.

use std::collections::HashMap;
use std::sync::Arc;

use pulldown_cmark::{html, Event, Options, Parser};

use crate::core::block::{
    BlockContent, BlockStatus, CompiledMarkdown, RenderBlock,
};
use crate::core::ids::BlockId;
use crate::core::document::Document;
use crate::html::fragment::HtmlFragment;
#[cfg(feature = "stream")]
use crate::core::{StreamDocument, StreamPatch, StreamUpdate};

/// Fenced code payload for backends that render code blocks outside the DOM.
#[derive(Debug, Clone)]
pub(crate) struct CachedCodeBlock {
    #[allow(dead_code)]
    pub(crate) language: Option<String>,
    pub(crate) code: String,
    /// Pre-parsed iced markdown items (highlighted lines for [`iced::widget::markdown::code_block`]).
    pub(crate) markdown_items: Arc<Vec<iced::widget::markdown::Item>>,
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
    ids: Vec<BlockId>,
    statuses: Vec<BlockStatus>,
    indices: HashMap<BlockId, usize>,
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
        self.ids.clear();
        self.statuses.clear();
        self.indices.clear();
        for block in blocks {
            self.push_block(block);
        }
    }

    #[cfg(feature = "stream")]
    pub fn sync_from_stream(&mut self, stream: &StreamDocument) {
        let mut blocks: Vec<RenderBlock> = stream.blocks().cloned().collect();
        if let Some(pending) = stream.pending() {
            blocks.push(pending.clone());
        }
        self.rebuild(&blocks);
    }

    #[cfg(feature = "stream")]
    pub fn apply_stream_update(&mut self, stream: &StreamDocument, update: &StreamUpdate) {
        if update.reset || matches!(update.patch, StreamPatch::ClearAndRebuild) {
            self.sync_from_stream(stream);
            return;
        }

        match &update.patch {
            StreamPatch::AppendCommitted { blocks } => {
                self.remove_pending();
                for id in blocks {
                    if let Some(block) = stream.committed_block(*id) {
                        self.push_block(block);
                    }
                }
                self.sync_pending(stream);
            }
            StreamPatch::ReplaceCommitted { .. } => {}
            StreamPatch::ReplacePending => {
                self.sync_pending(stream);
            }
            StreamPatch::Noop => {}
            StreamPatch::ClearAndRebuild => self.sync_from_stream(stream),
        }

        self.apply_invalidated(stream, &update.invalidated);
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
                let code = block.source.trim_end_matches('\n').to_string();
                let markdown_items = Arc::new(crate::backends::iced::iced_markdown_items_for_codeblock(
                    lang.as_deref(),
                    &code,
                ));
                CachedBlock::Code(CachedCodeBlock {
                    language: lang.clone(),
                    code,
                    markdown_items,
                })
            }
            BlockContent::PendingMarkdown => {
                CachedBlock::Fragment(pending_markdown_to_fragment(&block.source))
            }
            BlockContent::Unsupported { .. } => CachedBlock::Empty,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn compile_count(&self) -> usize {
        self.compile_count
    }

    fn push_block(&mut self, block: &RenderBlock) {
        let index = self.entries.len();
        self.ids.push(block.id);
        self.statuses.push(block.status);
        let compiled = self.compile_block(block);
        self.entries.push(compiled);
        self.indices.insert(block.id, index);
    }

    #[cfg(feature = "stream")]
    fn remove_pending(&mut self) {
        if self.statuses.last().copied() != Some(BlockStatus::Pending) {
            return;
        }
        if let Some(id) = self.ids.pop() {
            self.indices.remove(&id);
        }
        self.entries.pop();
        self.statuses.pop();
    }

    #[cfg(feature = "stream")]
    fn sync_pending(&mut self, stream: &StreamDocument) {
        self.remove_pending();
        if let Some(pending) = stream.pending() {
            self.push_block(pending);
        }
    }

    #[cfg(feature = "stream")]
    fn replace_block(&mut self, index: usize, block: &RenderBlock) {
        let old_id = self.ids[index];
        if old_id != block.id {
            self.indices.remove(&old_id);
        }
        self.ids[index] = block.id;
        self.statuses[index] = block.status;
        let compiled = self.compile_block(block);
        self.entries[index] = compiled;
        self.indices.insert(block.id, index);
    }

    #[cfg(feature = "stream")]
    fn apply_invalidated(&mut self, stream: &StreamDocument, invalidated: &[BlockId]) {
        for id in invalidated {
            let Some(index) = self.indices.get(id).copied() else {
                continue;
            };
            let Some(block) = stream.committed_block(*id) else {
                continue;
            };
            self.replace_block(index, block);
        }
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
    use crate::html::fragment::HtmlFragment;
    #[cfg(feature = "stream")]
    use crate::html::fragment::{HtmlNode, NodeId};
    use crate::options::ParseOptions;
    use crate::parse::pulldown;
    use crate::profile::ParseProfile;

    #[cfg(feature = "stream")]
    fn fragment_contains_tag(fragment: &HtmlFragment, id: NodeId, expected: &str) -> bool {
        match fragment.node(id) {
            Some(HtmlNode::Element { tag, children, .. }) => {
                tag.as_str() == expected
                    || children
                        .iter()
                        .any(|child| fragment_contains_tag(fragment, *child, expected))
            }
            _ => false,
        }
    }

    #[cfg(feature = "stream")]
    fn cache_entry_contains_tag(cache: &BlockRenderCache, index: usize, expected: &str) -> bool {
        match cache.entry(index) {
            Some(CachedBlock::Fragment(fragment)) => fragment
                .roots()
                .iter()
                .any(|root| fragment_contains_tag(fragment, *root, expected)),
            _ => false,
        }
    }

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

    #[cfg(feature = "stream")]
    #[test]
    fn incremental_stream_update_appends_without_rebuilding_prior_blocks() {
        use crate::core::{StreamDocument, StreamOptions};

        let mut stream = StreamDocument::new(StreamOptions::chat());
        let mut cache = BlockRenderCache::default();

        let update = stream.append("First paragraph.\n\nSecond paragraph.\n\nThird paragraph.\n\n");
        cache.apply_stream_update(&stream, &update);
        let compiled_after_first = cache.compile_count();

        let update = stream.append("Fourth paragraph.\n\n");
        let before = cache.compile_count();
        cache.apply_stream_update(&stream, &update);

        let incremental_delta = cache.compile_count() - before;
        let mut rebuilt = BlockRenderCache::default();
        rebuilt.sync_from_stream(&stream);

        assert_eq!(compiled_after_first, before);
        assert!(incremental_delta < rebuilt.compile_count());
        assert_eq!(cache.len(), rebuilt.len());
    }

    #[cfg(feature = "stream")]
    #[test]
    fn incremental_stream_update_recompiles_only_invalidated_blocks() {
        use crate::core::{StreamDocument, StreamOptions};

        let mut stream = StreamDocument::new(StreamOptions::chat());
        let mut cache = BlockRenderCache::default();

        let update = stream.append("Intro.\n\nAnother intro.\n\nSee [ref].\n\n");
        cache.apply_stream_update(&stream, &update);
        let update = stream.append("[ref]: https://example.com\n");
        cache.apply_stream_update(&stream, &update);
        stream.append("\n");
        let update = stream.append("Next\n");
        let before = cache.compile_count();
        cache.apply_stream_update(&stream, &update);
        let incremental_delta = cache.compile_count() - before;
        let mut rebuilt = BlockRenderCache::default();
        rebuilt.sync_from_stream(&stream);

        assert!(!update.invalidated.is_empty());
        assert!(incremental_delta < rebuilt.compile_count());
        assert_eq!(cache.len(), rebuilt.len());
        assert!(cache_entry_contains_tag(&cache, 2, "a"));
        assert!(cache_entry_contains_tag(&rebuilt, 2, "a"));
    }

    fn fragment_root_tags(fragment: &HtmlFragment) -> Vec<String> {
        fragment
            .roots()
            .iter()
            .filter_map(|&id| match fragment.node(id) {
                Some(crate::html::fragment::HtmlNode::Element { tag, .. }) => {
                    Some(tag.as_str().to_string())
                }
                Some(crate::html::fragment::HtmlNode::Text(t)) => {
                    Some(format!("text:{}", &t[..t.len().min(40)]))
                }
                _ => None,
            })
            .collect()
    }

    fn fragment_has_tag(fragment: &HtmlFragment, expected: &str) -> bool {
        fn walk(fragment: &HtmlFragment, id: crate::html::fragment::NodeId, expected: &str) -> bool {
            match fragment.node(id) {
                Some(crate::html::fragment::HtmlNode::Element { tag, children, .. }) => {
                    tag.as_str() == expected
                        || children
                            .iter()
                            .any(|&c| walk(fragment, c, expected))
                }
                _ => false,
            }
        }
        fragment
            .roots()
            .iter()
            .any(|&r| walk(fragment, r, expected))
    }

    #[test]
    fn hello_fixture_blocks_contain_heading_hr_and_blockquote() {
        const HELLO: &str = r"
# Hello, World!
This is a markdown renderer <b>with inline HTML support!</b>
- You can mix and match markdown and HTML together
<hr>

```rust
App { state: 1 }
```

## Note

> <b>Fun fact</b>: This is all built on top of existing iced widgets.
>
> No new widgets were made for this.
";
        let doc = crate::core::document::Document::parse(HELLO, ParseProfile::GitHubPreview)
            .expect("parse");
        let cache = BlockRenderCache::from_document(&doc);
        let mut kinds = Vec::new();
        let mut has_h1 = false;
        let mut has_h2 = false;
        let mut has_hr = false;
        let mut has_blockquote = false;
        for (i, block) in doc.blocks().iter().enumerate() {
            kinds.push(format!("{:?}", block.kind));
            if let Some(CachedBlock::Fragment(fragment)) = cache.entry(i) {
                let _roots = fragment_root_tags(fragment);
                has_h1 |= fragment_has_tag(fragment, "h1");
                has_h2 |= fragment_has_tag(fragment, "h2");
                has_hr |= fragment_has_tag(fragment, "hr");
                has_blockquote |= fragment_has_tag(fragment, "blockquote");
            }
        }
        assert!(
            doc.blocks().iter().any(|b| b.kind == BlockKind::Heading),
            "expected heading blocks, kinds={kinds:?}"
        );
        assert!(has_h1, "fragment missing h1, kinds={kinds:?}");
        assert!(has_h2, "fragment missing h2, kinds={kinds:?}");
        assert!(has_hr, "fragment missing hr, kinds={kinds:?}");
        assert!(has_blockquote, "fragment missing blockquote, kinds={kinds:?}");
        for (i, block) in doc.blocks().iter().enumerate() {
            if let Some(CachedBlock::Fragment(fragment)) = cache.entry(i) {
                if block.kind == BlockKind::HtmlBlock {
                    assert!(
                        fragment.roots().iter().any(|&r| {
                            matches!(
                                fragment.node(r),
                                Some(crate::html::fragment::HtmlNode::Element { tag, .. })
                                    if tag.as_str() == "hr"
                            )
                        }),
                        "hr html block should have hr element root"
                    );
                }
            }
        }
    }

    #[test]
    fn hello_example_code_fence_in_cached_code_block() {
        const HELLO: &str = r"
# Hello
```rust
fn demo() {}
```
";
        let doc =
            crate::core::document::Document::parse(HELLO, ParseProfile::GitHubPreview).expect("parse");
        let cache = BlockRenderCache::from_document(&doc);
        let idx = doc
            .blocks()
            .iter()
            .position(|b| b.kind == BlockKind::CodeFence)
            .expect("hello has code fence");
        assert!(matches!(cache.entry(idx), Some(CachedBlock::Code(_))));
    }

    #[test]
    fn hello_code_fence_routes_to_cached_code_block() {
        const SNIPPET: &str = "```rust\nfn main() {}\n```\n";
        let doc =
            crate::core::document::Document::parse(SNIPPET, ParseProfile::GitHubPreview).expect("parse");
        let cache = BlockRenderCache::from_document(&doc);
        let code_block = doc
            .blocks()
            .iter()
            .find(|b| b.kind == BlockKind::CodeFence)
            .expect("code fence block");
        assert!(matches!(
            &code_block.content,
            BlockContent::Code {
                lang: Some(lang),
                ..
            } if lang == "rust"
        ));
        let idx = doc
            .blocks()
            .iter()
            .position(|b| b.kind == BlockKind::CodeFence)
            .unwrap();
        let CachedBlock::Code(c) = cache.entry(idx).expect("cache entry") else {
            panic!("expected CachedBlock::Code");
        };
        assert!(
            c.code.contains("fn main"),
            "code body wrong: {:?}",
            c.code
        );
        assert!(
            !c.code.contains("# Hello"),
            "code must not be full document source"
        );
        assert!(
            c.markdown_items
                .iter()
                .any(|item| matches!(item, iced::widget::markdown::Item::CodeBlock { .. })),
            "expected iced markdown CodeBlock item for syntax highlighting"
        );
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
