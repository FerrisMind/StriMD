//! Iced backend block-cache regression tests (Task 6.5).

#![cfg(all(feature = "_iced_backend", not(feature = "no_iced")))]

use strimd::{BlockKind, Document, MarkState, ParseProfile};

#[test]
fn mark_state_from_document_builds_block_cache() {
    let doc = Document::parse(
        "# Title\n\n<details><summary>x</summary></details>",
        ParseProfile::GitHubPreview,
    )
    .expect("parse");
    let _state = MarkState::from_document(&doc);
}

#[test]
fn document_with_table_and_code_produces_blocks() {
    let source = "| A | B |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n";
    let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
    assert!(doc.blocks().iter().any(|b| b.kind == BlockKind::Table));
    assert!(doc.blocks().iter().any(|b| b.kind == BlockKind::CodeFence));
    let _state = MarkState::from_document(&doc);
}

#[cfg(feature = "stream")]
#[test]
fn streamed_gfm_table_syncs_to_mark_state() {
    use strimd::{StreamDocument, StreamOptions};

    let source = include_str!("fixtures/stream_table.md");
    let mut stream = StreamDocument::new(StreamOptions::chat());
    for chunk in source.as_bytes().chunks(5) {
        stream.append(std::str::from_utf8(chunk).unwrap_or(""));
    }
    assert!(
        stream.blocks().any(|b| b.kind == BlockKind::Table)
            || stream.pending().is_some_and(|p| p.kind == BlockKind::Table)
    );

    let mut state = MarkState::from_blocks(&[]);
    state.sync_from_stream(&stream);
}
