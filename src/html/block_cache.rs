//! Compile-once cache mapping [`RenderBlock`] values to [`HtmlFragment`] trees for rendering.

use std::collections::HashMap;
use std::rc::Rc;

use pulldown_cmark::{Parser, html};

use crate::core::block::{BlockContent, BlockStatus, CompiledMarkdown, RenderBlock};
use crate::core::document::Document;
use crate::core::ids::BlockId;
#[cfg(feature = "stream")]
use crate::core::{StreamDocument, StreamPatch, StreamUpdate};
use crate::html::block_alignment::{
    BlockAlignment, block_closes_alignment_wrapper, block_opens_alignment_wrapper,
    fragment_is_complete_alignment_wrapper,
};
use crate::html::fragment::HtmlFragment;
use crate::profile::ParseProfile;

/// Fenced code payload for backends that render code blocks outside the DOM.
#[derive(Debug, Clone)]
pub(crate) struct CachedCodeBlock {
    #[allow(dead_code)]
    pub(crate) language: Option<String>,
    pub(crate) code: String,
    /// Pre-parsed iced markdown items (highlighted lines for [`iced::widget::markdown::code_block`]).
    pub(crate) markdown_items: Rc<Vec<iced::widget::markdown::Item>>,
}

/// One compiled block ready for DOM traversal.
pub(crate) enum CachedBlock {
    Fragment(HtmlFragment),
    Code(CachedCodeBlock),
    Empty,
}

/// Block-ordered render cache; entries are built once per block revision.
pub(crate) struct BlockRenderCache {
    profile: ParseProfile,
    entries: Vec<CachedBlock>,
    entry_alignment: Vec<Option<BlockAlignment>>,
    ids: Vec<BlockId>,
    statuses: Vec<BlockStatus>,
    indices: HashMap<BlockId, usize>,
    alignment_context: Option<BlockAlignment>,
    #[cfg(test)]
    compile_count: usize,
}

