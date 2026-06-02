//! Static Markdown profiling harness for parse/build/render costs.
//!
//! ```bash
//! cargo run --release --example profile_static_mark -- --source README.md --render-rounds 100
//! cargo flamegraph --example profile_static_mark -o /tmp/static_mark.svg -- --source README.md --render-rounds 100
//! heaptrack target/release/examples/profile_static_mark --source README.md --render-rounds 100
//! ```

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use iced::{Element, Theme};
use strimd::{Document, MarkState, MarkWidget, ParseProfile};
use tracing::{debug, info, info_span};

fn main() {
    let mut render_rounds = 100usize;
    let mut rebuild_rounds = 1usize;
    let mut source_path: Option<PathBuf> = None;
    let mut trace_enabled = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--render-rounds" => {
                if let Some(value) = args.next() {
                    render_rounds = value.parse().unwrap_or(render_rounds);
                }
            }
            "--rebuild-rounds" => {
                if let Some(value) = args.next() {
                    rebuild_rounds = value.parse().unwrap_or(rebuild_rounds);
                }
            }
            "--source" => {
                if let Some(value) = args.next() {
                    source_path = Some(PathBuf::from(value));
                }
            }
            "--trace" => trace_enabled = true,
            _ => {}
        }
    }

    if trace_enabled {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "profile_static_mark=info".into()),
            )
            .with_target(false)
            .try_init();
    }

    let source = source_path
        .as_ref()
        .map(std::fs::read_to_string)
        .transpose()
        .expect("read source")
        .unwrap_or_else(|| include_str!("assets/TEST.md").to_string());

    let _overall = info_span!(
        "profile_static_mark",
        bytes = source.len(),
        rebuild_rounds,
        render_rounds
    )
    .entered();

    let parse_started = Instant::now();
    let mut last_state = None;
    for round in 0..rebuild_rounds.max(1) {
        let _round = info_span!("rebuild_round", round).entered();
        let document =
            Document::parse(&source, ParseProfile::GitHubPreview).expect("parse markdown");
        let state = MarkState::from_document(&document);
        debug!(blocks = document.blocks().len(), "rebuilt document state");
        last_state = Some(state);
    }
    let parse_ms = parse_started.elapsed().as_secs_f64() * 1000.0;

    let state = last_state.expect("state");
    let render_started = Instant::now();
    for round in 0..render_rounds.max(1) {
        let _round = info_span!("render_round", round).entered();
        let element: Element<'_, (), Theme> = MarkWidget::new(&state).into();
        black_box(element);
        debug!(render = round + 1, "rendered widget");
    }
    let render_ms = render_started.elapsed().as_secs_f64() * 1000.0;

    info!(
        parse_ms,
        render_ms,
        total_ms = parse_ms + render_ms,
        "static profiling run complete"
    );
    println!(
        "fixture_bytes={} rebuild_rounds={} render_rounds={} parse_ms={:.2} render_ms={:.2} total_ms={:.2}",
        source.len(),
        rebuild_rounds,
        render_rounds,
        parse_ms,
        render_ms,
        parse_ms + render_ms
    );
}
