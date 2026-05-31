# 🧊 Frostmark

**An HTML + Markdown viewer for [iced](https://iced.rs/)**

Render rich text in your `iced` app at lightning-fast speeds using plain HTML or Markdown!

![(Demo showing HTML and Markdown together)](https://github.com/Mrmayman/frostmark/raw/main/examples/assets/live_edit.png)

---

## Usage

1. Create a [`MarkState`] and **store it in your application state**.

```no_run
#[cfg(feature = "_iced_backend")]
fn iced_usage() {
    use frostmark::MarkState;

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
    use frostmark::{MarkState, MarkWidget};
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

**Note:** Markdown parsing uses **pulldown-cmark** only (comrak removed in 0.3).
Headless consumers should disable default features and enable only the supported
public features they need.

## How does this work

The default **iced** backend still renders HTML via [`html5ever`](https://crates.io/crates/html5ever/)
and `MarkWidget`. New **headless** APIs parse Markdown into backend-agnostic
[`Document`](https://docs.rs/frostmark/latest/frostmark/struct.Document.html) /
[`StreamDocument`](https://docs.rs/frostmark/latest/frostmark/struct.StreamDocument.html)
blocks using **pulldown-cmark** and vendored **mdstream** for streaming.

## Supported Crate Features (public contract)

| Feature | Purpose |
|---------|---------|
| `no_iced` | Headless mode — use with `default-features = false` |
| `static` | Full-document parse and `Document::to_html()` export |
| `stream` | Incremental LLM streaming via `StreamDocument` |

Example headless dependency:

```toml
frostmark = { path = "...", default-features = false, features = ["no_iced", "static", "stream"] }
```

Headless examples (no GPU / no iced window):

```sh
# Static preview + HTML export from TEST.md
cargo run --example static_export --no-default-features --features no_iced,static

# Simulated LLM token streaming via StreamDocument
cargo run --example stream_chat --no-default-features --features no_iced,stream
```

CI runs `./scripts/verify-features.sh` and headless jobs via `.github/workflows/ci.yml`.

See [docs/API.md](docs/API.md) for the public API reference, [docs/MIGRATION.md](docs/MIGRATION.md) for the pulldown/mdstream migration guide, and [docs/LEGACY_REMOVAL_GATE.md](docs/LEGACY_REMOVAL_GATE.md) for comrak/RcDom removal criteria.

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
| 0.3 | **0.14** | **1.88** |
| 0.2 | 0.13 | 1.82 |
| 0.1 | 0.13 | 1.82 |

# Contributing

This library is experimental.
Bug reports and pull requests are welcome;
contributions are appreciated!

## Contributors

- **[Mrmayman](https://github.com/Mrmayman) - Creator**
- [mariinkys](https://github.com/mariinkys) - Tables, other changes
- [Drodofsky](https://github.com/Drodofsky) - Ruby text support
- [ruguysgoingtrickortreating](https://github.com/ruguysgoingtrickortreating) - Updated to iced 0.14

---

**License**: Dual licensed under MIT and Apache 2.0.
