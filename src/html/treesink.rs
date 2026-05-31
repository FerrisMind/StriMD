//! Direct `html5ever::tree_builder::TreeSink` → [`HtmlFragment`] parser.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use html5ever::tendril::StrTendril;
use html5ever::tree_builder::{
    ElementFlags, NodeOrText, QuirksMode, TreeSink,
};
use html5ever::{Attribute, ExpandedName, QualName};

use crate::core::error::HtmlFragmentError;
use crate::html::fragment::{HtmlAttr, HtmlFragment, HtmlTag, NodeId};

type WeakHandle = Weak<SinkNode>;

#[derive(Clone)]
pub(crate) struct Handle(Rc<SinkNode>);

enum SinkData {
    Document,
    Element {
        name: QualName,
        attrs: RefCell<Vec<HtmlAttr>>,
        template_contents: RefCell<Option<Handle>>,
        mathml_annotation_xml_integration_point: bool,
    },
    Text(RefCell<StrTendril>),
    Comment(StrTendril),
    Doctype {
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    },
    ProcessingInstruction {
        target: StrTendril,
        contents: StrTendril,
    },
}

struct SinkNode {
    parent: Cell<Option<WeakHandle>>,
    children: RefCell<Vec<Handle>>,
    data: SinkData,
}

pub(crate) struct FragmentSink {
    document: Handle,
    errors: RefCell<Vec<String>>,
    quirks_mode: Cell<QuirksMode>,
}

impl FragmentSink {
    fn new() -> Self {
        Self {
            document: Handle(Rc::new(SinkNode {
                parent: Cell::new(None),
                children: RefCell::new(Vec::new()),
                data: SinkData::Document,
            })),
            errors: RefCell::new(Vec::new()),
            quirks_mode: Cell::new(QuirksMode::NoQuirks),
        }
    }

    fn node(name: SinkData) -> Handle {
        Handle(Rc::new(SinkNode {
            parent: Cell::new(None),
            children: RefCell::new(Vec::new()),
            data: name,
        }))
    }

    fn append(parent: &Handle, child: Handle) {
        child.0.parent.set(Some(Rc::downgrade(&parent.0)));
        parent.0.children.borrow_mut().push(child);
    }

    fn append_to_existing_text(node: &Handle, text: &StrTendril) -> bool {
        if let SinkData::Text(contents) = &node.0.data {
            contents.borrow_mut().push_tendril(text);
            true
        } else {
            false
        }
    }

    fn parent_and_index(target: &Handle) -> Option<(Handle, usize)> {
        let parent = target.0.parent.take()?.upgrade()?;
        target.0.parent.set(Some(Rc::downgrade(&parent)));
        let index = parent
            .children
            .borrow()
            .iter()
            .position(|child| Rc::ptr_eq(&child.0, &target.0))?;
        Some((Handle(parent), index))
    }

    fn remove_from_parent(target: &Handle) {
        if let Some((parent, index)) = Self::parent_and_index(target) {
            parent.0.children.borrow_mut().remove(index);
            target.0.parent.set(None);
        }
    }

    fn into_fragment(self) -> HtmlFragment {
        let mut fragment = HtmlFragment::empty();
        for child in self.document.0.children.borrow().iter() {
            if let Some(id) = convert_handle(child, &mut fragment) {
                fragment.push_root(id);
            }
        }
        fragment.normalize_roots()
    }
}

