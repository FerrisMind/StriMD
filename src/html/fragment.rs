use std::fmt;
use std::sync::Arc;

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

impl HtmlFragment {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_html(html: &str) -> Self {
        // Phase 2: html5ever TreeSink integration. For now preserve source as text root.
        let mut fragment = Self::empty();
        if html.is_empty() {
            return fragment;
        }
        let text_id = fragment.push_node(HtmlNode::Text(Arc::from(html)));
        fragment.roots.push(text_id);
        fragment
    }

    #[must_use]
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&HtmlNode> {
        self.nodes.get(id.0)
    }

    fn push_node(&mut self, node: HtmlNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }
}
