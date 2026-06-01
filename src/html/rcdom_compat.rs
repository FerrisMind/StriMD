//! `RcDom` ↔ [`HtmlFragment`] bridge for migration parity tests only.
//! Production rendering uses TreeSink → `HtmlFragment` directly.

use std::sync::Arc;

use html5ever::{ParseOpts, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::html::fragment::{HtmlAttr, HtmlFragment, HtmlNode, HtmlTag, NodeId};

/// Parse an HTML document string into [`RcDom`] for the iced backend.
#[must_use]
pub fn html_to_rcdom(input: &str) -> RcDom {
    html5ever::parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut input.as_bytes())
        .expect("reading from UTF-8 bytes cannot fail")
}

/// Serialize a fragment to HTML and parse it into [`RcDom`].
#[must_use]
pub fn fragment_to_rcdom(fragment: &HtmlFragment) -> RcDom {
    let mut html = String::new();
    for root in fragment.roots() {
        write_fragment_node(&mut html, fragment, *root);
    }
    if html.is_empty() {
        html_to_rcdom("<div></div>")
    } else {
        html_to_rcdom(&html)
    }
}

fn write_fragment_node(out: &mut String, fragment: &HtmlFragment, id: NodeId) {
    let Some(node) = fragment.node(id) else {
        return;
    };
    match node {
        HtmlNode::Text(text) => out.push_str(text),
        HtmlNode::Comment(comment) => {
            out.push_str("<!--");
            out.push_str(comment);
            out.push_str("-->");
        }
        HtmlNode::Element {
            tag,
            attrs,
            children,
        } => {
            out.push('<');
            out.push_str(tag.as_str());
            for attr in attrs {
                out.push(' ');
                out.push_str(&attr.name);
                out.push_str("=\"");
                out.push_str(&attr.value);
                out.push('"');
            }
            out.push('>');
            for child in children {
                write_fragment_node(out, fragment, *child);
            }
            out.push_str("</");
            out.push_str(tag.as_str());
            out.push('>');
        }
    }
}

/// Convert a parsed [`RcDom`] tree into a backend-agnostic [`HtmlFragment`].
#[must_use]
pub fn from_rcdom(dom: &RcDom) -> HtmlFragment {
    let mut fragment = HtmlFragment::empty();
    for child in dom.document.children.borrow().iter() {
        if let Some(id) = convert_node(child, &mut fragment) {
            fragment.push_root(id);
        }
    }
    fragment.normalize_roots()
}

fn convert_node(node: &Handle, fragment: &mut HtmlFragment) -> Option<NodeId> {
    match &node.data {
        NodeData::Document => {
            let mut roots = Vec::new();
            for child in node.children.borrow().iter() {
                if let Some(id) = convert_node(child, fragment) {
                    roots.push(id);
                }
            }
            if roots.len() == 1 {
                return roots.into_iter().next();
            }
            let id = fragment.push_element(HtmlTag::new("div"), Vec::new(), roots);
            Some(id)
        }
        NodeData::Text { contents } => {
            let text = contents.borrow();
            if text.is_empty() {
                None
            } else {
                Some(fragment.push_text(Arc::from(text.as_ref())))
            }
        }
        NodeData::Comment { contents } => Some(fragment.push_comment(Arc::from(contents.as_ref()))),
        NodeData::Element { name, attrs, .. } => {
            let tag = HtmlTag::new(name.local.to_string());
            let html_attrs = attrs
                .borrow()
                .iter()
                .map(|attr| HtmlAttr {
                    name: Arc::from(attr.name.local.to_string()),
                    value: Arc::from(attr.value.to_string()),
                })
                .collect();
            let children: Vec<NodeId> = node
                .children
                .borrow()
                .iter()
                .filter_map(|child| convert_node(child, fragment))
                .collect();
            Some(fragment.push_element(tag, html_attrs, children))
        }
        NodeData::Doctype { .. } | NodeData::ProcessingInstruction { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use html5ever::ParseOpts;

    fn parse_html(input: &str) -> RcDom {
        use html5ever::{QualName, local_name, ns, tendril::TendrilSink};
        html5ever::parse_fragment(
            RcDom::default(),
            ParseOpts::default(),
            QualName::new(None, ns!(html), local_name!("div")),
            vec![],
            true,
        )
        .one(input)
    }

    fn tag_name(fragment: &HtmlFragment, id: NodeId) -> &str {
        match fragment.node(id).expect("node") {
            HtmlNode::Element { tag, .. } => tag.as_str(),
            other => panic!("expected element, got {other:?}"),
        }
    }

    #[test]
    fn preserves_details_summary_hierarchy() {
        let dom = parse_html("<details><summary>Title</summary><p>Body</p></details>");
        let fragment = from_rcdom(&dom);
        assert_eq!(fragment.roots().len(), 1);
        let details = fragment.roots()[0];
        assert_eq!(tag_name(&fragment, details), "details");
        let HtmlNode::Element { children, .. } = fragment.node(details).expect("details") else {
            panic!("not element");
        };
        assert_eq!(children.len(), 2);
        assert_eq!(tag_name(&fragment, children[0]), "summary");
        assert_eq!(tag_name(&fragment, children[1]), "p");
    }

    #[test]
    fn preserves_img_src_attribute() {
        let dom = parse_html(r#"<img src="x.png" alt="icon">"#);
        let fragment = from_rcdom(&dom);
        let root = fragment.roots()[0];
        let HtmlNode::Element { attrs, .. } = fragment.node(root).expect("img") else {
            panic!("not element");
        };
        assert!(
            attrs
                .iter()
                .any(|a| a.name.as_ref() == "src" && a.value.as_ref() == "x.png")
        );
    }

    #[test]
    fn preserves_picture_and_table_structure() {
        let dom = parse_html(
            "<picture><source srcset=\"a.webp\"><img src=\"a.png\"></picture><table><tr><td>x</td></tr></table>",
        );
        let fragment = from_rcdom(&dom);
        assert_eq!(fragment.roots().len(), 2);
        assert_eq!(tag_name(&fragment, fragment.roots()[0]), "picture");
        assert_eq!(tag_name(&fragment, fragment.roots()[1]), "table");
    }
}
