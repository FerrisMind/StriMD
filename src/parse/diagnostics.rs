use std::fmt;

/// Which Markdown parser produced the final document blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseBackend {
    /// Primary pulldown-cmark path.
    Pulldown,
}

impl fmt::Display for ParseBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pulldown => f.write_str("pulldown"),
        }
    }
}

/// Parse-time diagnostics for the active Markdown backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseDiagnostics {
    pub backend: ParseBackend,
}

impl ParseDiagnostics {
    #[must_use]
    pub fn pulldown() -> Self {
        Self {
            backend: ParseBackend::Pulldown,
        }
    }
}

impl fmt::Display for ParseDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "backend={}", self.backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_reports_pulldown() {
        let diag = ParseDiagnostics::pulldown();
        assert_eq!(diag.to_string(), "backend=pulldown");
    }
}
