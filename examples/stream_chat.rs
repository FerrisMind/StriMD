//! Headless LLM streaming simulation (Task 7.2).
//!
//! ```bash
//! cargo run --example stream_chat --no-default-features --features no_iced,stream
//! ```

use strimd::{BlockKind, StreamDocument, StreamOptions, StreamPatch};

fn main() {
    let response = include_str!("../tests/fixtures/stream_table.md");
    let mut doc = StreamDocument::new(StreamOptions::chat());
    let mut append_patches = 0usize;
    let mut replace_pending = 0usize;

    for (index, chunk) in chunk_by_words(response, 3).into_iter().enumerate() {
        let update = doc.append(&chunk);
        if update.reset {
            eprintln!("chunk {index}: reset");
        }
        match update.patch {
            StreamPatch::AppendCommitted { .. } => append_patches += 1,
            StreamPatch::ReplacePending => replace_pending += 1,
            StreamPatch::Noop => {}
            other => eprintln!("chunk {index}: {other:?}"),
        }
    }

    let committed = doc.blocks().count();
    let has_table = doc.blocks().any(|b| b.kind == BlockKind::Table)
        || doc.pending().is_some_and(|p| p.kind == BlockKind::Table);

    eprintln!(
        "committed blocks: {committed}, append patches: {append_patches}, \
         pending replacements: {replace_pending}, has_table: {has_table}"
    );
    assert!(committed >= 1, "expected at least one committed block");
    assert!(append_patches >= 1, "expected append patches during streaming");
}

fn chunk_by_words(text: &str, words_per_chunk: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![text.to_string()];
    }
    words
        .chunks(words_per_chunk.max(1))
        .map(|chunk| format!("{} ", chunk.join(" ")))
        .collect()
}
