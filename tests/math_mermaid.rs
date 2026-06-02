//! RaTeX math and mermaid-rs-renderer integration tests.

#![cfg(all(feature = "_iced_backend", feature = "math", feature = "mermaid"))]

use strimd::render::{latex_to_svg, mermaid_to_svg};
use strimd::{BlockContent, BlockKind, Document, MarkState, ParseProfile};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/{name}"))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

#[test]
fn ratex_renders_svg() {
    let art = latex_to_svg(r"x^2 + y^2", false).expect("latex svg");
    let svg = std::str::from_utf8(&art.bytes).expect("utf8");
    assert!(svg.contains("<svg"));
}

#[test]
fn mermaid_renders_svg() {
    let src = "flowchart LR\n  A-->B\n";
    let art = mermaid_to_svg(src).expect("mermaid svg");
    assert!(std::str::from_utf8(&art.bytes).unwrap().contains("<svg"));
}

#[test]
fn document_math_display_block() {
    let doc =
        Document::parse(&fixture("math_display.md"), ParseProfile::GitHubPreview).expect("parse");
    assert!(
        doc.blocks()
            .iter()
            .any(|b| matches!(b.content, BlockContent::Math { .. })),
        "expected BlockContent::Math for display math paragraph"
    );
}

#[test]
fn document_mermaid_fence_block() {
    let doc =
        Document::parse(&fixture("mermaid_flow.md"), ParseProfile::GitHubPreview).expect("parse");
    let block = doc
        .blocks()
        .iter()
        .find(|b| b.kind == BlockKind::CodeFence)
        .expect("mermaid fence");
    assert!(matches!(
        block.content,
        BlockContent::Mermaid { complete: true, .. }
    ));
}

#[test]
fn mark_state_from_combined_fixtures() {
    let doc = Document::parse(
        &format!(
            "{}\n\n{}\n\n{}",
            fixture("math_inline.md"),
            fixture("math_display.md"),
            fixture("mermaid_flow.md")
        ),
        ParseProfile::GitHubPreview,
    )
    .expect("parse");
    let _state = MarkState::from_document(&doc);
}

#[test]
fn mark_state_from_math_inline_fixture() {
    let doc =
        Document::parse(&fixture("math_inline.md"), ParseProfile::GitHubPreview).expect("parse");
    let _state = MarkState::from_document(&doc);
}
