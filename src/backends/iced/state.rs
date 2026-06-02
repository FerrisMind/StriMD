use std::collections::{HashMap, HashSet};

use super::structs::{UpdateMsg, UpdateMsgKind};
use crate::core::block::{BlockStatus, RenderBlock};
use crate::core::document::Document;
use crate::core::ids::BlockId;
use crate::html::block_cache::{BlockRenderCache, CachedBlock};
use crate::html::fragment::{HtmlFragment, NodeId};
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
    pub(crate) dropdown_nodes: HashMap<(BlockId, NodeId), usize>,
    dropdown_blocks: HashMap<BlockId, Vec<(NodeId, usize)>>,
    next_dropdown_id: usize,
    pending_dropdown_block: Option<BlockId>,
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
        let mut state = Self {
            cache: Some(cache),
            dropdown_state: HashMap::new(),
            dropdown_nodes: HashMap::new(),
            dropdown_blocks: HashMap::new(),
            next_dropdown_id: 0,
            pending_dropdown_block: None,
        };
        state.rebuild_dropdown_state();
        state
    }

    /// Build iced state from backend-agnostic [`RenderBlock`] values.
    #[must_use]
    pub fn from_blocks(blocks: &[RenderBlock]) -> Self {
        let cache = BlockRenderCache::from_blocks(blocks);
        let mut state = Self {
            cache: Some(cache),
            dropdown_state: HashMap::new(),
            dropdown_nodes: HashMap::new(),
            dropdown_blocks: HashMap::new(),
            next_dropdown_id: 0,
            pending_dropdown_block: None,
        };
        state.rebuild_dropdown_state();
        state
    }

    /// Replace block cache from a streaming document snapshot.
    #[cfg(feature = "stream")]
    pub fn sync_from_stream(&mut self, stream: &StreamDocument) {
        let mut cache = BlockRenderCache::default();
        cache.sync_from_stream(stream);
        self.cache = Some(cache);
        self.rebuild_dropdown_state();
    }

    /// Apply an incremental streaming update while preserving existing cache entries where possible.
    #[cfg(feature = "stream")]
    pub fn apply_stream_update(
        &mut self,
        stream: &StreamDocument,
        update: &crate::core::StreamUpdate,
    ) {
        if let Some(cache) = &mut self.cache {
            cache.apply_stream_update(stream, update);
        } else {
            let mut cache = BlockRenderCache::default();
            cache.sync_from_stream(stream);
            self.cache = Some(cache);
        }
        if update.reset || matches!(update.patch, crate::core::StreamPatch::ClearAndRebuild) {
            self.rebuild_dropdown_state();
            return;
        }

        let previous_pending = self.pending_dropdown_block;
        let current_pending = stream.pending().map(|block| block.id);
        if let Some(previous) = previous_pending.filter(|id| Some(*id) != current_pending) {
            self.remove_dropdown_block(previous);
        }

        let mut touched = Vec::new();
        match &update.patch {
            crate::core::StreamPatch::AppendCommitted { blocks } => {
                touched.extend(blocks.iter().copied());
            }
            crate::core::StreamPatch::ReplaceCommitted { id } => touched.push(*id),
            crate::core::StreamPatch::ReplacePending
            | crate::core::StreamPatch::Noop
            | crate::core::StreamPatch::ClearAndRebuild => {}
        }
        touched.extend(update.invalidated.iter().copied());
        if let Some(pending) = current_pending {
            touched.push(pending);
        }
        touched.sort_unstable();
        touched.dedup();

        let scans = if let Some(cache) = &self.cache {
            touched
                .into_iter()
                .map(|block_id| (block_id, scan_dropdown_block(cache, block_id)))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        for (block_id, details) in scans {
            self.refresh_dropdown_block(block_id, details);
        }
        self.pending_dropdown_block = current_pending;
    }

    pub(crate) fn dropdown_id_for(&self, block_id: BlockId, node_id: NodeId) -> Option<usize> {
        self.dropdown_nodes.get(&(block_id, node_id)).copied()
    }

    fn rebuild_dropdown_state(&mut self) {
        self.dropdown_state.clear();
        self.dropdown_nodes.clear();
        self.dropdown_blocks.clear();
        self.next_dropdown_id = 0;
        self.pending_dropdown_block = None;

        let scanned = if let Some(cache) = &self.cache {
            (0..cache.len())
                .filter_map(|index| {
                    let block_id = cache.block_id(index)?;
                    Some((
                        block_id,
                        cache.status(index),
                        scan_dropdown_block(cache, block_id),
                    ))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        for (block_id, status, details) in scanned {
            self.refresh_dropdown_block(block_id, details);
            if status == Some(BlockStatus::Pending) {
                self.pending_dropdown_block = Some(block_id);
            }
        }
    }

    fn refresh_dropdown_block(&mut self, block_id: BlockId, details: Vec<(NodeId, bool)>) {
        let previous = self
            .dropdown_blocks
            .remove(&block_id)
            .unwrap_or_default()
            .into_iter()
            .collect::<HashMap<_, _>>();
        for node_id in previous.keys() {
            self.dropdown_nodes.remove(&(block_id, *node_id));
        }

        if details.is_empty() {
            for id in previous.into_values() {
                self.dropdown_state.remove(&id);
            }
            return;
        }

        let mut reusable = previous;
        let mut refreshed = Vec::with_capacity(details.len());
        for (node_id, open) in details {
            let id = reusable.remove(&node_id).unwrap_or_else(|| {
                let id = self.next_dropdown_id;
                self.next_dropdown_id += 1;
                id
            });
            self.dropdown_state.entry(id).or_insert(open);
            self.dropdown_nodes.insert((block_id, node_id), id);
            refreshed.push((node_id, id));
        }

        for id in reusable.into_values() {
            self.dropdown_state.remove(&id);
        }

        self.dropdown_blocks.insert(block_id, refreshed);
    }

    #[cfg(feature = "stream")]
    fn remove_dropdown_block(&mut self, block_id: BlockId) {
        let Some(entries) = self.dropdown_blocks.remove(&block_id) else {
            return;
        };
        for (node_id, id) in entries {
            self.dropdown_nodes.remove(&(block_id, node_id));
            self.dropdown_state.remove(&id);
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
        match profile {
            ParseProfile::StrictCommonMark => match Document::parse(input, profile) {
                Ok(document) => Self::from_document(&document),
                Err(_) => Self::from_blocks(&[]),
            },
            ParseProfile::GitHubPreview | ParseProfile::ChatStream => {
                match Document::parse(input, profile) {
                    Ok(document) => Self::from_document(&document),
                    Err(_) => Self::from_blocks(&[]),
                }
            }
        }
    }

    /// Updates the internal state of the document.
    pub fn update(&mut self, action: UpdateMsg) {
        match action.kind {
            UpdateMsgKind::DetailsToggle(id, open) => {
                self.dropdown_state.insert(id, open);
            }
        }
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

fn scan_dropdown_block(cache: &BlockRenderCache, block_id: BlockId) -> Vec<(NodeId, bool)> {
    let Some(index) = cache.index_of(block_id) else {
        return Vec::new();
    };
    let Some(CachedBlock::Fragment(fragment)) = cache.entry(index) else {
        return Vec::new();
    };
    let mut details = Vec::new();
    for root in DomRef::fragment_roots(fragment) {
        find_state_dom(root, &mut details);
    }
    details
}

fn find_state_dom(node: DomRef<'_>, details: &mut Vec<(NodeId, bool)>) {
    if node.tag_name() == Some("details") {
        details.push((node.id(), node.get_attr("open").is_some()));
    }
    for child in node.children_iter() {
        find_state_dom(child, details);
    }
}

fn find_image_links_dom(node: DomRef<'_>, storage: &mut HashSet<String>) {
    if node.tag_name() == Some("img")
        && let Some(url) = node.get_attr("src")
        && !url.is_empty()
    {
        storage.insert(url.to_string());
    }
    for child in node.children_iter() {
        find_image_links_dom(child, storage);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "stream")]
    use crate::{StreamDocument, StreamOptions};

    use super::MarkState;

    #[test]
    fn details_open_attribute_initializes_dropdown_state() {
        let state = MarkState::with_html_and_markdown(
            "<details open><summary>Open</summary><p>x</p></details>\
             <details><summary>Closed</summary><p>y</p></details>",
        );
        assert_eq!(state.dropdown_state.get(&0), Some(&true));
        assert_eq!(state.dropdown_state.get(&1), Some(&false));
    }

    #[cfg(feature = "stream")]
    #[test]
    fn streaming_append_preserves_existing_details_toggle_state() {
        let mut stream = StreamDocument::new(StreamOptions::chat());
        let mut state = MarkState::from_blocks(&[]);

        let update = stream.append("<details><summary>Open</summary><p>Body</p></details>\n\n");
        state.apply_stream_update(&stream, &update);
        let details_id = *state
            .dropdown_state
            .keys()
            .next()
            .expect("details dropdown id");
        state.update(crate::backends::iced::structs::UpdateMsg {
            kind: crate::backends::iced::structs::UpdateMsgKind::DetailsToggle(details_id, true),
        });

        let update = stream.append("Trailing paragraph.\n");
        state.apply_stream_update(&stream, &update);

        assert_eq!(state.dropdown_state.get(&details_id), Some(&true));
        assert_eq!(state.dropdown_state.len(), 1);
    }
}
