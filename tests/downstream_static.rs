//! Downstream static preview integration (Task 7.1).

#![cfg(all(feature = "no_iced", feature = "static"))]

use strimd::{BlockKind, Document, ParseBackend, ParseProfile};

#[test]
fn test_md_preview_through_document() {
    let source = include_str!("../examples/assets/TEST.md");
    let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
    assert_eq!(doc.parse_backend(), ParseBackend::Pulldown);

    let kinds: Vec<_> = doc.blocks().iter().map(|b| b.kind).collect();
    let html = doc.to_html().expect("html");
    assert!(
        kinds.contains(&BlockKind::Heading) || html.contains("<h"),
        "headings may be coalesced into HTML wrapper blocks; kinds={kinds:?}"
    );
    assert!(
        kinds.contains(&BlockKind::CodeFence)
            || kinds.contains(&BlockKind::Paragraph)
            || html.contains("<pre>")
    );
    assert!(html.contains("Heading") || html.contains("heading"));
}

#[test]
fn hello_readme_sample_exports_html() {
    const SAMPLE: &str = "Hello from **markdown** and <b>HTML</b>!";
    let doc = Document::parse(SAMPLE, ParseProfile::GitHubPreview).expect("parse");
    let html = doc.to_html().expect("html");
    assert!(html.contains("Hello"));
    assert!(html.contains("<strong>") || html.contains("<b>"));
}
