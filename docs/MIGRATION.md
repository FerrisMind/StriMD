# Frostmark stack migration guide

This document tracks the frostmark-only migration from comrak + full-document RcDom to pulldown-cmark + mdstream + `RenderBlock` / `HtmlFragment`.

Nova application integration (including removal of `chat_table.rs`) is **out of scope** for this crate; see your app repo after frostmark parity gates pass.

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
- `_rcdom_compat` — RcDom bridge for iced HTML traversal
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

- **8.1 / 8.2** — remove comrak and RcDom after stabilization window and wikilink policy
- **8.3** — this document + README (ongoing)
- **4.5 / 7.4** — application table workarounds (e.g. `chat_table.rs`) live in downstream apps, not in frostmark
