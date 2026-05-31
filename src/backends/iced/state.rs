use std::collections::{HashMap, HashSet};

use crate::html::block_cache::{BlockRenderCache, CachedBlock};
use super::structs::{UpdateMsg, UpdateMsgKind};
use crate::core::document::Document;
use crate::core::block::RenderBlock;
use crate::html::fragment::HtmlFragment;
use crate::profile::ParseProfile;

#[cfg(feature = "stream")]
use crate::core::StreamDocument;

use super::dom::DomRef;

/// The state of the document.
///
/// - Put this in your Application struct.
/// - Use [`Self::with_html`] and [`Self::with_html_and_markdown`]
///   functions to create this.
/// - Create a new one if the document changes
///
/// ```no_run
/// # use strimd::MarkState;
/// # const YOUR_TEXT: &str = "";
/// # fn e() { let m =
/// MarkState::with_html(YOUR_TEXT)
/// # ;
/// // or if you just want HTML
/// # let m =
/// MarkState::with_html(YOUR_TEXT)
/// # ; }
/// ```
pub struct MarkState {
    pub(crate) cache: Option<BlockRenderCache>,
    pub(crate) dropdown_state: HashMap<usize, bool>,
}

impl MarkState {
    /// Processes documents containing **pure HTML**, without any Markdown support.
    #[must_use]
    pub fn with_html(input: &str) -> Self {
        let fragment = HtmlFragment::from_html(input);
        let block = RenderBlock {
            id: crate::core::ids::BlockId::new(0),
            status: crate::core::block::BlockStatus::Committed,
            kind: crate::core::block::BlockKind::HtmlBlock,
            source: std::sync::Arc::from(input),
            content: crate::core::block::BlockContent::Html(fragment),
        };
        Self::from_blocks(std::slice::from_ref(&block))
    }

    /// Build iced state from a parsed [`Document`] (pulldown block model).
    #[must_use]
    pub fn from_document(document: &Document) -> Self {
        let cache = BlockRenderCache::from_document(document);
        let mut dropdown_state = HashMap::new();
        let mut dropdown_counter = 0;
        scan_block_cache_dropdowns(&cache, &mut dropdown_state, &mut dropdown_counter);
        Self {
            cache: Some(cache),
            dropdown_state,
        }
    }

    /// Build iced state from backend-agnostic [`RenderBlock`] values.
    #[must_use]
    pub fn from_blocks(blocks: &[RenderBlock]) -> Self {
        let cache = BlockRenderCache::from_blocks(blocks);
        let mut dropdown_state = HashMap::new();
        let mut dropdown_counter = 0;
        scan_block_cache_dropdowns(&cache, &mut dropdown_state, &mut dropdown_counter);

        Self {
            cache: Some(cache),
            dropdown_state,
        }
    }

    /// Replace block cache from a streaming document snapshot.
    #[cfg(feature = "stream")]
    pub fn sync_from_stream(&mut self, stream: &StreamDocument) {
        let mut cache = BlockRenderCache::default();
        cache.sync_from_stream(stream);
        self.cache = Some(cache);
        self.dropdown_state.clear();
        let mut dropdown_counter = 0;
        if let Some(cache) = &self.cache {
            scan_block_cache_dropdowns(cache, &mut self.dropdown_state, &mut dropdown_counter);
        }
    }

    /// Processes documents containing both **HTML and Markdown** (pulldown path).
    #[must_use]
    pub fn with_html_and_markdown(input: &str) -> Self {
        Self::from_parsed_markdown(input, ParseProfile::GitHubPreview)
    }

    /// Processes documents with **Markdown only** (raw HTML escaped per strict profile).
    #[must_use]
    pub fn with_markdown_only(input: &str) -> Self {
        Self::from_parsed_markdown(input, ParseProfile::StrictCommonMark)
    }

    fn from_parsed_markdown(input: &str, profile: ParseProfile) -> Self {
        match Document::parse(input, profile) {
            Ok(document) => Self::from_document(&document),
            Err(_) => Self::from_blocks(&[]),
        }
    }

    /// Updates the internal state of the document.
    pub fn update(&mut self, action: UpdateMsg) {
        let UpdateMsgKind::DetailsToggle(id, action) = action.kind;
        self.dropdown_state.insert(id, action);
    }

    /// Retrieves all image URLs that need to be loaded.
    #[must_use]
    pub fn find_image_links(&self) -> HashSet<String> {
        let mut storage = HashSet::new();
        if let Some(cache) = &self.cache {
            for entry in cache.entries() {
                if let CachedBlock::Fragment(fragment) = entry {
                    for root in DomRef::fragment_roots(fragment) {
                        find_image_links_dom(root, &mut storage);
                    }
                }
            }
        }
        storage
    }
}

impl Default for MarkState {
    fn default() -> Self {
        Self::with_html("")
    }
}

fn scan_block_cache_dropdowns(
    cache: &BlockRenderCache,
    dropdown_state: &mut HashMap<usize, bool>,
    dropdown_counter: &mut usize,
) {
    for index in 0..cache.len() {
        if let Some(CachedBlock::Fragment(fragment)) = cache.entry(index) {
            for root in DomRef::fragment_roots(fragment) {
                find_state_dom(root, dropdown_state, dropdown_counter);
            }
        }
    }
}

fn find_state_dom(
    node: DomRef<'_>,
    dropdown_state: &mut HashMap<usize, bool>,
    dropdown_counter: &mut usize,
) {
    if node.tag_name() == Some("details") {
        dropdown_state.insert(*dropdown_counter, false);
        *dropdown_counter += 1;
    }
    for child in node.children() {
        find_state_dom(child, dropdown_state, dropdown_counter);
    }
}

fn find_image_links_dom(node: DomRef<'_>, storage: &mut HashSet<String>) {
    if node.tag_name() == Some("img")
        && let Some(url) = node.get_attr("src")
        && !url.is_empty()
    {
        storage.insert(url);
    }
    for child in node.children() {
        find_image_links_dom(child, storage);
    }
}
