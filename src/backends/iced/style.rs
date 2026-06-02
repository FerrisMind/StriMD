/// The style of a [`crate::MarkWidget`]
/// that affects how it's rendered.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Style {
    /// Color of regular text.
    pub text_color: Option<iced::Color>,
    /// Color of link **text**.
    ///
    /// Default: `#5A6B9E`
    pub link_color: Option<iced::Color>,
    /// Background color for text highlights (`<mark>` element).
    ///
    /// Default: `#F7D84B`
    pub highlight_color: Option<iced::Color>,
    /// Background color for inline `<code>` snippets.
    pub inline_code_background: Option<iced::Color>,
    /// Text color for inline `<code>` snippets.
    pub inline_code_color: Option<iced::Color>,
    /// Background color for block `<pre><code>` sections.
    pub code_block_background: Option<iced::Color>,
}

/// Subtle inline-code pill background (readable on light and dark UI chrome).
pub const DEFAULT_INLINE_CODE_BACKGROUND: iced::Color =
    iced::Color::from_rgba8(100, 106, 120, 0.16);

/// Subtle inline-code border, using the same neutral palette as the background.
pub const DEFAULT_INLINE_CODE_BORDER: iced::Color = iced::Color::from_rgba8(100, 106, 120, 0.28);

/// Neutral inline-code text when no [`Style::inline_code_color`] is set (markdown path).
pub const DEFAULT_INLINE_CODE_FOREGROUND: iced::Color = iced::Color::from_rgb8(0x58, 0x60, 0x6E);
