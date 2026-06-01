# Markdown Preview Fixture (GFM)

Visual regression file for Nova editor markdown preview.

**Primary reference:** [GitHub Flavored Markdown Spec](https://github.github.io/gfm/) (CommonMark + GFM extensions, v0.29-gfm).

**Secondary reference:** [GitHub writing quickstart](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/quickstart-for-writing-on-github) for README-style HTML (`<picture>`, `<details>`, etc.).

Open this file in Nova → **Markdown Preview**.

---

## Spec coverage map

| GFM section | Topic | In this file |
|-------------|-------|--------------|
| §4.1 | Thematic breaks | yes |
| §4.2 | ATX headings | yes |
| §4.3 | Setext headings | yes |
| §4.4 | Indented code blocks | yes |
| §4.5 | Fenced code blocks | yes |
| §4.6 | HTML blocks | yes |
| §4.7 | Link reference definitions | yes |
| §4.8 | Paragraphs | yes |
| §4.10 | **Tables (extension)** | yes |
| §5.1 | Block quotes | yes |
| §5.2 | List items (tight / loose) | yes |
| §5.3 | **Task lists (extension)** | yes |
| §5.4 | Lists (nested, markers) | yes |
| §6.1 | Backslash escapes | yes |
| §6.2 | Entity references | yes |
| §6.3 | Code spans | yes |
| §6.4 | Emphasis / strong | yes |
| §6.5 | **Strikethrough (extension)** | yes |
| §6.6 | Links | yes |
| §6.7 | Images | yes |
| §6.8 | Autolinks (`<url>`) | yes |
| §6.9 | **Autolinks (extension)** (`www.`, email) | yes |
| §6.10 | Raw HTML inlines | yes |
| §6.11 | **Disallowed raw HTML (extension)** | yes |
| §6.12 | Hard line breaks | yes |
| §6.13 | Soft line breaks | yes |
| — | Nova/comrak extras (underline, sub/sup) | noted |
| — | **GitHub README HTML** | [full section](#github-readme-html) |

---

# §4 Leaf blocks

## §4.1 Thematic breaks

Valid rules (GFM examples 13, 17):

***

---

___

   ***

Invalid (must stay plain text, examples 14–16):

+++

===

--

## §4.2 ATX headings

# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

Heading with trailing hashes ### still works ###

## §4.3 Setext headings

Foo *bar*
=========

Foo *bar*
---------

## §4.4 Indented code blocks

    fn main() {
        println!("indented code block");
    }

Paragraph before indented block is required in strict parsers.

    second block

## §4.5 Fenced code blocks

````markdown
```rust
fn main() {}
```
````

Nova preview: syntax highlighting + copy button on fenced blocks.

```rust
struct Calculator;

impl iced::Application for Calculator {
    fn title(&self) -> String {
        "Calculator".into()
    }
}
```

```powershell
Set-Location .\nova
cargo run -p nova-app
```

```yaml
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
```

```toml
[package]
name = "nova-app"
edition = "2024"
```

Language aliases: `powershell`, `yml`, `shell`.

## §4.6 HTML blocks

<div align="center">

HTML block: centered div

</div>

<table>
  <tr><th>A</th><th>B</th></tr>
  <tr><td>1</td><td>2</td></tr>
</table>

## §4.7 Link reference definitions

[ref link][ref-id] and [second ref][]

[ref-id]: https://github.github.io/gfm/ "GFM spec"
[second ref]: https://docs.github.com/

## §4.8 Paragraphs

First paragraph with **strong**, *emphasis*, ***both***, and `code`.

Second paragraph after blank line. Soft break inside paragraph
continues on next source line (§6.13).

Hard break with two trailing spaces  
Next line after hard break (§6.12).

---

## §4.10 Tables (GFM extension)

Pipe table, default alignment:

| Option | Description |
| ------ | ----------- |
| data   | Path to data files |
| engine | Template engine |

Center columns (`:---:`):

| Left | Center | Right |
|:-----|:------:|------:|
| a    | b      | 1     |

Inline markup in cells:

| Feature | Status |
|---------|--------|
| `code`  | **ok** |

---

# §5 Container blocks

## §5.1 Block quotes

> Single line quote.

> Multi-line
> block quote.

Nested:

> Outer
>
> > Inner quote
> >
> > With `inline code`

## §5.2 List items — tight vs loose

Tight list (no blank lines between items):

- one
- two
- three

Loose list (blank line inside item → wrapped in `<p>`):

- item one

  continuation paragraph

- item two

List after heading (README pattern):

**Files:**

- Create: `src/main.rs`
- Modify: `Cargo.toml`

## §5.3 Task list items (GFM extension)

GFM examples 279–280:

- [ ] foo
- [x] bar

Nested tasks:

- [x] foo
  - [ ] bar
  - [x] baz
- [ ] bim

## §5.4 Lists

Unordered markers (`-`, `*`, `+`):

- dash
* star
+ plus

Ordered:

1. first
2. second
   1. nested ordered
   2. nested ordered
3. third

Mixed nesting:

1. ordered parent
   - unordered child
     1. ordered grandchild

Marker start number (example 287 style):

67. sixty-seven
1. resets visually

---

# §6 Inlines

## §6.1 Backslash escapes

\*not emphasis\*

\[not a link\](url)

## §6.2 Entity and numeric character references

&copy; 2026 &mdash; Nova &bull; &#x2665;

## §6.3 Code spans

Requires Rust `1.93` or newer.

Backtick with spaces: `` `code` `` and `` ` ``.

Plain dotted number (must **not** become ordered list):

1.93

Inline in prose: through a `Calculator` struct implementing `iced::Application`.

## §6.4 Emphasis and strong emphasis

*italic* _italic_

**bold** __bold__

***bold italic*** ___bold italic___

Word-level emphasis: un*frigging*believable.

## §6.5 Strikethrough (GFM extension)

GFM example 491:

~~Hi~~ Hello, ~there~ world!

Must not strike (example 493):

This will ~~~not~~~ strike.

## §6.6 Links

Inline: [GFM spec](https://github.github.io/gfm/)

Reference: [CommonMark help][cm]

[cm]: https://commonmark.org/help/

Autolink with title from ref above.

## §6.7 Images

Markdown image:

![Nova icon](crates/nova-app/assets/icons/icon.svg)

HTML image:

<img src="crates/nova-app/assets/icons/icon.svg" width="48" alt="Nova icon">

Link with image: [![icon](crates/nova-app/assets/icons/icon.svg)](https://github.github.io/gfm/)

## §6.8 Autolinks

<https://github.github.io/gfm/>

<user@example.com>

## §6.9 Autolinks (GFM extension)

www.commonmark.org

Visit www.commonmark.org/help for more information.

Trailing punctuation excluded: www.example.com.

Email: contact@example.com

## §6.10 Raw HTML inlines

<em>HTML emphasis</em>, <strong>strong</strong>, <code>html code</code>, <kbd>Ctrl</kbd>, <mark>highlight</mark>.

Line break: line one<br>line two

## §6.11 Disallowed raw HTML (GFM extension)

GFM example 657 — tags filtered in HTML output (`<` → `&lt;`):

<strong> <title> <style> <em>

<blockquote>
  <xmp> is disallowed.  <XMP> is also disallowed.
</blockquote>

<script>alert("xss")</script> should not execute.

## §6.12 Hard line breaks

Two spaces at end of line  
This is a new line.

Backslash before newline also works\
like this.

## §6.13 Soft line breaks

This is one paragraph
because the line break is soft.

---

# Nova / comrak extras (beyond GFM core)

Enabled in Nova `comrak` options but not part of base GFM spec:

__underline__

H~2~O subscript

X^2^ superscript

---

# GitHub README HTML

Patterns commonly used on [github.com](https://github.com) README/profile pages and in the [GitHub writing quickstart](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/quickstart-for-writing-on-github). These are **not** part of the core [GFM spec](https://github.github.io/gfm/), but Nova preview should handle them via frostmark + comrak (`render.unsafe = true`).

## README header layout (table + picture)

Same pattern as `nova/README.md` — icon column + wordmark with theme-aware SVG:

<table>
  <tr>
    <td align="center" valign="middle" width="120">
      <img src="crates/nova-app/assets/icons/icon.svg" height="100" alt="Nova Code Icon" />
    </td>
    <td align="left" valign="middle">
      <h1><picture>
        <source media="(prefers-color-scheme: dark)" srcset="crates/nova-app/assets/icons/icon-wordmark-white.svg" />
        <source media="(prefers-color-scheme: light)" srcset="crates/nova-app/assets/icons/icon-wordmark.svg" />
        <img src="crates/nova-app/assets/icons/icon-wordmark.svg" height="64" alt="Nova IDE" />
      </picture></h1>
      <p><code>Nova Code</code> — preview fixture for GitHub-style README HTML.</p>
    </td>
  </tr>
</table>

## Responsive images

### `<picture>` with `prefers-color-scheme` (GitHub quickstart)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://user-images.githubusercontent.com/25423296/163456776-7f95b81a-f1ed-45f7-b7ab-8fa810d529fa.png">
  <source media="(prefers-color-scheme: light)" srcset="https://user-images.githubusercontent.com/25423296/163456779-a8556205-d0a5-45e2-ac17-42d089e3c3f8.png">
  <img alt="Sun in light mode, moon in dark mode." src="https://user-images.githubusercontent.com/25423296/163456779-a8556205-d0a5-45e2-ac17-42d089e3c3f8.png">
</picture>

### Plain `<img>` attributes

<img src="crates/nova-app/assets/icons/icon.svg" width="64" height="64" alt="Sized icon">

<img src="crates/nova-app/assets/icons/icon.svg" align="right" width="32" alt="Floated icon"> Text wrapping around a right-aligned image (when supported).

## Alignment blocks

<center>Deprecated `<center>` block</center>

<div align="center">

`<div align="center">` block

</div>

<p align="center">Centered paragraph via HTML attribute.</p>

## Collapsible sections (`<details>`)

Basic collapsed block (quickstart table inside):

<details>
<summary>My top languages</summary>

| Rank | Languages |
|-----:|-----------|
|     1| Rust      |
|     2| TypeScript|
|     3| SQL       |

</details>

Open by default:

<details open>
<summary>Open by default</summary>

Expanded content with **markdown** and `inline code`.

</details>

Nested details:

<details>
<summary>Outer section</summary>

Outer text.

<details>
<summary>Inner section</summary>

Inner text with list:

- nested A
- nested B

```rust
fn nested_code() {}
```

</details>

</details>

Multiple `<summary>` tags (only first should show when closed):

<details>
<summary>Visible summary</summary>
Hidden duplicate summary below should not appear when collapsed.
<summary>Hidden summary</summary>
</details>

## GitHub alert blockquotes (profile/docs style)

> [!NOTE]
> Useful information that users should know.

> [!TIP]
> Helpful advice for doing things better.

> [!IMPORTANT]
> Key information users need to know.

> [!WARNING]
> Urgent info that needs immediate attention.

> [!CAUTION]
> Advises about risks or negative outcomes.

## Inline HTML typography

<em>emphasis</em>, <strong>strong</strong>, <b>bold</b>, <i>italic</i>, <u>underline</u>, <s>strike</s>, <del>deleted</del>, <ins>inserted</ins>, <mark>highlighted</mark>, <sub>sub</sub>, <sup>sup</sup>, <code>inline code</code>, <span>span wrapper</span>.

Keyboard: <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>P</kbd>

Ruby: <ruby>東<rt>とう</rt>京<rt>きょう</rt></ruby>

## Links and badge row

<a href="https://github.github.io/gfm/">HTML anchor link</a>

Linked badge images (common in README headers):

<a href="https://github.github.io/gfm/">
  <img src="examples/assets/badge-gfm.svg" height="20" alt="GFM badge">
</a>
<a href="https://docs.github.com/">
  <img src="examples/assets/badge-docs.svg" height="20" alt="Docs badge">
</a>

## HTML tables (layout + data)

<table>
  <thead>
    <tr>
      <th align="left">Crate</th>
      <th align="center">Role</th>
      <th align="right">Layer</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><code>nova-app</code></td>
      <td align="center">binary</td>
      <td align="right">app</td>
    </tr>
    <tr>
      <td><code>nova-ui</code></td>
      <td align="center">widgets</td>
      <td align="right">ui</td>
    </tr>
  </tbody>
  <tfoot>
    <tr>
      <td colspan="3"><em>HTML table with thead/tbody/tfoot</em></td>
    </tr>
  </tfoot>
</table>

## Block elements via HTML

<blockquote>
  HTML blockquote (not markdown <code>&gt;</code> syntax).
</blockquote>

Horizontal rule via HTML:

<hr>

Line breaks:

Line one<br>Line two<br><br>Line three after double break.

## HTML code block

<pre><code>// pre/code without fence
let html_block = true;
</code></pre>

## Forms and inputs

Task checkbox (native HTML, distinct from GFM `- [ ]`):

<input type="checkbox" checked disabled> Completed HTML checkbox

Unsupported input types should degrade gracefully:

<input type="text" value="todo placeholder">

## Hidden comments

<!-- TO DO: verify GitHub README HTML section in Nova preview -->

## Mixed markdown + HTML

<div>

**Markdown bold** inside HTML div with a [markdown link](https://github.github.io/gfm/) and `inline code`.

<ul>
  <li>HTML list item with <code>code</code></li>
  <li>Second HTML item</li>
</ul>

</div>

## Quote block (quickstart)

> If we pull together and commit ourselves, then we can push through anything.
>
> — Mona the Octocat

---

# Preview checklist

When editing Nova markdown preview, verify:

**GFM core**

- [ ] §4.1 only `---` / `***` / `___` render as rules; `+++` stays text
- [ ] §4.3 setext headings render as H1/H2
- [ ] §5.2 loose vs tight lists; **Files:** bullets indented with `•`
- [ ] §5.3 task checkboxes without duplicate bullet markers
- [ ] §6.3 inline code pills: spacing + height next to prose
- [ ] §6.5 `~~strike~~` and `~one tilde~`
- [ ] §6.9 `www.` autolinks clickable
- [ ] §6.11 `<title>`, `<script>` escaped/disabled
- [ ] §4.5 fenced blocks highlighted with copy affordance
- [ ] `1.93` inline code vs plain `1.93` line (no false ordered list)

**GitHub README HTML**

- [ ] Header layout table (icon + `<picture>` wordmark) renders like `nova/README.md`
- [ ] `<picture>` / `<source>` / remote GitHub user-images URLs load or fail gracefully
- [ ] `<div align="center">`, `<center>`, `<p align="center">` alignment
- [ ] `<details>` / `<summary>` collapse; `open` attribute; nested details
- [ ] GitHub alert blockquotes (`> [!NOTE]`, `> [!WARNING]`, …) styled distinctly
- [ ] Inline HTML: `<kbd>`, `<mark>`, `<ruby>`, `<sub>`, `<sup>`, `<del>`, `<ins>`
- [ ] Linked badge `<a><img></a>` row
- [ ] HTML `<table>` with `<thead>` / `<tbody>` / `<tfoot>` and cell alignment
- [ ] `<blockquote>`, `<hr>`, `<br>`, `<pre><code>` HTML code block
- [ ] HTML `<ul>` / `<li>` nested inside `<div>` with markdown
- [ ] HTML comments hidden; unsupported `<input type="text">` degrades safely
