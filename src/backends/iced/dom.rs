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
                | "center"
        )
    }

    #[must_use]
    pub(crate) fn has_task_checkbox_child(&self) -> bool {
        self.find_descendant(|node| {
            node.tag_name() == Some("input") && node.get_attr("type").as_deref() == Some("checkbox")
        })
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

    fn find_descendant(&self, mut pred: impl FnMut(DomRef<'a>) -> bool) -> bool {
        fn walk<'a>(
            node: DomRef<'a>,
            depth: usize,
            pred: &mut impl FnMut(DomRef<'a>) -> bool,
        ) -> bool {
            if depth > 8 {
                return false;
            }
            if pred(node) {
                return true;
            }
            for child in node.children() {
                if walk(child, depth + 1, pred) {
                    return true;
                }
            }
            false
        }
        walk(*self, 0, &mut pred)
    }

    /// Paragraph that only contains badge image(s), optionally wrapped in a link.
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
        let mut has_img = false;
        for child in meaningful {
            match child.tag_name() {
                Some("img") => has_img = true,
                Some("a") => {
                    let kids: Vec<_> = child.children().into_iter().filter(|c| !c.is_useless()).collect();
                    if kids.len() != 1 || kids[0].tag_name() != Some("img") {
                        return false;
                    }
                    has_img = true;
                }
                _ => return false,
            }
        }
        has_img
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
