//! GFM spec fixtures (docs/spec.txt extensions).

#![cfg(feature = "static")]

use strimd::{Document, ParseProfile};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}"))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

#[test]
fn gfm_tagfilter_example_657() {
    let doc = Document::parse(&fixture("gfm_tagfilter.md"), ParseProfile::GitHubPreview).expect("parse");
    let html = doc.to_html().expect("html");
    assert!(html.contains("&lt;title>"), "tagfilter title: {html}");
    assert!(html.contains("&lt;style>"), "tagfilter style: {html}");
    assert!(html.contains("&lt;xmp>"), "tagfilter xmp: {html}");
    assert!(html.contains("&lt;XMP>"), "tagfilter XMP: {html}");
    assert!(html.contains("<strong>"), "strong preserved: {html}");
}

#[test]
fn gfm_extended_www_autolink() {
    let doc = Document::parse("www.commonmark.org\n", ParseProfile::GitHubPreview).expect("parse");
    let html = doc.to_html().expect("html");
    assert!(
        html.contains("http://www.commonmark.org"),
        "expected www autolink href, got: {html}"
    );
}
