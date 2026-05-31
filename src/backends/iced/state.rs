use std::collections::{HashMap, HashSet};

use markup5ever_rcdom::RcDom;

use crate::html::block_cache::{BlockRenderCache, CachedBlock};
use super::structs::{UpdateMsg, UpdateMsgKind};
use crate::core::document::Document;
use crate::core::block::RenderBlock;
use crate::html::rcdom_compat::html_to_rcdom;

#[cfg(feature = "stream")]
use crate::core::StreamDocument;

/// Source DOM for [`MarkState`]: legacy full-document tree or block cache.
pub(crate) enum MarkStateSource {
    LegacyDom(RcDom),
    Blocks(BlockRenderCache),
}

/// The state of the document.
///
/// - Put this in your Application struct.
/// - Use [`Self::with_html`] and [`Self::with_html_and_markdown`]
///   functions to create this.
/// - Create a new one if the document changes
///
/// ```no_run
/// # use frostmark::MarkState;
/// # const YOUR_TEXT: &str = "";
/// # fn e() { let m =
/// MarkState::with_html_and_markdown(YOUR_TEXT)
/// # ;
/// // or if you just want HTML
/// # let m =
/// MarkState::with_html(YOUR_TEXT)
/// # ; }
/// ```
pub struct MarkState {
    pub(crate) source: MarkStateSource,
    pub(crate) dropdown_state: HashMap<usize, bool>,
}

impl MarkState {
    /// Processes documents containing **pure HTML**,
    /// without any Markdown support.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn with_html(input: &str) -> Self {
        let dom = html_to_rcdom(input);
        let mut dropdown_state = HashMap::new();
        let mut dropdown_counter = 0;
        find_state(&dom.document, &mut dropdown_state, &mut dropdown_counter);

        Self {
            source: MarkStateSource::LegacyDom(dom),
            dropdown_state,
        }
    }

    /// Build iced state from a parsed [`Document`] (pulldown block model).
    #[must_use]
    pub fn from_document(document: &Document) -> Self {
        let cache = BlockRenderCache::from_document(document);
        let mut dropdown_state = HashMap::new();
        let mut dropdown_counter = 0;
        scan_block_cache_dropdowns(&cache, &mut dropdown_state, &mut dropdown_counter);
        Self {
            source: MarkStateSource::Blocks(cache),
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
            source: MarkStateSource::Blocks(cache),
            dropdown_state,
        }
    }

    /// Replace block cache from a streaming document snapshot.
    #[cfg(feature = "stream")]
    pub fn sync_from_stream(&mut self, stream: &StreamDocument) {
        match &mut self.source {
            MarkStateSource::Blocks(cache) => {
                cache.sync_from_stream(stream);
            }
            MarkStateSource::LegacyDom(_) => {
                let mut cache = BlockRenderCache::default();
                cache.sync_from_stream(stream);
                self.source = MarkStateSource::Blocks(cache);
            }
        }
        self.dropdown_state.clear();
        let mut dropdown_counter = 0;
        if let MarkStateSource::Blocks(cache) = &self.source {
            scan_block_cache_dropdowns(cache, &mut self.dropdown_state, &mut dropdown_counter);
        }
    }

    /// Processes documents containing both
    /// **HTML and Markdown** (or a mix of both).
    #[must_use]
    #[cfg(feature = "_legacy_comrak")]
    pub fn with_html_and_markdown(input: &str) -> Self {
        let html = crate::parse::comrak_migration::markdown_to_html(input);
        Self::with_html(&html)
    }

    /// Processes documents containing **pure Markdown**,
    /// filtering out any HTML content.
    #[must_use]
    #[cfg(feature = "_legacy_comrak")]
    pub fn with_markdown_only(input: &str) -> Self {
        let mut out = String::new();
        _ = comrak::html::escape(&mut out, input);
        Self::with_html_and_markdown(&out)
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
        match &self.source {
            MarkStateSource::LegacyDom(dom) => {
                find_image_links(&dom.document, &mut storage);
            }
            MarkStateSource::Blocks(cache) => {
                for entry in cache.entries() {
                    if let CachedBlock::Dom(dom) = entry {
                        find_image_links(&dom.document, &mut storage);
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
        if let Some(CachedBlock::Dom(dom)) = cache.entry(index) {
            find_state(&dom.document, dropdown_state, dropdown_counter);
        }
    }
}

fn find_state(
    node: &markup5ever_rcdom::Node,
    dropdown_state: &mut HashMap<usize, bool>,
    dropdown_counter: &mut usize,
) {
    let borrow = node.children.borrow();
    match &node.data {
        markup5ever_rcdom::NodeData::Element { name, .. } if &name.local == "details" => {
            dropdown_state.insert(*dropdown_counter, false);
            *dropdown_counter += 1;
            for child in &*borrow {
                find_state(child, dropdown_state, dropdown_counter);
            }
        }
        _ => {
            for child in &*borrow {
                find_state(child, dropdown_state, dropdown_counter);
            }
        }
    }
}

fn find_image_links(node: &markup5ever_rcdom::Node, storage: &mut HashSet<String>) {
    let borrow = node.children.borrow();
    match &node.data {
        markup5ever_rcdom::NodeData::Element { name, attrs, .. } if &name.local == "img" => {
            let attrs = attrs.borrow();
            if let Some(attr) = attrs.iter().find(|attr| &*attr.name.local == "src") {
                let url = &*attr.value;
                if !url.is_empty() {
                    storage.insert(url.to_owned());
                }
            }
        }
        _ => {
            for child in &*borrow {
                find_image_links(child, storage);
            }
        }
    }
}
