//! Static parse and HTML export parity fixtures (Task 6.2).

#![cfg(feature = "static")]

use frostmark::{BlockContent, BlockKind, Document, ParseProfile};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}"))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

fn normalize_html(html: &str) -> String {
    html.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn test_md_fixture_parses_multiple_block_kinds() {
    let source = include_str!("../examples/assets/TEST.md");
    let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
    let kinds: Vec<_> = doc.blocks().iter().map(|b| b.kind).collect();
    assert!(kinds.contains(&BlockKind::Heading));
    assert!(kinds.iter().any(|k| matches!(k, BlockKind::HtmlBlock | BlockKind::Paragraph)));
}

#[test]
fn gfm_table_exports_table_markup() {
    let doc = Document::parse(&fixture("gfm_table.md"), ParseProfile::GitHubPreview).expect("parse");
    assert!(doc.blocks().iter().any(|b| b.kind == BlockKind::Table));
    let html = doc.to_html().expect("html");
    let norm = normalize_html(&html);
    assert!(norm.contains("<table") || norm.contains("<tbody"), "html: {norm}");
}

#[test]
fn gfm_tasks_export_checkbox_markup() {
    let doc = Document::parse(&fixture("gfm_tasks.md"), ParseProfile::GitHubPreview).expect("parse");
    let html = doc.to_html().expect("html");
    assert!(html.contains("checkbox") || html.contains("task-list"));
}

#[test]
fn raw_details_routes_to_html_fragment() {
    let doc = Document::parse(&fixture("raw_details.md"), ParseProfile::GitHubPreview).expect("parse");
    assert!(doc.blocks().iter().any(|b| matches!(b.content, BlockContent::Html(_))));
    let html = doc.to_html().expect("html");
    assert!(html.contains("details"));
    assert!(
        html.contains("summary") || html.contains("Summary"),
        "expected details export, got: {html}"
    );
}

#[cfg(feature = "_legacy_comrak")]
#[test]
fn wikilink_fixture_reports_shadow_mismatch_without_fallback() {
    let doc = Document::parse(&fixture("gfm_wikilink.md"), ParseProfile::GitHubPreview).expect("parse");
    assert!(doc.shadow_mismatch());
    assert!(!doc.legacy_fallback_used());
    assert_eq!(
        doc.parse_backend(),
        frostmark::ParseBackend::Pulldown
    );
}
