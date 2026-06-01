//! egui UI helpers for StriMD harness examples.

#![allow(dead_code)]

use egui::{Color32, RichText, ScrollArea, Ui};
use strimd::{BlockKind, Document, RenderBlock, StreamDocument, StreamPatch, StreamUpdate};

pub fn status_banner(ui: &mut Ui, label: &str, ok: bool) {
    let (text, color) = if ok {
        ("PASS", Color32::from_rgb(40, 160, 80))
    } else {
        ("FAIL", Color32::from_rgb(200, 60, 60))
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(text).strong().color(color));
        ui.label(label);
    });
}

pub fn show_check_errors(ui: &mut Ui, errors: &[String]) {
    if errors.is_empty() {
        status_banner(ui, "All harness checks passed", true);
        return;
    }
    status_banner(ui, "Harness checks failed", false);
    for err in errors {
        ui.colored_label(Color32::LIGHT_RED, err);
    }
}

pub fn render_block_inspector(ui: &mut Ui, blocks: &[RenderBlock]) {
    ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        for block in blocks {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("{:?}", block.kind));
                    ui.label(format!("id={}", block.id.0));
                    ui.label(format!("{:?}", block.status));
                });
                ui.monospace(block.source.chars().take(120).collect::<String>());
            });
        }
    });
}

pub fn render_markdown_table_preview(ui: &mut Ui, source: &str) {
    let rows: Vec<Vec<&str>> = source
        .lines()
        .filter(|line| line.contains('|'))
        .map(|line| {
            line.trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect()
        })
        .filter(|cells: &Vec<&str>| {
            !cells.is_empty() && !cells.iter().all(|c| c.chars().all(|ch| ch == '-'))
        })
        .collect();

    if rows.is_empty() {
        ui.label("No table rows in block source");
        return;
    }

    egui::Grid::new("table_preview")
        .striped(true)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for row in &rows {
                for cell in row {
                    ui.label(*cell);
                }
                ui.end_row();
            }
        });
}

pub fn render_table_blocks(ui: &mut Ui, blocks: &[RenderBlock]) {
    let tables: Vec<_> = blocks
        .iter()
        .filter(|b| b.kind == BlockKind::Table)
        .collect();
    if tables.is_empty() {
        ui.label("No BlockKind::Table blocks yet");
        return;
    }
    for block in tables {
        ui.heading(format!("Table block #{}", block.id.0));
        render_markdown_table_preview(ui, &block.source);
    }
}

pub fn render_html_export(ui: &mut Ui, html: &str) {
    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        ui.monospace(html);
    });
}

pub fn render_stream_patch_log(ui: &mut Ui, updates: &[StreamUpdate]) {
    ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
        for (i, update) in updates.iter().enumerate() {
            ui.label(format!(
                "chunk {i}: patch={:?} reset={} invalidated={}",
                update.patch,
                update.reset,
                update.invalidated.len()
            ));
        }
    });
}

pub fn collect_blocks(doc: &StreamDocument) -> Vec<RenderBlock> {
    let mut blocks: Vec<RenderBlock> = doc.blocks().cloned().collect();
    if let Some(pending) = doc.pending() {
        blocks.push(pending.clone());
    }
    blocks
}

pub fn stream_fixture_by_words(text: &str, words_per_chunk: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![text.to_string()];
    }
    words
        .chunks(words_per_chunk.max(1))
        .map(|chunk| format!("{} ", chunk.join(" ")))
        .collect()
}

pub fn append_stream_chunks(
    source: &str,
    chunk_size: usize,
) -> (StreamDocument, Vec<StreamUpdate>) {
    let mut doc = StreamDocument::new(strimd::StreamOptions::chat());
    let mut updates = Vec::new();
    for chunk in source.as_bytes().chunks(chunk_size) {
        updates.push(doc.append(std::str::from_utf8(chunk).unwrap_or("")));
    }
    (doc, updates)
}

pub fn count_append_committed(updates: &[StreamUpdate]) -> usize {
    updates
        .iter()
        .filter(|u| matches!(u.patch, StreamPatch::AppendCommitted { .. }))
        .count()
}

pub fn render_document_summary(ui: &mut Ui, doc: &Document, html: &str) {
    ui.label(format!("blocks: {}", doc.blocks().len()));
    ui.label(format!("backend: {:?}", doc.parse_backend()));
    ui.label(format!("diagnostics: {}", doc.diagnostics()));
    render_table_blocks(ui, doc.blocks());
    ui.separator();
    ui.heading("HTML export");
    render_html_export(ui, html);
}
