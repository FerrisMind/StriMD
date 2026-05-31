//! Detect legacy `<center>` / `<div align="center">` wrapper blocks from pulldown output.

use crate::core::block::{BlockContent, RenderBlock};
use crate::html::fragment::{HtmlFragment, HtmlNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockAlignment {
    Center,
    Right,
}

fn fragment_root_tag(fragment: &HtmlFragment, id: crate::html::fragment::NodeId) -> Option<&str> {
    match fragment.node(id) {
        Some(HtmlNode::Element { tag, .. }) => Some(tag.as_str()),
        _ => None,
    }
}

fn fragment_root_text(fragment: &HtmlFragment, id: crate::html::fragment::NodeId) -> Option<&str> {
    match fragment.node(id) {
        Some(HtmlNode::Text(t)) => Some(t.as_ref()),
        _ => None,
    }
}

fn alignment_from_div_attrs(attrs: &[crate::html::fragment::HtmlAttr]) -> Option<BlockAlignment> {
    for attr in attrs {
        if attr.name.as_ref() == "align" {
            return match attr.value.as_ref() {
                "center" | "centre" => Some(BlockAlignment::Center),
                "right" => Some(BlockAlignment::Right),
                _ => None,
            };
        }
    }
    None
}

/// Opening wrapper tag emitted as its own pulldown HTML block.
#[must_use]
pub(crate) fn block_opens_alignment_wrapper(block: &RenderBlock) -> Option<BlockAlignment> {
    let BlockContent::Html(fragment) = &block.content else {
        return None;
    };
    for &root in fragment.roots() {
        if fragment_root_tag(fragment, root) == Some("center") {
            return Some(BlockAlignment::Center);
        }
        if let Some(HtmlNode::Element { tag, attrs, .. }) = fragment.node(root) {
            if tag.as_str() == "div" {
                if let Some(align) = alignment_from_div_attrs(attrs) {
                    return Some(align);
                }
            }
        }
    }
    None
}

/// Closing wrapper tag block (skip rendering), or a self-contained `<center>…</center>` / `<div align>…</div>` chunk.
#[must_use]
pub(crate) fn block_closes_alignment_wrapper(block: &RenderBlock) -> bool {
    if fragment_is_complete_alignment_wrapper(block) {
        return true;
    }
    let BlockContent::Html(fragment) = &block.content else {
        return false;
    };
    fragment.roots().iter().all(|&root| {
        fragment_root_text(fragment, root)
            .is_some_and(|t| matches!(t.trim(), "</center>" | "</div>"))
    }) && !fragment.roots().is_empty()
}

/// Single HTML block produced by wrapper coalescing (`<center>` + markdown + `</center>`).
#[must_use]
pub(crate) fn fragment_is_complete_alignment_wrapper(block: &RenderBlock) -> bool {
    let BlockContent::Html(fragment) = &block.content else {
        return false;
    };
    for &root in fragment.roots() {
        if let Some(HtmlNode::Element { tag, children, .. }) = fragment.node(root) {
            if tag.as_str() == "center" && !children.is_empty() {
                return true;
            }
            if tag.as_str() == "div" {
                if let Some(HtmlNode::Element { attrs, children, .. }) = fragment.node(root) {
                    if alignment_from_div_attrs(attrs).is_some() && !children.is_empty() {
                        return true;
                    }
                }
            }
        }
    }
    false
}
