//! Headless verification for egui harness apps (Tasks 4.5 and 7.4).
//! Uses only the public StriMD API — no app-specific table workarounds.

#![allow(dead_code)]

use strimd::{
    BlockContent, BlockKind, Document, ParseBackend, ParseProfile, StreamDocument, StreamOptions,
    StreamPatch,
};

pub type CheckResult = Result<(), String>;

/// Task 4.5 — static GFM tables come from shared `Document` / `RenderBlock` path.
pub fn check_static_table_path() -> CheckResult {
    let source = include_str!("../../tests/fixtures/gfm_table.md");
    let doc = Document::parse(source, ParseProfile::GitHubPreview)
        .map_err(|e| format!("parse gfm_table: {e}"))?;

    if doc.parse_backend() != ParseBackend::Pulldown {
        return Err(format!(
            "expected Pulldown backend, got {:?}",
            doc.parse_backend()
        ));
    }
    let table_blocks: Vec<_> = doc
        .blocks()
        .iter()
        .filter(|b| b.kind == BlockKind::Table)
        .collect();
    if table_blocks.is_empty() {
        return Err("expected at least one BlockKind::Table block".into());
    }
    if !table_blocks
        .iter()
        .all(|b| matches!(b.content, BlockContent::Markdown(_)))
    {
        return Err("table blocks must use BlockContent::Markdown (shared pulldown path)".into());
    }

    let html = doc.to_html().map_err(|e| format!("to_html: {e}"))?;
    if !html.contains("<table") {
        return Err(format!("HTML export missing <table>, got: {html}"));
    }

    Ok(())
}

/// Task 4.5 — streamed tables use the same block model; chunking is invariant.
pub fn check_stream_table_path() -> CheckResult {
    let source = include_str!("../../tests/fixtures/stream_table.md");

    let whole_count = {
        let mut doc = StreamDocument::new(StreamOptions::chat());
        doc.append(source);
        doc.blocks().count()
    };

    let mut chunked_doc = StreamDocument::new(StreamOptions::chat());
    let mut append_patches = 0usize;
    for chunk in source.as_bytes().chunks(4) {
        let update = chunked_doc.append(std::str::from_utf8(chunk).unwrap_or(""));
        if matches!(update.patch, StreamPatch::AppendCommitted { .. }) {
            append_patches += 1;
        }
    }

    if chunked_doc.blocks().count() != whole_count {
        return Err(format!(
            "chunk invariance failed: whole={whole_count}, chunked={}",
            chunked_doc.blocks().count()
        ));
    }

    let has_table = chunked_doc.blocks().any(|b| b.kind == BlockKind::Table)
        || chunked_doc
            .pending()
            .is_some_and(|p| p.kind == BlockKind::Table);
    if !has_table {
        return Err("streamed fixture missing BlockKind::Table".into());
    }
    if append_patches == 0 {
        return Err("expected AppendCommitted patches while streaming table".into());
    }

    Ok(())
}

/// Task 7.4 — static preview + streaming share StriMD without duplicate parsers.
pub fn check_unified_pipeline() -> CheckResult {
    check_static_table_path()?;
    check_stream_table_path()?;

    let preview = include_str!("../assets/TEST.md");
    let doc = Document::parse(preview, ParseProfile::GitHubPreview)
        .map_err(|e| format!("parse TEST.md: {e}"))?;
    if doc.parse_backend() != ParseBackend::Pulldown {
        return Err("TEST.md preview must use Pulldown".into());
    }

    let kinds: Vec<_> = doc.blocks().iter().map(|b| b.kind).collect();
    let html = doc.to_html().map_err(|e| format!("TEST.md to_html: {e}"))?;
    if html.is_empty() {
        return Err("TEST.md HTML export empty".into());
    }
    if !kinds.contains(&BlockKind::Heading) && !html.contains("<h") {
        return Err("TEST.md missing heading blocks or heading HTML".into());
    }

    // Stream mixed fixture: table + paragraph + raw HTML path
    let mixed = concat!(
        include_str!("../../tests/fixtures/stream_table.md"),
        "\n",
        include_str!("../../tests/fixtures/raw_details.md"),
    );
    let mut stream = StreamDocument::new(StreamOptions::chat());
    for chunk in mixed.as_bytes().chunks(6) {
        stream.append(std::str::from_utf8(chunk).unwrap_or(""));
    }
    if stream.blocks().count() < 2 {
        return Err("mixed stream expected multiple committed blocks".into());
    }
    let has_html = stream
        .blocks()
        .any(|b| matches!(b.content, BlockContent::Html(_)))
        || stream
            .pending()
            .is_some_and(|p| matches!(p.content, BlockContent::Html(_)));
    if !has_html {
        return Err("mixed stream missing HtmlFragment block (raw HTML path)".into());
    }

    Ok(())
}

pub fn run_all_checks() -> CheckResult {
    check_static_table_path()?;
    check_stream_table_path()?;
    check_unified_pipeline()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_table_check_passes() {
        check_static_table_path().expect("static table");
    }

    #[test]
    fn stream_table_check_passes() {
        check_stream_table_path().expect("stream table");
    }

    #[test]
    fn unified_pipeline_check_passes() {
        check_unified_pipeline().expect("unified pipeline");
    }
}
