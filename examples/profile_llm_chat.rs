//! Streaming pipeline profiling harness for the `llm_chat` example.
//!
//! ```bash
//! cargo run --release --example profile_llm_chat -- --chunk-words 1 --render-every 1
//! cargo flamegraph --example profile_llm_chat -- --chunk-words 1 --render-every 1
//! heaptrack target/release/examples/profile_llm_chat --chunk-words 1 --render-every 1
//! ```

use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use iced::{Element, Theme};
use strimd::{MarkState, MarkWidget, StreamDocument, StreamOptions};
use tracing::{debug, info, info_span};
use tracing_subscriber::Layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    let mut chunk_words = 1usize;
    let mut render_every = 1usize;
    let mut flush_every_chunks = 1usize;
    let mut rounds = 5usize;
    let mut dump_final_blocks = false;
    let mut source_path: Option<PathBuf> = None;
    let mut trace_enabled = false;
    let mut tracy_enabled = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--chunk-words" => {
                if let Some(value) = args.next() {
                    chunk_words = value.parse().unwrap_or(chunk_words);
                }
            }
            "--render-every" => {
                if let Some(value) = args.next() {
                    render_every = value.parse().unwrap_or(render_every);
                }
            }
            "--flush-every-chunks" => {
                if let Some(value) = args.next() {
                    flush_every_chunks = value.parse().unwrap_or(flush_every_chunks);
                }
            }
            "--rounds" => {
                if let Some(value) = args.next() {
                    rounds = value.parse().unwrap_or(rounds);
                }
            }
            "--dump-final-blocks" => dump_final_blocks = true,
            "--source" => {
                if let Some(value) = args.next() {
                    source_path = Some(PathBuf::from(value));
                }
            }
            "--trace" => trace_enabled = true,
            "--tracy" => tracy_enabled = true,
            _ => {}
        }
    }

    if trace_enabled || tracy_enabled {
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "profile_llm_chat=info".into());
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_filter(env_filter.clone());
        let subscriber = tracing_subscriber::registry().with(fmt_layer);

        if tracy_enabled {
            let tracy_layer = tracing_tracy::TracyLayer::default().with_filter(env_filter.clone());
            subscriber.with(tracy_layer).init();
        } else {
            subscriber.init();
        }
    }

    let source = source_path
        .as_ref()
        .map(std::fs::read_to_string)
        .transpose()
        .expect("read source")
        .unwrap_or_else(|| include_str!("assets/TEST.md").to_string());
    let chunks = chunk_by_words(&source, chunk_words.max(1));

    let started = Instant::now();
    let mut total_appends = 0usize;
    let mut total_renders = 0usize;
    let mut total_flushes = 0usize;
    let mut total_flushed_bytes = 0usize;
    let _overall = info_span!(
        "profile_llm_chat",
        bytes = source.len(),
        chunks = chunks.len(),
        rounds,
        flush_every_chunks,
        render_every
    )
    .entered();

    for round in 0..rounds.max(1) {
        let _round = info_span!("round", round).entered();
        let mut stream = StreamDocument::new(StreamOptions::chat());
        let mut state = MarkState::default();
        state.sync_from_stream(&stream);
        let mut pending = String::new();
        let mut buffered_chunks = 0usize;

        for (index, chunk) in chunks.iter().enumerate() {
            pending.push_str(chunk);
            buffered_chunks += 1;

            if buffered_chunks >= flush_every_chunks.max(1) || index + 1 == chunks.len() {
                let flush_bytes = pending.len();
                let update = {
                    let _span = info_span!("stream_append", round, chunk_index = index).entered();
                    stream.append(&pending)
                };
                {
                    let _span = info_span!("state_apply_stream_update", round, chunk_index = index)
                        .entered();
                    state.apply_stream_update(&stream, &update);
                }
                total_appends += 1;
                total_flushes += 1;
                total_flushed_bytes += flush_bytes;
                pending.clear();
                buffered_chunks = 0;
                debug!(
                    flush = total_flushes,
                    chunk_index = index,
                    bytes = flush_bytes,
                    patch = ?update.patch,
                    "applied stream flush"
                );

                if (total_flushes - 1).is_multiple_of(render_every.max(1))
                    || index + 1 == chunks.len()
                {
                    let element: Element<'_, (), Theme> = {
                        let _span =
                            info_span!("widget_render", round, chunk_index = index).entered();
                        MarkWidget::new(&state).into()
                    };
                    black_box(element);
                    total_renders += 1;
                    debug!(
                        render = total_renders,
                        flush = total_flushes,
                        "rendered widget"
                    );
                }
            }
        }

        if dump_final_blocks {
            println!("=== FINAL BLOCKS FOR ROUND {} ===", round);
            for block in stream.blocks() {
                println!(
                    "Block #{} - kind: {:?}, content: {:?}",
                    block.id.0, block.kind, block.content
                );
            }
        }
    }

    let elapsed = started.elapsed();
    info!(
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        appends = total_appends,
        flushes = total_flushes,
        renders = total_renders,
        flushed_bytes = total_flushed_bytes,
        "profiling run complete"
    );
    println!(
        "fixture_bytes={} chunks={} rounds={} flush_every_chunks={} render_every={} appends={} renders={} elapsed_ms={:.2}",
        source.len(),
        chunks.len(),
        rounds,
        flush_every_chunks,
        render_every,
        total_appends,
        total_renders,
        elapsed.as_secs_f64() * 1000.0
    );
}

fn chunk_by_words(text: &str, words_per_chunk: usize) -> Vec<String> {
    let words_per_chunk = words_per_chunk.max(1);
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut words_in_chunk = 0usize;
    let mut i = 0usize;

    while i < text.len() {
        let rest = &text[i..];
        if let Some(ws) = rest.chars().next().filter(|c| c.is_whitespace()) {
            let len = ws.len_utf8()
                + rest
                    .chars()
                    .skip(1)
                    .take_while(|c| c.is_whitespace())
                    .map(|c| c.len_utf8())
                    .sum::<usize>();
            current.push_str(&text[i..i + len]);
            i += len;
            continue;
        }

        let word_len = rest
            .chars()
            .take_while(|c| !c.is_whitespace())
            .map(|c| c.len_utf8())
            .sum::<usize>();
        current.push_str(&text[i..i + word_len]);
        i += word_len;
        words_in_chunk += 1;

        if words_in_chunk >= words_per_chunk {
            chunks.push(std::mem::take(&mut current));
            words_in_chunk = 0;
        }
    }

    if !current.is_empty() || chunks.is_empty() {
        chunks.push(current);
    }
    chunks
}
