//! Styling example fixture: verifies the demo covers text + widget styling.

#![cfg(all(feature = "_iced_backend", not(feature = "no_iced")))]

use strimd::{BlockKind, Document, MarkState, ParseProfile};

const STYLING_TEXT: &str = include_str!("../examples/fixtures/styling.md");

/// Each entry is a substring the styling example must contain to document a feature.
const STYLING_DEMO_MARKERS: &[&str] = &[
    "Style::text_color",
    "Style::link_color",
    "Style::highlight_color",
    "Style::inline_code_color",
    "inline_code_background",
    "Style::code_block_background",
    ".paragraph_spacing(20.0)",
    ".style_link_button()",
];

#[test]
fn styling_fixture_documents_text_and_widget_styling() {
    for marker in STYLING_DEMO_MARKERS {
        assert!(
            STYLING_TEXT.contains(marker),
            "styling demo missing marker {marker:?}"
        );
    }
    assert!(STYLING_TEXT.contains("<mark>"));
    assert!(STYLING_TEXT.contains("https://example.com"));
    assert!(STYLING_TEXT.contains("[Block link"));
}

#[test]
fn styling_fixture_parses_blocks_for_all_demo_content() {
    let doc = Document::parse(STYLING_TEXT, ParseProfile::GitHubPreview).expect("parse");
    let kinds: Vec<_> = doc.blocks().iter().map(|b| b.kind).collect();
    assert!(
        kinds.contains(&BlockKind::Paragraph),
        "expected paragraph for body/link/mark/inline-code demo, got {kinds:?}"
    );
    assert!(
        kinds.contains(&BlockKind::CodeFence),
        "expected fenced code for code_block_background demo, got {kinds:?}"
    );
    assert!(
        kinds.iter().filter(|k| **k == BlockKind::Paragraph).count() >= 2,
        "expected multiple paragraphs (spacing demo), got {kinds:?}"
    );
}

#[test]
fn styling_fixture_builds_mark_state_like_example() {
    let _state = MarkState::with_html_and_markdown(STYLING_TEXT);
}
