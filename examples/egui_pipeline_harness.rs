//! Task 7.4 — egui harness: unified frostmark pipeline (static + stream, no special cases).
//!
//! ```bash
//! cargo run --example egui_pipeline_harness --no-default-features --features no_iced,static,stream -- --check
//! cargo run --example egui_pipeline_harness --no-default-features --features no_iced,static,stream
//! ```

#![cfg(all(feature = "no_iced", feature = "static", feature = "stream"))]

#[path = "shared/harness_checks.rs"]
mod harness_checks;

#[path = "shared/egui_ui.rs"]
mod egui_ui;

use eframe::egui;
use frostmark::{Document, ParseProfile, StreamDocument};

fn main() -> eframe::Result<()> {
    if std::env::args().any(|a| a == "--check") {
        match harness_checks::check_unified_pipeline() {
            Ok(()) => {
                eprintln!("egui_pipeline_harness: unified pipeline checks passed");
                return Ok(());
            }
            Err(e) => {
                eprintln!("egui_pipeline_harness: FAIL — {e}");
                std::process::exit(1);
            }
        }
    }

    let preview_source = include_str!("assets/TEST.md");
    let preview_doc =
        Document::parse(preview_source, ParseProfile::GitHubPreview).expect("TEST.md parse");
    let preview_html = preview_doc.to_html().expect("TEST.md html");

    let mixed = concat!(
        include_str!("../tests/fixtures/stream_table.md"),
        "\n",
        include_str!("../tests/fixtures/raw_details.md"),
    );
    let (stream_doc, stream_updates) = egui_ui::append_stream_chunks(&mixed, 6);
    let stream_blocks = egui_ui::collect_blocks(&stream_doc);

    let pipeline_err = harness_checks::check_unified_pipeline().err();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 760.0])
            .with_title("frostmark egui pipeline harness (Task 7.4)"),
        ..Default::default()
    };

    eframe::run_native(
        "frostmark egui pipeline harness",
        native_options,
        Box::new(|_ctx| {
            Ok(Box::new(PipelineHarnessApp {
                preview_doc,
                preview_html,
                stream_doc,
                stream_updates,
                stream_blocks,
                pipeline_err,
            }))
        }),
    )
}

struct PipelineHarnessApp {
    preview_doc: Document,
    preview_html: String,
    stream_doc: StreamDocument,
    stream_updates: Vec<frostmark::StreamUpdate>,
    stream_blocks: Vec<frostmark::RenderBlock>,
    pipeline_err: Option<String>,
}

impl eframe::App for PipelineHarnessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Task 7.4 — unified frostmark pipeline");
            ui.label(
                "Static preview + LLM streaming via Document / StreamDocument only. \
                 No comrak, no chat_table, no app preprocess hooks.",
            );

            egui_ui::show_check_errors(
                ui,
                &self.pipeline_err.clone().into_iter().collect::<Vec<_>>(),
            );

            ui.separator();
            ui.heading("Static preview (TEST.md)");
            egui_ui::render_document_summary(ui, &self.preview_doc, &self.preview_html);
            egui_ui::render_block_inspector(ui, self.preview_doc.blocks());

            ui.separator();
            ui.heading("Streamed mixed content (table + raw HTML)");
            ui.label(format!(
                "committed: {}, patches: {}",
                self.stream_doc.blocks().count(),
                self.stream_updates.len()
            ));
            egui_ui::render_stream_patch_log(ui, &self.stream_updates);
            egui_ui::render_block_inspector(ui, &self.stream_blocks);
        });
    }
}
