# StriMD stack migration guide

This document tracks the StriMD migration from comrak + full-document RcDom to pulldown-cmark + mdstream + `RenderBlock` / `HtmlFragment`.

Nova application integration is **out of scope** for this crate. Tasks 4.5 and 7.4 are validated by egui harness examples inside StriMD:

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
strimd = { version = "1.0", default-features = false, features = ["no_iced", "static", "stream"] }
```

| Feature | When to use |
|---------|-------------|
| `no_iced` | CI, export, servers — no GPU / no iced window |
| `static` | Full-document preview, `Document::to_html()` |
| `stream` | LLM token streaming via `StreamDocument` |

Default builds (`default-features = true`) enable the iced backend with pulldown parsing.

## Implementation-only features (unsupported)

- `_iced_backend` — iced renderer (on by default)
- `_rcdom_compat` — RcDom bridge for parity tests only (production iced path uses TreeSink → `HtmlFragment`)
- `_html_preprocess` — optional `lol_html` rewrite layer

Do not depend on these in application code.

## Integration patterns

### Static preview

```rust
use strimd::{Document, MarkState, ParseProfile};

let doc = Document::parse(source, ParseProfile::GitHubPreview)?;
let state = MarkState::from_document(&doc);
```

Headless export:

```rust
let html = doc.to_html()?;
```

### LLM streaming

```rust
use strimd::{MarkState, StreamDocument, StreamOptions};

let mut stream = StreamDocument::new(StreamOptions::chat());
let update = stream.append(chunk);
// Apply update.patch to UI; then:
let mut state = MarkState::from_document(/* or from_blocks */);
state.sync_from_stream(&stream);
```

Use `StreamOptions::chat()` — it sets footnote/reference invalidation expected by chat UIs.

## Legacy removal gate

Comrak (`_legacy_comrak`) was removed in Task 8.1 after parity tests passed — see [LEGACY_REMOVAL_GATE.md](LEGACY_REMOVAL_GATE.md). Production `markup5ever_rcdom` is already out of the iced path; `_rcdom_compat` remains for parity tests only.

## Verification

```bash
./scripts/verify-features.sh
./scripts/verify-all-features.sh   # optional full matrix + clippy
cargo test
cargo test --no-default-features --features no_iced,static,stream
cargo test --features stream --test stream_parity
```

## Task status (StriMD repo)

Phases 0–7 core implementation and parity fixtures are complete in `nova_refs/strimd`.

Remaining StriMD crate work:

- **8.1** — done: comrak removed; pulldown-only parsing
- **8.2** — done: production iced path uses `HtmlFragment` without RcDom
- **8.3** — done: README, `docs/API.md`, headless/migration guides
- **8.4** — done: see [CRATE_SPLIT_SPIKE.md](CRATE_SPLIT_SPIKE.md) (single crate for 1.0)
- **4.5 / 7.4** — done via egui harness examples (`egui_table_harness`, `egui_pipeline_harness`)
