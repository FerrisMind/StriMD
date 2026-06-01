//! Headless static preview: parse TEST.md and export HTML (Task 7.1).
//!
//! ```bash
//! cargo run --example static_export --no-default-features --features no_iced,static
//! cargo run --example static_export --no-default-features --features no_iced,static -- examples/assets/MPF.md
//! ```

use std::path::{Path, PathBuf};

use strimd::{Document, ParseBackend, ParseProfile};

fn main() {
    let input = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/assets/TEST.md"));
    let source = std::fs::read_to_string(&input)
        .unwrap_or_else(|err| panic!("read {}: {err}", input.display()));
    let doc = Document::parse(&source, ParseProfile::GitHubPreview)
        .unwrap_or_else(|err| panic!("parse {}: {err}", input.display()));

    eprintln!(
        "input: {}, blocks: {}, backend: {:?}",
        display_path(&input),
        doc.blocks().len(),
        doc.parse_backend()
    );
    assert_eq!(doc.parse_backend(), ParseBackend::Pulldown);

    let html = doc.to_html().expect("export html");
    assert!(!html.is_empty(), "expected non-empty HTML export");
    assert!(
        html.contains("<h1") || html.contains("Heading"),
        "missing heading markup"
    );

    // Downstream consumers typically write or snapshot this output.
    print!("{html}");
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
