# StriMD

**HTML + Markdown viewer for [iced](https://iced.rs/) — static, streaming, and headless**

Hard fork of [frostmark](https://github.com/Mrmayman/frostmark). Same iced widget surface where applicable; new headless APIs and a different Markdown/HTML stack — see [Provenance](#provenance--acknowledgments).

![(Demo showing HTML and Markdown together)](https://github.com/Mrmayman/frostmark/raw/main/examples/assets/live_edit.png)

---

## Usage

1. Create a [`MarkState`] and **store it in your application state**.

```no_run
#[cfg(feature = "_iced_backend")]
fn iced_usage() {
    use strimd::MarkState;

    let text = "Hello from **markdown** and <b>HTML</b>!";
    let _state = MarkState::with_html_and_markdown(text);
    let _state = MarkState::with_html(text);
}
```

2. In your `view` function use a [`MarkWidget`].

```txt
iced::widget::container( // just an example
    MarkWidget::new(&self.mark_state)
)
.padding(10)
```

You can find runnable examples [here](examples/README.md)

> **Note:** Code blocks in this readme are `no_run` only — they compile during
> `cargo test` but do **not** open an iced window. To see the UI, run
> `cargo run --example hello` (or `live_edit`).

<details>
<summary>Click to expand a full example</summary>

```no_run
#[cfg(feature = "_iced_backend")]
fn iced_full_example() {
    use strimd::{MarkState, MarkWidget};
    use iced::{widget, Element, Task};

    const YOUR_TEXT: &str = "Hello from **markdown** and <b>HTML</b>!";

    #[derive(Debug, Clone)]
    enum Message {}

    struct App {
        state: MarkState,
    }

    impl App {
        fn update(&mut self, _: Message) -> Task<Message> {
            Task::none()
        }

        fn view(&self) -> Element<'_, Message> {
            widget::container(MarkWidget::new(&self.state))
                .padding(10)
                .into()
        }
    }

    fn main() {
        iced::application(
            || App {
                state: MarkState::with_html_and_markdown(YOUR_TEXT),
            },
            App::update,
            App::view,
        )
        .run()
        .unwrap();
    }
}
```

</details>

**Note:** Markdown parsing uses **pulldown-cmark** only (comrak removed in 1.0). See [changelogs/1.0.0.md](changelogs/1.0.0.md) for the stack migration release notes.
Headless consumers should disable default features and enable only the supported
public features they need.

## How does this work

The default **iced** backend still renders HTML via [`html5ever`](https://crates.io/crates/html5ever/)
and `MarkWidget`. New **headless** APIs parse Markdown into backend-agnostic
[`Document`](https://docs.rs/strimd/latest/strimd/struct.Document.html) /
[`StreamDocument`](https://docs.rs/strimd/latest/strimd/struct.StreamDocument.html)
blocks using **pulldown-cmark** and vendored **mdstream** for streaming.

## Supported Crate Features (public contract)

| Feature | Purpose |
|---------|---------|
| `no_iced` | Headless mode — use with `default-features = false` |
| `static` | Full-document parse and `Document::to_html()` export |
| `stream` | Incremental LLM streaming via `StreamDocument` |

Example headless dependency:

```toml
strimd = { path = "...", default-features = false, features = ["no_iced", "static", "stream"] }
```

Headless examples (no GPU / no iced window):

```sh
# Static preview + HTML export from TEST.md
cargo run --example static_export --no-default-features --features no_iced,static

# Simulated LLM token streaming via StreamDocument
cargo run --example stream_chat --no-default-features --features no_iced,stream
```

CI runs `./scripts/verify-features.sh` and headless jobs via `.github/workflows/ci.yml`.

See [docs/API.md](docs/API.md) for the public API reference, [docs/MIGRATION.md](docs/MIGRATION.md) for the pulldown/mdstream migration guide, [changelogs/1.0.0.md](changelogs/1.0.0.md) for the 1.0 release notes, and [docs/LEGACY_REMOVAL_GATE.md](docs/LEGACY_REMOVAL_GATE.md) for legacy removal criteria.

## Implementation-only features (unsupported)

These exist for migration and may change without notice:

- `_iced_backend` — default iced renderer (on by default)
- `_html_preprocess` — optional `lol_html` rewrite layer
- `_rcdom_compat` — `markup5ever_rcdom` bridge for migration parity tests only (not used by default iced backend)

## Iced passthrough features

- `markdown` — alias for `static` (pulldown HTML export)
- `iced-wgpu`, `iced-tiny-skia`, `iced-tokio`, `iced-windowing` — forwarded to `iced`

## TODO

- Support for more elements (eg: superscript)
- Fix `<ruby>` edge cases
- (Maybe) support for CSS?

---

| Version | iced | MSRV |
|:-:|:-:|:-:|
| 1.0 | **0.14** | **1.88** |
| 0.3 | **0.14** | **1.88** |
| 0.2 | 0.13 | 1.82 |
| 0.1 | 0.13 | 1.82 |

## Provenance & acknowledgments

**StriMD** (`strimd`) is a **hard fork** of [frostmark](https://github.com/Mrmayman/frostmark) by FerrisMind — not a continuation of the upstream 0.3.x release line.

| | frostmark (upstream) | StriMD (this repo) |
|---|---|---|
| Crate name | `frostmark` | `strimd` |
| Markdown parser | comrak | pulldown-cmark |
| Headless / stream | — | `Document`, `StreamDocument`, mdstream |
| Last shared commit | [`fbe35a6`](https://github.com/Mrmayman/frostmark/commit/fbe35a6) (0.3.1 line) | 13+ commits of stack migration |

Upstream copyright and attribution are in [NOTICE](NOTICE). Stack migration: [changelogs/1.0.0.md](changelogs/1.0.0.md).

# Contributing

This library is experimental.
Bug reports and pull requests are welcome;
contributions are appreciated!

## Contributors

### frostmark (upstream)

- **[Mrmayman](https://github.com/Mrmayman)** — creator
- [mariinkys](https://github.com/mariinkys) — tables and related work
- [Drodofsky](https://github.com/Drodofsky) — ruby text support
- [ruguysgoingtrickortreating](https://github.com/ruguysgoingtrickortreating) — iced 0.14 update

### StriMD (FerrisMind fork)

- **[FerrisMind](https://github.com/FerrisMind)** — stack migration (pulldown, mdstream, headless API), maintenance

---

**License**: [Apache License 2.0](LICENSE). Upstream frostmark attribution and MIT notice: [NOTICE](NOTICE).
