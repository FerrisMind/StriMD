Some examples to show off functionality and act as a reference.

# Hello

![A showcase of basic HTML/Markdown features](assets/hello.png)

```sh
cargo run --example hello
```

---

# Live Editing

Editing the document through a text editor,
with a live preview of the result.

![](assets/live_edit.png)

```sh
cargo run --example live_edit
```

---

# Image

A more advanced version of **"Live Edit"**
which also showcases basic rendering of images.

> Note: This doesn't deal with SVG images,
> for that, see the **Large Readme** example.

![](assets/image.png)

```sh
cargo run --example image --features="iced/image"
```

---

# Large Readme

Renders two large READMEs:
- One being a custom test page that covers all available formatting features
- One being an example README of a large project ([QuantumLauncher](https://github.com/Mrmayman/quantumlauncher))

Demonstrates:
- Async image loading
- SVG rendering
- Handling link clicks

Side-by-side comparison with frostmark (left) and VSCode (right):

![](assets/large_readme.png)

```sh
cargo run --example large_readme --features="iced/image iced/svg"
```

---

# Styling

An example for styling text and widgets.

```sh
cargo run --example styling
```

---

# Static Export (headless)

Parse `assets/TEST.md` through `Document` and export HTML without iced/GPU.

```sh
cargo run --example static_export --no-default-features --features no_iced,static
```

---

# Stream Chat (headless)

Simulate LLM token streaming through `StreamDocument` and stream patches.

```sh
cargo run --example stream_chat --no-default-features --features no_iced,stream
```
