//! HtmlFragment TreeSink vs RcDom converter parity (Task 6.4).

#![cfg(all(feature = "static", feature = "_rcdom_compat"))]

use html5ever::{ParseOpts, QualName, local_name, ns, tendril::TendrilSink};
use markup5ever_rcdom::RcDom;
use strimd::HtmlFragment;
use strimd::html::fragment::HtmlNode;
use strimd::html::rcdom_compat;

const FIXTURES: &[&str] = &[
    "<details><summary>Title</summary><p>Body</p></details>",
    r#"<img src="x.png" alt="icon">"#,
    "<picture><source srcset=\"a.webp\"><img src=\"a.png\"></picture>",
    "<table><tr><td>x</td></tr></table>",
    "<p>open <span>inner",
];

fn parse_treesink(html: &str) -> HtmlFragment {
    strimd::html::treesink::parse_html_fragment(html).expect("treesink parse")
}

fn parse_rcdom(html: &str) -> HtmlFragment {
    let dom = html5ever::parse_fragment(
        RcDom::default(),
        ParseOpts::default(),
        QualName::new(None, ns!(html), local_name!("div")),
        vec![],
        true,
    )
    .one(html);
    rcdom_compat::from_rcdom(&dom)
}

fn root_tag(fragment: &HtmlFragment) -> Option<&str> {
    let root = *fragment.roots().first()?;
    match fragment.node(root)? {
        HtmlNode::Element { tag, .. } => Some(tag.as_str()),
        _ => None,
    }
}

#[test]
fn treesink_matches_rcdom_for_supported_fixtures() {
    for html in FIXTURES {
        let direct = parse_treesink(html);
        let converted = parse_rcdom(html);
        assert_eq!(
            direct.roots().len(),
            converted.roots().len(),
            "root count for {html:?}"
        );
        if let (Some(a), Some(b)) = (root_tag(&direct), root_tag(&converted)) {
            assert_eq!(a, b, "root tag for {html:?}");
        }
    }
}