fn convert_handle(handle: &Handle, fragment: &mut HtmlFragment) -> Option<NodeId> {
    match &handle.0.data {
        SinkData::Document => {
            let children: Vec<NodeId> = handle
                .0
                .children
                .borrow()
                .iter()
                .filter_map(|child| convert_handle(child, fragment))
                .collect();
            if children.len() == 1 {
                children.into_iter().next()
            } else if children.is_empty() {
                None
            } else {
                Some(fragment.push_element(HtmlTag::new("div"), Vec::new(), children))
            }
        }
        SinkData::Text(contents) => {
            let text = contents.borrow();
            if text.is_empty() {
                None
            } else {
                Some(fragment.push_text(std::sync::Arc::from(text.as_ref())))
            }
        }
        SinkData::Comment(text) => Some(fragment.push_comment(std::sync::Arc::from(text.as_ref()))),
        SinkData::Element { name, attrs, .. } => {
            let converted: Vec<NodeId> = handle
                .0
                .children
                .borrow()
                .iter()
                .filter_map(|child| convert_handle(child, fragment))
                .collect();
            Some(fragment.push_element(
                HtmlTag::new(name.local.to_string()),
                attrs.borrow().clone(),
                converted,
            ))
        }
        SinkData::Doctype { .. } | SinkData::ProcessingInstruction { .. } => None,
    }
}

impl TreeSink for FragmentSink {
    type Handle = Handle;
    type Output = HtmlFragment;
    type ElemName<'a> = ExpandedName<'a>;

    fn finish(self) -> HtmlFragment {
        self.into_fragment()
    }

    fn parse_error(&self, msg: std::borrow::Cow<'static, str>) {
        self.errors.borrow_mut().push(msg.into_owned());
    }

    fn get_document(&self) -> Handle {
        self.document.clone()
    }

    fn elem_name<'a>(&'a self, target: &'a Handle) -> ExpandedName<'a> {
        match &target.0.data {
            SinkData::Element { name, .. } => name.expanded(),
            _ => panic!("elem_name on non-element"),
        }
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        flags: ElementFlags,
    ) -> Handle {
        let html_attrs = attrs
            .into_iter()
            .map(|attr| HtmlAttr {
                name: std::sync::Arc::from(attr.name.local.to_string()),
                value: std::sync::Arc::from(attr.value.to_string()),
            })
            .collect();
        Self::node(SinkData::Element {
            name,
            attrs: RefCell::new(html_attrs),
            template_contents: RefCell::new(if flags.template {
                Some(Self::node(SinkData::Document))
            } else {
                None
            }),
            mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
        })
    }

