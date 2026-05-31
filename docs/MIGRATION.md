# Frostmark stack migration guide

This document tracks the frostmark-only migration from comrak + full-document RcDom to pulldown-cmark + mdstream + `RenderBlock` / `HtmlFragment`.

Nova application integration is **out of scope** for this crate. Tasks 4.5 and 7.4 are validated by egui harness examples inside frostmark:

- `examples/egui_table_harness` — shared table path (static + stream)
- `examples/egui_pipeline_harness` — unified pipeline without app workarounds
- `./scripts/verify-egui-harness.sh` — CI-friendly checks

## Target architecture

```text
Markdown -> pulldown / mdstream -> RenderBlock -> backend (iced / headless HTML)
Raw HTML -> html5ever TreeSink -> HtmlFragment -> backend
```

## Public Cargo contract

Enable only these features in downstream `Cargo.toml`:

```toml
frostmark = { version = "0.3", default-features = false, features = ["no_iced", "static", "stream"] }
```

| Feature | When to use |
|---------|-------------|
| `no_iced` | CI, export, servers — no GPU / no iced window |
| `static` | Full-document preview, `Document::to_html()` |
| `stream` | LLM token streaming via `StreamDocument` |

Default builds (`default-features = true`) still enable iced + migration fallbacks.

## Implementation-only features (unsupported)

- `_iced_backend` — iced renderer (on by default)
- `_legacy_comrak` — comrak shadow/fallback until pulldown parity
- `_rcdom_compat` — RcDom bridge for parity tests only (production iced path uses TreeSink → `HtmlFragment`)
- `_html_preprocess` — optional `lol_html` rewrite layer

Do not depend on these in application code.

## Integration patterns

### Static preview

```rust
use frostmark::{Document, MarkState, ParseProfile};

let doc = Document::parse(source, ParseProfile::GitHubPreview)?;
let state = MarkState::from_document(&doc);
```

Headless export:

```rust
let html = doc.to_html()?;
```

### LLM streaming

```rust
use frostmark::{MarkState, StreamDocument, StreamOptions};

let mut stream = StreamDocument::new(StreamOptions::chat());
let update = stream.append(chunk);
// Apply update.patch to UI; then:
let mut state = MarkState::from_document(/* or from_blocks */);
state.sync_from_stream(&stream);
```

Use `StreamOptions::chat()` — it sets footnote/reference invalidation expected by chat UIs.

## Legacy removal gate

Do **not** remove `_legacy_comrak` or production `markup5ever_rcdom` until [LEGACY_REMOVAL_GATE.md](LEGACY_REMOVAL_GATE.md) is fully green.

Current known accepted delta:

| Fixture | Delta |
|---------|--------|
| `tests/fixtures/gfm_wikilink.md` | pulldown wikilink HTML ≠ comrak (shadow compare only) |

## Verification

```bash
./scripts/verify-features.sh
./scripts/verify-all-features.sh   # optional full matrix + clippy
cargo test
cargo test --no-default-features --features no_iced,static,stream
cargo test --features stream --test stream_parity
```

## Task status (frostmark repo)

Phases 0–7 core implementation and parity fixtures are complete in `nova_refs/frostmark`.

Remaining frostmark crate work:

- **8.1** — remove comrak after 2-week stabilization window (wikilink policy documented; pulldown canonical)
- **8.2** — done: production iced path uses `HtmlFragment` without RcDom
- **8.3** — done: README, `docs/API.md`, headless/migration guides
- **8.4** — done: see [CRATE_SPLIT_SPIKE.md](CRATE_SPLIT_SPIKE.md) (recommend single crate for 0.3)
- **4.5 / 7.4** — done via egui harness examples (`egui_table_harness`, `egui_pipeline_harness`)