impl Default for BlockRenderCache {
    fn default() -> Self {
        Self {
            profile: ParseProfile::GitHubPreview,
            entries: Vec::new(),
            entry_alignment: Vec::new(),
            ids: Vec::new(),
            statuses: Vec::new(),
            indices: HashMap::new(),
            alignment_context: None,
            #[cfg(test)]
            compile_count: 0,
        }
    }
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
        let mut cache = Self::from_blocks(document.blocks());
        cache.profile = document.profile();
        cache
    }

    #[must_use]
    pub fn entry_alignment(&self, index: usize) -> Option<BlockAlignment> {
        self.entry_alignment.get(index).copied().flatten()
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
        self.entry_alignment.clear();
        self.ids.clear();
        self.statuses.clear();
        self.indices.clear();
        self.alignment_context = None;
        for block in blocks {
            if fragment_is_complete_alignment_wrapper(block) {
                let align = block_opens_alignment_wrapper(block);
                self.push_block(block, align);
                self.alignment_context = None;
                continue;
            }
            if let Some(align) = block_opens_alignment_wrapper(block) {
                self.alignment_context = Some(align);
                continue;
            }
            if block_closes_alignment_wrapper(block) {
                self.alignment_context = None;
                continue;
            }
            self.push_block(block, self.alignment_context);
        }
    }

    #[allow(dead_code)]
    fn ingest_block(&mut self, block: &RenderBlock) {
        if let Some(align) = block_opens_alignment_wrapper(block) {
            self.alignment_context = Some(align);
            return;
        }
        if block_closes_alignment_wrapper(block) {
            self.alignment_context = None;
            return;
        }
        self.push_block(block, self.alignment_context);
    }

    #[cfg(feature = "stream")]
    pub fn sync_from_stream(&mut self, stream: &StreamDocument) {
        self.profile = stream.profile();
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
                        self.ingest_block(block);
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
                CachedBlock::Fragment(markdown_to_fragment(compiled, self.profile))
            }
            BlockContent::Html(fragment) => CachedBlock::Fragment(fragment.clone()),
            BlockContent::Code { lang, .. } => {
                let code = block.source.trim_end_matches('\n').to_string();
                let markdown_items =
                    Rc::new(crate::backends::iced::iced_markdown_items_for_codeblock(
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
                CachedBlock::Fragment(pending_markdown_to_fragment(&block.source, self.profile))
            }
            BlockContent::Unsupported { .. } => CachedBlock::Empty,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn compile_count(&self) -> usize {
        self.compile_count
    }

    fn push_block(&mut self, block: &RenderBlock, alignment: Option<BlockAlignment>) {
        let index = self.entries.len();
        self.ids.push(block.id);
        self.statuses.push(block.status);
        let compiled = self.compile_block(block);
        self.entries.push(compiled);
        self.entry_alignment.push(alignment);
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
        self.entry_alignment.pop();
        self.statuses.pop();
    }

    #[cfg(feature = "stream")]
    fn sync_pending(&mut self, stream: &StreamDocument) {
        self.remove_pending();
        if let Some(pending) = stream.pending() {
            self.ingest_block(pending);
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

fn finish_markdown_html(mut html_buf: String, profile: ParseProfile) -> String {
    if profile.uses_gfm_extensions() {
        html_buf = crate::html::tagfilter::apply_gfm_tagfilter(&html_buf);
    }
    html_buf
}

#[cfg(test)]
fn markdown_source_to_html(source: &str, profile: ParseProfile) -> String {
    let prepared = if profile.uses_gfm_extensions() {
        crate::parse::gfm_preprocess::apply_gfm_extended_autolinks(source)
    } else {
        source.to_string()
    };
    let options = profile.pulldown_options();
    let mut html_buf = String::new();
    html::push_html(
        &mut html_buf,
        Parser::new_ext(&prepared, options).map(|event| event.into_static()),
    );
    finish_markdown_html(html_buf, profile)
}

fn markdown_to_fragment(compiled: &CompiledMarkdown, profile: ParseProfile) -> HtmlFragment {
    let mut html_buf = String::new();
    html::push_html(&mut html_buf, compiled.events().iter().cloned());
    let html_buf = finish_markdown_html(html_buf, profile);
    if html_buf.is_empty() {
        HtmlFragment::from_html("<p></p>")
    } else {
        HtmlFragment::from_html(&html_buf)
    }
}

fn pending_markdown_to_fragment(source: &str, profile: ParseProfile) -> HtmlFragment {
    let prepared = if profile.uses_gfm_extensions() {
        crate::parse::gfm_preprocess::apply_gfm_extended_autolinks(source)
    } else {
        source.to_string()
    };
    let mut html_buf = String::new();
    html::push_html(
        &mut html_buf,
        Parser::new_ext(&prepared, profile.pulldown_options()).map(|event| event.into_static()),
    );
    let html_buf = finish_markdown_html(html_buf, profile);
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
    use crate::html::block_alignment::BlockAlignment;
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
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b.content, BlockContent::Html(_)))
        );
        let cache = BlockRenderCache::from_blocks(&blocks);
        assert!(
            cache
                .entries()
                .iter()
                .any(|entry| matches!(entry, CachedBlock::Fragment(_)))
        );
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

    fn fragment_has_tag(fragment: &HtmlFragment, expected: &str) -> bool {
        fn walk(
            fragment: &HtmlFragment,
            id: crate::html::fragment::NodeId,
            expected: &str,
        ) -> bool {
            match fragment.node(id) {
                Some(crate::html::fragment::HtmlNode::Element { tag, children, .. }) => {
                    tag.as_str() == expected
                        || children.iter().any(|&c| walk(fragment, c, expected))
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
    fn split_center_wrapper_assigns_alignment_to_inner_blocks() {
        let md = "<center>\n\n# Title\n\n</center>\n\n- outside\n";
        let doc =
            crate::core::document::Document::parse(md, ParseProfile::GitHubPreview).expect("parse");
        let cache = BlockRenderCache::from_document(&doc);
        let mut saw_centered_content = false;
        let mut saw_uncentered_list = false;
        for i in 0..cache.len() {
            if let Some(CachedBlock::Fragment(fragment)) = cache.entry(i) {
                if fragment_has_tag(fragment, "center") && fragment_has_tag(fragment, "h1") {
                    saw_centered_content = true;
                }
                if fragment_has_tag(fragment, "h1")
                    && cache.entry_alignment(i) == Some(BlockAlignment::Center)
                {
                    saw_centered_content = true;
                }
                if fragment_has_tag(fragment, "ul") {
                    assert_eq!(
                        cache.entry_alignment(i),
                        None,
                        "list after </center> must not inherit center"
                    );
                    saw_uncentered_list = true;
                }
            }
        }
        assert!(
            saw_centered_content,
            "center wrapper should coalesce or align inner heading"
        );
        assert!(saw_uncentered_list);
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
        assert!(
            has_blockquote,
            "fragment missing blockquote, kinds={kinds:?}"
        );
        for (i, block) in doc.blocks().iter().enumerate() {
            if block.kind != BlockKind::HtmlBlock {
                continue;
            }
            let Some(CachedBlock::Fragment(fragment)) = cache.entry(i) else {
                continue;
            };
            if block.source.trim().starts_with("<hr") {
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

    #[test]
    fn hello_example_code_fence_in_cached_code_block() {
        const HELLO: &str = r"
# Hello
```rust
fn demo() {}
```
";
        let doc = crate::core::document::Document::parse(HELLO, ParseProfile::GitHubPreview)
            .expect("parse");
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
        let doc = crate::core::document::Document::parse(SNIPPET, ParseProfile::GitHubPreview)
            .expect("parse");
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
        assert!(c.code.contains("fn main"), "code body wrong: {:?}", c.code);
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
                Parser::new(source).map(|e| e.into_static()).collect(),
            )),
        }];
        let cache = BlockRenderCache::from_blocks(&blocks);
        let CachedBlock::Fragment(fragment) = cache.entry(0).expect("entry") else {
            panic!("expected fragment");
        };
        assert!(!fragment.roots().is_empty());
    }

    #[test]
    fn platforms_list_item_html_structure() {
        use crate::core::document::Document;
        use crate::profile::ParseProfile;
        let md = r"## Platforms

(note: WIP)

- [x] Windows x86_64
- [x] Linux x86_64
";
        let html = Document::parse(md, ParseProfile::GitHubPreview)
            .unwrap()
            .to_html()
            .unwrap();
        let inputs = html.matches("<input").count();
        eprintln!("platforms html ({inputs} inputs):\n{html}");
        assert!(html.contains("checkbox"));
        assert_eq!(inputs, 2, "expected one input per item");
    }

    #[test]
    fn ql_platforms_full_file_input_count() {
        use super::markdown_source_to_html;
        use crate::profile::ParseProfile;
        let md = include_str!("../../examples/assets/QL_README.md");
        let html = markdown_source_to_html(md, ParseProfile::GitHubPreview);
        let start = html.find("<h2>Platforms</h2>").expect("platforms heading");
        let after = start + "<h2>Platforms</h2>".len();
        let end = html[after..]
            .find("<h2>")
            .map(|i| after + i)
            .unwrap_or(html.len());
        let section = &html[start..end];
        eprintln!("QL platforms section:\n{section}");
        let inputs = section.matches("<input").count();
        let lis = section.matches("<li").count();
        eprintln!("inputs={inputs} lis={lis}");
        assert!(
            inputs >= 10,
            "expected many platform checkboxes, got {inputs}"
        );
        assert!(
            section.contains("<p><input"),
            "QL platforms use li>p>input; renderer must not duplicate checkbox in body"
        );
    }

    #[test]
    fn ql_loaders_list_item_uses_inline_input_not_wrapped_in_p() {
        use super::markdown_source_to_html;
        use crate::profile::ParseProfile;
        let md = include_str!("../../examples/assets/QL_README.md");
        let html = markdown_source_to_html(md, ParseProfile::GitHubPreview);
        let start = html.find("<h3>Loaders</h3>").expect("loaders");
        let section = &html[start..start + 600];
        eprintln!("loaders snippet:\n{section}");
        assert!(
            section.contains("<li><input") || section.contains("<li>\n<input"),
            "loaders items often omit p wrapper"
        );
    }

    #[test]
    fn ql_and_test_have_task_list_blocks() {
        use crate::core::block::BlockKind;
        use crate::core::document::Document;
        use crate::profile::ParseProfile;
        let test = include_str!("../../examples/assets/TEST.md");
        let ql = include_str!("../../examples/assets/QL_README.md");
        let test_doc = Document::parse(test, ParseProfile::GitHubPreview).unwrap();
        let ql_doc = Document::parse(ql, ParseProfile::GitHubPreview).unwrap();
        let test_lists = test_doc
            .blocks()
            .iter()
            .filter(|b| b.kind == BlockKind::List)
            .count();
        let ql_lists = ql_doc
            .blocks()
            .iter()
            .filter(|b| b.kind == BlockKind::List)
            .count();
        assert!(
            test_lists >= 1,
            "TEST.md should have list blocks, got {test_lists}"
        );
        assert!(
            ql_lists >= 5,
            "QL_README should have many list blocks, got {ql_lists}"
        );
    }

    #[test]
    fn ql_coalesced_center_shield_p_has_four_direct_children() {
        use crate::core::document::Document;
        use crate::html::fragment::{HtmlFragment, HtmlNode};
        use crate::profile::ParseProfile;
        let ql = include_str!("../../examples/assets/QL_README.md");
        let doc = Document::parse(ql, ParseProfile::GitHubPreview).unwrap();
        let cache = BlockRenderCache::from_document(&doc);
        let mut found = 0usize;
        for i in 0..cache.len() {
            let Some(CachedBlock::Fragment(fragment)) = cache.entry(i) else {
                continue;
            };
            fn walk(f: &HtmlFragment, id: crate::html::fragment::NodeId, found: &mut usize) {
                let Some(HtmlNode::Element { tag, children, .. }) = f.node(id) else {
                    return;
                };
                if tag.as_str() == "p" {
                    let n = children
                        .iter()
                        .filter(|&&c| {
                            matches!(
                                f.node(c),
                                Some(HtmlNode::Element { tag, .. })
                                    if tag.as_str() == "img" || tag.as_str() == "a"
                            )
                        })
                        .count();
                    if n >= 4 {
                        *found = n;
                    }
                }
                for &c in children {
                    walk(f, c, found);
                }
            }
            for &root in fragment.roots() {
                walk(fragment, root, &mut found);
            }
        }
        eprintln!("coalesced cache shield-p direct children: {found}");
        assert!(
            found >= 4,
            "Document cache should preserve 4 badges in one <p>, got {found}"
        );
    }

    #[test]
    fn ql_shield_paragraph_dom_child_img_nodes() {
        use super::markdown_source_to_html;
        use crate::html::fragment::{HtmlFragment, HtmlNode};
        use crate::profile::ParseProfile;
        let html = markdown_source_to_html(
            include_str!("../../examples/assets/QL_README.md"),
            ParseProfile::GitHubPreview,
        );
        let fragment = HtmlFragment::from_html(&html);
        let mut shield_p_direct = 0usize;
        fn walk(f: &HtmlFragment, id: crate::html::fragment::NodeId, count: &mut usize) {
            let Some(HtmlNode::Element { tag, children, .. }) = f.node(id) else {
                return;
            };
            if tag.as_str() == "p" {
                let direct = children
                    .iter()
                    .filter(|&&c| {
                        matches!(
                            f.node(c),
                            Some(HtmlNode::Element { tag, .. })
                                if tag.as_str() == "img" || tag.as_str() == "a"
                        )
                    })
                    .count();
                if direct >= 3 {
                    *count = direct;
                }
            }
            for &c in children {
                walk(f, c, count);
            }
        }
        for &root in fragment.roots() {
            walk(&fragment, root, &mut shield_p_direct);
        }
        eprintln!("QL shield-p direct img/a children: {shield_p_direct}");
        assert!(
            shield_p_direct >= 4,
            "expected 4 badge nodes in one <p>, got {shield_p_direct}"
        );
    }

    #[test]
    fn ql_readme_shields_are_single_paragraph_with_four_imgs() {
        use super::markdown_source_to_html;
        use crate::profile::ParseProfile;
        let md = include_str!("../../examples/assets/QL_README.md");
        let html = markdown_source_to_html(md, ParseProfile::GitHubPreview);
        let start = html.find("center").expect("center");
        let end = html[start..].find("</div>").unwrap_or(500) + start;
        let header = &html[start..end];
        let p_start = header.find("<p><img").expect("shield paragraph");
        let p_end = header[p_start..].find("</p>").unwrap() + p_start;
        let shield_p = &header[p_start..p_end];
        assert_eq!(
            shield_p.matches("<img").count(),
            4,
            "QL packs all badges into one <p>"
        );
        assert!(shield_p.contains("<a href"), "iced badge is a>img");
    }

    #[test]
    fn ql_readme_center_does_not_wrap_todo_section() {
        use super::markdown_source_to_html;
        use crate::html::fragment::{HtmlFragment, HtmlNode};
        use crate::profile::ParseProfile;
        let md = include_str!("../../examples/assets/QL_README.md");
        let html = markdown_source_to_html(md, ParseProfile::GitHubPreview);
        let fragment = HtmlFragment::from_html(&html);
        fn find_todo_in_center(
            f: &HtmlFragment,
            id: crate::html::fragment::NodeId,
            in_center: bool,
        ) -> bool {
            match f.node(id) {
                Some(HtmlNode::Element { tag, children, .. }) => {
                    let in_center = in_center || tag.as_str() == "center";
                    children
                        .iter()
                        .any(|&c| find_todo_in_center(f, c, in_center))
                }
                Some(HtmlNode::Text(t)) => in_center && t.contains("To-do"),
                _ => false,
            }
        }
        assert!(
            !fragment
                .roots()
                .iter()
                .any(|&r| find_todo_in_center(&fragment, r, false)),
            "To-do must not be inside <center>"
        );
    }

    #[test]
    fn test_fixture_centered_inline_markdown_stays_single_paragraph_block() {
        use crate::core::document::Document;
        use crate::profile::ParseProfile;

        let md = include_str!("../../examples/assets/TEST.md");
        let doc = Document::parse(md, ParseProfile::GitHubPreview).expect("parse");
        let cache = BlockRenderCache::from_document(&doc);
        let matches: Vec<_> = doc
            .blocks()
            .iter()
            .enumerate()
            .filter(|(_, block)| block.source.contains("Normal"))
            .map(|(index, block)| {
                (
                    index,
                    block.kind,
                    cache.entry_alignment(index),
                    block.source.to_string(),
                )
            })
            .collect();

        eprintln!("blocks containing 'Normal': {matches:#?}");
        assert!(
            matches.iter().any(|(_, kind, align, source)| {
                matches!(kind, BlockKind::Paragraph | BlockKind::HtmlBlock)
                    && *align == Some(BlockAlignment::Center)
                    && (source.contains("**bold**") || source.contains("<strong>bold</strong>"))
                    && (source.contains("`code`") || source.contains("<code>code</code>"))
                    && (source.contains("[link]") || source.contains(">link</a>"))
            }),
            "expected centered paragraph or coalesced HTML block for inline markdown inside <center>; got {matches:#?}"
        );
    }
}
