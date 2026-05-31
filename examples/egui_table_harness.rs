//! Task 4.5 — egui harness: static + streamed GFM tables via shared StriMD model.
//!
//! ```bash
//! # Headless checks (CI)
//! cargo run --example egui_table_harness --no-default-features --features no_iced,static,stream -- --check
//!
//! # Visual inspection
//! cargo run --example egui_table_harness --no-default-features --features no_iced,static,stream
//! ```

#![cfg(all(feature = "no_iced", feature = "static", feature = "stream"))]

#[path = "shared/harness_checks.rs"]
mod harness_checks;

#[path = "shared/egui_ui.rs"]
mod egui_ui;

use eframe::egui;
use strimd::{Document, ParseProfile};

fn main() -> eframe::Result<()> {
    if std::env::args().any(|a| a == "--check") {
        match harness_checks::run_all_checks() {
            Ok(()) => {
                eprintln!("egui_table_harness: all checks passed");
                return Ok(());
            }
            Err(e) => {
                eprintln!("egui_table_harness: FAIL — {e}");
                std::process::exit(1);
            }
        }
    }

    let static_source = include_str!("../tests/fixtures/gfm_table.md");
    let stream_source = include_str!("../tests/fixtures/stream_table.md");

    let static_doc =
        Document::parse(static_source, ParseProfile::GitHubPreview).expect("static parse");
    let static_html = static_doc.to_html().expect("static html");

    let (stream_doc, stream_updates) = egui_ui::append_stream_chunks(stream_source, 4);
    let stream_blocks = egui_ui::collect_blocks(&stream_doc);

    let static_err = harness_checks::check_static_table_path().err();
    let stream_err = harness_checks::check_stream_table_path().err();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 720.0])
            .with_title("StriMD egui table harness (Task 4.5)"),
        ..Default::default()
    };

    eframe::run_native(
        "StriMD egui table harness",
        native_options,
        Box::new(|_ctx| {
            Ok(Box::new(TableHarnessApp {
                static_doc,
                static_html,
                _stream_doc: stream_doc,
                stream_updates,
                stream_blocks,
                static_err,
                stream_err,
                stream_source: stream_source.to_string(),
            }))
        }),
    )
}

struct TableHarnessApp {
    static_doc: Document,
    static_html: String,
    _stream_doc: strimd::StreamDocument,
    stream_updates: Vec<strimd::StreamUpdate>,
    stream_blocks: Vec<strimd::RenderBlock>,
    static_err: Option<String>,
    stream_err: Option<String>,
    stream_source: String,
}

impl eframe::App for TableHarnessApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Task 4.5 — shared StriMD table path");
            ui.label("No chat_table workaround — tables from BlockKind::Table only.");

            ui.separator();
            ui.heading("Static GFM table");
            egui_ui::show_check_errors(
                ui,
                &self.static_err.clone().into_iter().collect::<Vec<_>>(),
            );
            egui_ui::render_document_summary(ui, &self.static_doc, &self.static_html);

            ui.separator();
            ui.heading("Streamed table (chunk size 4)");
            egui_ui::show_check_errors(
                ui,
                &self.stream_err.clone().into_iter().collect::<Vec<_>>(),
            );
            ui.label(format!(
                "append patches: {}",
                egui_ui::count_append_committed(&self.stream_updates)
            ));
            egui_ui::render_stream_patch_log(ui, &self.stream_updates);
            egui_ui::render_table_blocks(ui, &self.stream_blocks);
            ui.collapsing("stream source", |ui| {
                ui.monospace(&self.stream_source);
            });
        });
    }
}
