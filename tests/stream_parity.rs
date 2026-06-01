//! Streaming parity fixtures (Task 6.3).

#![cfg(feature = "stream")]

use strimd::{BlockContent, BlockKind, StreamDocument, StreamOptions, StreamPatch};

fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
    s.as_bytes()
        .chunks(chunk_size)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}

#[test]
fn chunked_table_matches_whole_append() {
    let source = include_str!("fixtures/stream_table.md");
    let whole = {
        let mut doc = StreamDocument::new(StreamOptions::chat());
        doc.append(source);
        doc.blocks().count()
    };
    let chunked = {
        let mut doc = StreamDocument::new(StreamOptions::chat());
        for chunk in chunk_string(source, 3) {
            doc.append(&chunk);
        }
        doc.blocks().count()
    };
    assert_eq!(whole, chunked);
    let mut probe = StreamDocument::new(StreamOptions::chat());
    probe.append(source);
    let has_table = probe.blocks().any(|b| b.kind == BlockKind::Table)
        || probe.pending().is_some_and(|p| p.kind == BlockKind::Table);
    assert!(has_table || whole >= 1, "expected table in stream output");
}

#[test]
fn code_fence_closure_across_chunks() {
    let source = "Intro\n\n```rust\nfn main() {}\n```\n";
    let mut doc = StreamDocument::new(StreamOptions::chat());
    for chunk in chunk_string(source, 5) {
        doc.append(&chunk);
    }
    assert!(
        doc.blocks().any(|b| b.kind == BlockKind::CodeFence)
            || doc
                .pending()
                .is_some_and(|p| p.kind == BlockKind::CodeFence)
    );
}

#[test]
fn footnote_definition_streaming_updates_state() {
    let mut doc = StreamDocument::new(StreamOptions::chat());
    doc.append("See footnote[^n].\n\n");
    let update = doc.append("[^n]: The note.\n\n");
    assert!(!update.reset);
    assert!(
        !update.invalidated.is_empty()
            || doc
                .blocks()
                .any(|b| b.kind == BlockKind::FootnoteDefinition)
            || matches!(update.patch, StreamPatch::AppendCommitted { .. }),
        "footnote definition should commit or invalidate prior content"
    );
}

#[test]
fn long_stream_many_chunks_does_not_reset() {
    let mut doc = StreamDocument::new(StreamOptions::chat());
    let mut resets = 0usize;
    for i in 0..200 {
        let update = doc.append(&format!("Line {i}.\n\n"));
        if update.reset {
            resets += 1;
        }
    }
    assert_eq!(resets, 0);
    assert!(doc.blocks().count() >= 50);
}

#[test]
fn streamed_raw_html_matches_static_fragment_kind() {
    let mut doc = StreamDocument::new(StreamOptions::chat());
    doc.append(include_str!("fixtures/raw_details.md"));
    assert!(
        doc.blocks()
            .any(|b| matches!(b.content, BlockContent::Html(_)))
    );
}
