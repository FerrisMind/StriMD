//! Headless static preview: parse TEST.md and export HTML (Task 7.1).
//!
//! ```bash
//! cargo run --example static_export --no-default-features --features no_iced,static
//! ```

use strimd::{Document, ParseBackend, ParseProfile};

fn main() {
    let source = include_str!("assets/TEST.md");
    let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse TEST.md");

    eprintln!(
        "blocks: {}, backend: {:?}",
        doc.blocks().len(),
        doc.parse_backend()
    );
    assert_eq!(doc.parse_backend(), ParseBackend::Pulldown);

    let html = doc.to_html().expect("export html");
    assert!(!html.is_empty(), "expected non-empty HTML export");
    assert!(html.contains("<h1") || html.contains("Heading"), "missing heading markup");

    // Downstream consumers typically write or snapshot this output.
    print!("{html}");
}
