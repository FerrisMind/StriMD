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
    pub(crate) fn get_attr(&self, name: &str) -> Option<&'a str> {
        let Some(HtmlNode::Element { attrs, .. }) = self.fragment.node(self.id) else {
            return None;
        };
        attrs
            .iter()
            .find(|attr| attr.name.as_ref() == name)
            .map(|attr| attr.value.as_ref())
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
                | "center"
        )
    }

    #[must_use]
    pub(crate) fn is_task_checkbox(node: DomRef<'_>) -> bool {
        node.tag_name() == Some("input") && node.get_attr("type") == Some("checkbox")
    }

    #[must_use]
    pub(crate) fn direct_task_checkbox(&self) -> Option<DomRef<'a>> {
        if let Some(node) = self
            .children()
            .into_iter()
            .find(|child| Self::is_task_checkbox(*child))
        {
            return Some(node);
        }
        for child in self.children() {
            if child.tag_name() == Some("p")
                && let Some(input) = child
                    .children()
                    .into_iter()
                    .find(|c| Self::is_task_checkbox(*c))
            {
                return Some(input);
            }
        }
        None
    }

    #[must_use]
    pub(crate) fn accumulated_text(&self) -> String {
        let mut out = String::new();
        self.accumulate_text(&mut out);
        out
    }

    fn accumulate_text(&self, out: &mut String) {
        match self.fragment.node(self.id) {
            Some(HtmlNode::Text(text)) => out.push_str(text.as_ref()),
            Some(HtmlNode::Element { children, .. }) => {
                for &child in children {
                    Self::new(self.fragment, child).accumulate_text(out);
                }
            }
            _ => {}
        }
    }

    /// Paragraph that only contains badge/shield image(s), optionally wrapped in a link.
    #[must_use]
    pub(crate) fn is_shield_paragraph(&self) -> bool {
        if self.tag_name() != Some("p") {
            return false;
        }
        let meaningful: Vec<_> = self
            .children()
            .into_iter()
            .filter(|c| !c.is_useless())
            .collect();
        let mut has_badge = false;
        for child in meaningful {
            match child.tag_name() {
                Some("img") => {
                    if !Self::is_badge_image(child) {
                        return false;
                    }
                    has_badge = true;
                }
                Some("a") => {
                    let kids: Vec<_> = child
                        .children()
                        .into_iter()
                        .filter(|c| !c.is_useless())
                        .collect();
                    if kids.len() != 1 || kids[0].tag_name() != Some("img") {
                        return false;
                    }
                    if !Self::is_badge_image(kids[0]) {
                        return false;
                    }
                    has_badge = true;
                }
                _ => return false,
            }
        }
        has_badge
    }

    #[must_use]
    fn is_badge_image(node: DomRef<'_>) -> bool {
        let Some(src) = node.get_attr("src") else {
            return false;
        };
        let lower = src.to_ascii_lowercase();
        lower.contains("img.shields.io")
            || lower.contains("shields.io/")
            || lower.contains("badge.svg")
            || lower.contains("/badge")
    }
}

pub(crate) fn alignment_read(
    data: &mut crate::backends::iced::structs::ChildData,
    dom: DomRef<'_>,
) {
    let Some(align) = dom.get_attr("align") else {
        return;
    };
    use crate::backends::iced::structs::ChildAlignment;

    if matches!(align, "right" | "center" | "centre") {
        data.alignment = Some(if align == "right" {
            ChildAlignment::Right
        } else {
            ChildAlignment::Center
        });
    } else if align == "left" {
        data.alignment = None;
    }
}
