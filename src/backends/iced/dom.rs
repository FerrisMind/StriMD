//! Unified DOM cursor for iced rendering over [`HtmlFragment`] node arenas.

use std::borrow::Cow;

use crate::html::fragment::{HtmlFragment, HtmlNode, NodeId};

/// A single node in an [`HtmlFragment`] tree.
#[derive(Clone, Copy)]
pub(crate) struct DomRef<'a> {
    fragment: &'a HtmlFragment,
    id: NodeId,
}

impl<'a> DomRef<'a> {
    #[must_use]
    pub(crate) fn new(fragment: &'a HtmlFragment, id: NodeId) -> Self {
        Self { fragment, id }
    }

    #[must_use]
    pub(crate) fn fragment_roots(fragment: &'a HtmlFragment) -> Vec<DomRef<'a>> {
        fragment
            .roots()
            .iter()
            .map(|&id| Self::new(fragment, id))
            .collect()
    }

    #[must_use]
    pub(crate) fn is_document_root(&self) -> bool {
        self.fragment.roots().contains(&self.id)
    }

    #[must_use]
    pub(crate) fn tag_name(&self) -> Option<&str> {
        match self.fragment.node(self.id)? {
            HtmlNode::Element { tag, .. } => Some(tag.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub(crate) fn text_contents(&self) -> Option<Cow<'a, str>> {
        match self.fragment.node(self.id)? {
            HtmlNode::Text(text) => Some(Cow::Borrowed(text.as_ref())),
            _ => None,
        }
    }

    #[must_use]
    pub(crate) fn children(&self) -> Vec<DomRef<'a>> {
        match self.fragment.node(self.id) {
            Some(HtmlNode::Element { children, .. }) => children
                .iter()
                .map(|&child_id| Self::new(self.fragment, child_id))
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn for_each_attr<F>(&self, mut f: F)
    where
        F: FnMut(&str, &str),
    {
        if let Some(HtmlNode::Element { attrs, .. }) = self.fragment.node(self.id) {
            for attr in attrs {
                f(attr.name.as_ref(), attr.value.as_ref());
            }
        }
    }

    #[must_use]
    pub(crate) fn get_attr(&self, name: &str) -> Option<String> {
        let mut found = None;
        self.for_each_attr(|attr_name, value| {
            if attr_name == name {
                found = Some(value.to_string());
            }
        });
        found
    }

    #[must_use]
    pub(crate) fn is_useless(&self) -> bool {
        self.text_contents()
            .is_some_and(|text| text.trim().is_empty())
    }

    #[must_use]
    pub(crate) fn is_block_element(&self) -> bool {
        let Some(name) = self.tag_name() else {
            return false;
        };
        matches!(
            name,
            "address"
                | "article"
                | "aside"
                | "blockquote"
                | "canvas"
                | "dd"
                | "div"
                | "dl"
                | "dt"
                | "fieldset"
                | "figcaption"
                | "figure"
                | "footer"
                | "form"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "header"
                | "hr"
                | "li"
                | "main"
                | "nav"
                | "noscript"
                | "ol"
                | "p"
                | "pre"
                | "section"
                | "table"
                | "tfoot"
                | "ul"
                | "video"
                | "br"
                | "details"
                | "summary"
        )
    }

    #[must_use]
    pub(crate) fn has_task_checkbox_child(&self) -> bool {
        self.children().iter().any(|child| {
            child.tag_name() == Some("input") && child.get_attr("type").as_deref() == Some("checkbox")
        })
    }
}

pub(crate) fn alignment_read(data: &mut crate::backends::iced::structs::ChildData, dom: DomRef<'_>) {
    let Some(align) = dom.get_attr("align") else {
        return;
    };
    use crate::backends::iced::structs::ChildAlignment;

    if matches!(align.as_str(), "right" | "center" | "centre") {
        data.alignment = Some(if align == "right" {
            ChildAlignment::Right
        } else {
            ChildAlignment::Center
        });
    } else if align == "left" {
        data.alignment = None;
    }
}
