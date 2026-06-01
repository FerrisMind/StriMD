use crate::core::block::BlockContent;
use crate::core::error::UnsupportedReason;
use crate::html::fragment::{HtmlFragment, HtmlNode, NodeId};
use crate::options::RawHtmlPolicy;

/// Tags that must never be rendered or exported as live HTML.
const UNSAFE_TAGS: &[&str] = &[
    "script", "iframe", "object", "embed", "link", "style", "meta", "base",
];

/// Build block content from raw HTML according to [`RawHtmlPolicy`].
#[must_use]
pub fn block_content_from_raw_html(html: &str, policy: RawHtmlPolicy) -> BlockContent {
    match policy {
        RawHtmlPolicy::Preserve => BlockContent::Html(HtmlFragment::from_html(html)),
        RawHtmlPolicy::Escape => BlockContent::Unsupported {
            reason: UnsupportedReason::Policy(
                "raw HTML escaped (use Preserve or StripUnsupported)".into(),
            ),
        },
        RawHtmlPolicy::StripUnsupported => {
            if let Some(tag) = unsafe_tag_in_raw_html(html) {
                return BlockContent::Unsupported {
                    reason: UnsupportedReason::HtmlTag(tag),
                };
            }
            content_from_fragment(HtmlFragment::from_html(html))
        }
    }
}

fn unsafe_tag_in_raw_html(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    for tag in UNSAFE_TAGS {
        let needle = format!("<{tag}");
        let Some(pos) = lower.find(&needle) else {
            continue;
        };
        let after = lower.as_bytes().get(pos + needle.len());
        if matches!(
            after,
            Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
        ) {
            return Some(tag.to_string());
        }
    }
    None
}

/// Reject or keep a parsed [`HtmlFragment`] based on unsafe tag policy.
#[must_use]
pub fn content_from_fragment(fragment: HtmlFragment) -> BlockContent {
    if let Some(tag) = first_unsafe_tag(&fragment) {
        BlockContent::Unsupported {
            reason: UnsupportedReason::HtmlTag(tag),
        }
    } else {
        BlockContent::Html(fragment)
    }
}

fn first_unsafe_tag(fragment: &HtmlFragment) -> Option<String> {
    for &root in fragment.roots() {
        if let Some(tag) = walk_for_unsafe(fragment, root) {
            return Some(tag);
        }
    }
    None
}

fn walk_for_unsafe(fragment: &HtmlFragment, id: NodeId) -> Option<String> {
    let node = fragment.node(id)?;
    match node {
        HtmlNode::Element { tag, children, .. } => {
            if UNSAFE_TAGS.contains(&tag.as_str()) {
                return Some(tag.as_str().to_string());
            }
            for child in children {
                if let Some(found) = walk_for_unsafe(fragment, *child) {
                    return Some(found);
                }
            }
            None
        }
        HtmlNode::Text(_) | HtmlNode::Comment(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_keeps_details() {
        let content = block_content_from_raw_html(
            "<details><summary>x</summary></details>",
            RawHtmlPolicy::Preserve,
        );
        assert!(matches!(content, BlockContent::Html(_)));
    }

    #[test]
    fn strip_unsupported_rejects_script() {
        let content = block_content_from_raw_html(
            "<p>ok</p><script>alert(1)</script>",
            RawHtmlPolicy::StripUnsupported,
        );
        assert!(matches!(
            content,
            BlockContent::Unsupported {
                reason: UnsupportedReason::HtmlTag(_)
            }
        ));
    }

    #[test]
    fn escape_policy_does_not_emit_html_block() {
        let content = block_content_from_raw_html("<span>x</span>", RawHtmlPolicy::Escape);
        assert!(matches!(content, BlockContent::Unsupported { .. }));
    }

    #[test]
    fn nested_script_is_detected() {
        let content = block_content_from_raw_html(
            "<div><script>x</script></div>",
            RawHtmlPolicy::StripUnsupported,
        );
        assert!(matches!(content, BlockContent::Unsupported { .. }));
    }
}
