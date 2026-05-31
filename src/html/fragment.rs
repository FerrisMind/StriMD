use std::fmt;
use std::sync::Arc;

#[cfg(feature = "static")]
use crate::core::error::HtmlFragmentError;

/// Index into an [`HtmlFragment`] node arena.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

/// Owned HTML tag name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HtmlTag(pub Arc<str>);

impl HtmlTag {
    #[must_use]
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Owned HTML attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlAttr {
    pub name: Arc<str>,
    pub value: Arc<str>,
}

/// One node in an [`HtmlFragment`] tree.
#[derive(Debug, Clone)]
pub enum HtmlNode {
    Element {
        tag: HtmlTag,
        attrs: Vec<HtmlAttr>,
        children: Vec<NodeId>,
    },
    Text(Arc<str>),
    Comment(Arc<str>),
}

/// Backend-agnostic parsed HTML fragment.
#[derive(Debug, Clone, Default)]
pub struct HtmlFragment {
    nodes: Vec<HtmlNode>,
    roots: Vec<NodeId>,
}

#[cfg_attr(not(feature = "static"), allow(dead_code))]
impl HtmlFragment {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_html(html: &str) -> Self {
        if html.is_empty() {
            return Self::empty();
        }
        let html = crate::html::preprocess::preprocess_raw_html(html);
        let html = html.as_ref();
        #[cfg(feature = "static")]
        {
            if let Ok(fragment) = crate::html::treesink::parse_html_fragment(html)
                && !fragment.roots().is_empty()
            {
                return fragment;
            }
        }
        let mut fragment = Self::empty();
        let text_id = fragment.push_text(Arc::from(html));
        fragment.roots.push(text_id);
        fragment
    }

    /// Parse HTML using html5ever when the `static` feature is enabled.
    #[cfg(feature = "static")]
    pub fn parse(html: &str) -> Result<Self, HtmlFragmentError> {
        if html.is_empty() {
            return Ok(Self::empty());
        }
        let html = crate::html::preprocess::preprocess_raw_html(html);
        crate::html::treesink::parse_html_fragment(html.as_ref())
    }

    #[must_use]
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&HtmlNode> {
        self.nodes.get(id.0)
    }

    pub(crate) fn push_root(&mut self, id: NodeId) {
        self.roots.push(id);
    }

    pub(crate) fn push_element(
        &mut self,
        tag: HtmlTag,
        attrs: Vec<HtmlAttr>,
        children: Vec<NodeId>,
    ) -> NodeId {
        self.push_node(HtmlNode::Element { tag, attrs, children })
    }

    pub(crate) fn push_text(&mut self, text: Arc<str>) -> NodeId {
        self.push_node(HtmlNode::Text(text))
    }

    pub(crate) fn push_comment(&mut self, text: Arc<str>) -> NodeId {
        self.push_node(HtmlNode::Comment(text))
    }

    fn push_node(&mut self, node: HtmlNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    /// Unwrap synthetic html5ever fragment wrappers (`html`, `body`, context `div`).
    pub(crate) fn normalize_roots(mut self) -> Self {
        loop {
            if self.roots.len() != 1 {
                break;
            }
            let root = self.roots[0];
            let Some(HtmlNode::Element { tag, children, .. }) = self.node(root).cloned() else {
                break;
            };
            match tag.as_str() {
                "html" | "body" => {
                    if children.len() == 1 {
                        self.roots = vec![children[0]];
                        continue;
                    }
                    self.roots = children;
                    break;
                }
                "div" if children.len() == 1 => {
                    self.roots = vec![children[0]];
                    continue;
                }
                "div" if children.len() > 1 => {
                    self.roots = children;
                    break;
                }
                _ => break,
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_fragment_construction() {
        let mut fragment = HtmlFragment::empty();
        let text = fragment.push_text(Arc::from("hello"));
        let root = fragment.push_element(HtmlTag::new("p"), Vec::new(), vec![text]);
        fragment.push_root(root);
        assert_eq!(fragment.roots().len(), 1);
    }

    #[cfg(feature = "static")]
    #[test]
    fn from_html_parses_element_tree() {
        let fragment = HtmlFragment::from_html("<details><summary>x</summary></details>");
        assert_eq!(fragment.roots().len(), 1);
        match fragment.node(fragment.roots()[0]) {
            Some(HtmlNode::Element { tag, .. }) => assert_eq!(tag.as_str(), "details"),
            other => panic!("expected element, got {other:?}"),
        }
    }
}
