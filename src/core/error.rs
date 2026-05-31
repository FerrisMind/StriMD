use std::fmt;

/// Markdown or block-model parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// HTML fragment construction or traversal failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlFragmentError {
    message: String,
}

impl HtmlFragmentError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for HtmlFragmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HtmlFragmentError {}

/// Static export or backend render failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError {
    message: String,
}

impl RenderError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RenderError {}

/// Why a construct is not supported on the current path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsupportedReason {
    HtmlTag(String),
    MarkdownConstruct(String),
    Policy(String),
}