    fn create_comment(&self, text: StrTendril) -> Handle {
        Self::node(SinkData::Comment(text))
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Handle {
        Self::node(SinkData::ProcessingInstruction {
            target,
            contents: data,
        })
    }

    fn append(&self, parent: &Handle, child: NodeOrText<Handle>) {
        if let NodeOrText::AppendText(text) = &child {
            if let Some(last) = parent.0.children.borrow().last() {
                if Self::append_to_existing_text(last, text) {
                    return;
                }
            }
        }
        Self::append(
            parent,
            match child {
                NodeOrText::AppendText(text) => Self::node(SinkData::Text(RefCell::new(text))),
                NodeOrText::AppendNode(node) => node,
            },
        );
    }

    fn append_based_on_parent_node(
        &self,
        element: &Handle,
        prev_element: &Handle,
        child: NodeOrText<Handle>,
    ) {
        let parent = element.0.parent.take();
        let has_parent = parent.is_some();
        element.0.parent.set(parent);

        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        Self::append(
            &self.document,
            Self::node(SinkData::Doctype {
                name,
                public_id,
                system_id,
            }),
        );
    }

    fn get_template_contents(&self, target: &Handle) -> Handle {
        match &target.0.data {
            SinkData::Element {
                template_contents, ..
            } => template_contents
                .borrow()
                .as_ref()
                .expect("not a template element")
                .clone(),
            _ => panic!("not a template element"),
        }
    }

    fn same_node(&self, x: &Handle, y: &Handle) -> bool {
        Rc::ptr_eq(&x.0, &y.0)
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks_mode.set(mode);
    }

    fn append_before_sibling(&self, sibling: &Handle, child: NodeOrText<Handle>) {
        let Some((parent, index)) = FragmentSink::parent_and_index(sibling) else {
            return;
        };

        let child = match (child, index) {
            (NodeOrText::AppendText(text), 0) => Self::node(SinkData::Text(RefCell::new(text))),
            (NodeOrText::AppendText(text), i) => {
                let prev = parent.0.children.borrow()[i - 1].clone();
                if Self::append_to_existing_text(&prev, &text) {
                    return;
                }
                Self::node(SinkData::Text(RefCell::new(text)))
            }
            (NodeOrText::AppendNode(node), _) => node,
        };

        FragmentSink::remove_from_parent(&child);
        child.0.parent.set(Some(Rc::downgrade(&parent.0)));
        parent.0.children.borrow_mut().insert(index, child);
    }

    fn add_attrs_if_missing(&self, target: &Handle, attrs: Vec<Attribute>) {
        let SinkData::Element { attrs: existing, .. } = &target.0.data else {
            panic!("add_attrs_if_missing on non-element");
        };
        let mut existing = existing.borrow_mut();
        for attr in attrs {
            if !existing
                .iter()
                .any(|existing_attr| existing_attr.name.as_ref() == attr.name.local.as_ref())
            {
                existing.push(HtmlAttr {
                    name: std::sync::Arc::from(attr.name.local.to_string()),
                    value: std::sync::Arc::from(attr.value.to_string()),
                });
            }
        }
    }

    fn remove_from_parent(&self, target: &Handle) {
        FragmentSink::remove_from_parent(target);
    }

    fn reparent_children(&self, node: &Handle, new_parent: &Handle) {
        let children = node.0.children.borrow().clone();
        for child in children {
            FragmentSink::remove_from_parent(&child);
            FragmentSink::append(new_parent, child);
        }
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &Handle) -> bool {
        matches!(
            handle.0.data,
            SinkData::Element {
                mathml_annotation_xml_integration_point: true,
                ..
            }
        )
    }
}

/// Parse an HTML fragment string into an [`HtmlFragment`] using a custom TreeSink.
pub fn parse_html_fragment(html: &str) -> Result<HtmlFragment, HtmlFragmentError> {
    use html5ever::{local_name, ns, ParseOpts, QualName, tendril::TendrilSink};

    let sink = FragmentSink::new();
    let fragment = html5ever::parse_fragment(
        sink,
        ParseOpts::default(),
        QualName::new(None, ns!(html), local_name!("div")),
        vec![],
        true,
    )
    .one(html);
    Ok(fragment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::fragment::HtmlNode;

    fn tag(fragment: &HtmlFragment, id: NodeId) -> &str {
        match fragment.node(id).expect("node") {
            HtmlNode::Element { tag, .. } => tag.as_str(),
            other => panic!("expected element, got {other:?}"),
        }
    }

    #[test]
    fn treesink_matches_details_fixture() {
        let fragment =
            parse_html_fragment("<details><summary>x</summary></details>").expect("parse");
        assert_eq!(fragment.roots().len(), 1);
        assert_eq!(tag(&fragment, fragment.roots()[0]), "details");
    }

    #[test]
    fn treesink_recovers_malformed_html() {
        let fragment = parse_html_fragment("<p>open <span>inner").expect("parse");
        assert!(!fragment.roots().is_empty());
    }

    #[cfg(feature = "_rcdom_compat")]
    #[test]
    fn treesink_matches_rcdom_converter() {
        use crate::html::rcdom_compat;
        use html5ever::{local_name, ns, ParseOpts, QualName, tendril::TendrilSink};
        use markup5ever_rcdom::RcDom;

        let html = "<details><summary>Title</summary><img src=\"a.png\"></details>";
        let direct = parse_html_fragment(html).expect("treesink");
        let dom = html5ever::parse_fragment(
            RcDom::default(),
            ParseOpts::default(),
            QualName::new(None, ns!(html), local_name!("div")),
            vec![],
            true,
        )
        .one(html);
        let converted = rcdom_compat::from_rcdom(&dom);
        assert_eq!(direct.roots().len(), converted.roots().len());
        assert_eq!(
            tag(&direct, direct.roots()[0]),
            tag(&converted, converted.roots()[0])
        );
    }
}
