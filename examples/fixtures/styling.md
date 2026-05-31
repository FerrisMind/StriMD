This text uses `Style::text_color` (red).

[This link uses `Style::link_color` (magenta)](https://example.com)

<mark>This text uses `Style::highlight_color` (green background)</mark>

Inline `code` uses `Style::inline_code_color` and `inline_code_background`.

```rust
// Fenced code uses iced syntax highlighting (see `Style::code_block_background` for fallback blocks).
fn main() {}
```

Extra space between blocks comes from `.paragraph_spacing(20.0)`.

[Block link content uses .style_link_button()](https://example.com)
