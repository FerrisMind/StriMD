use std::fmt;

use crate::parse::legacy_fallback::LegacyFallbackReport;

/// Which Markdown parser produced the final document blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseBackend {
    /// Primary pulldown-cmark path.
    Pulldown,
    /// Legacy comrak output selected by migration fallback policy.
    LegacyComrak,
}

impl fmt::Display for ParseBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pulldown => f.write_str("pulldown"),
            Self::LegacyComrak => f.write_str("legacy_comrak"),
        }
    }
}

/// Parse-time diagnostics: active backend and optional comrak migration report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseDiagnostics {
    pub backend: ParseBackend,
    pub fallback: LegacyFallbackReport,
}

impl ParseDiagnostics {
    #[must_use]
    pub fn from_fallback_report(fallback: LegacyFallbackReport) -> Self {
        let backend = if fallback.legacy_fallback_used {
            ParseBackend::LegacyComrak
        } else {
            ParseBackend::Pulldown
        };
        Self { backend, fallback }
    }

    #[must_use]
    pub fn pulldown() -> Self {
        Self::from_fallback_report(LegacyFallbackReport::default())
    }

    #[must_use]
    pub fn legacy_fallback_used(&self) -> bool {
        self.fallback.legacy_fallback_used
    }

    #[must_use]
    pub fn shadow_mismatch(&self) -> bool {
        self.fallback.shadow_mismatch
    }
}

impl Default for ParseDiagnostics {
    fn default() -> Self {
        Self::pulldown()
    }
}

impl fmt::Display for ParseDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "backend={}", self.backend)?;
        if self.fallback.shadow_mismatch {
            f.write_str(", shadow_mismatch")?;
        }
        if self.fallback.legacy_fallback_used {
            f.write_str(", legacy_fallback_used")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::legacy_fallback::LegacyFallbackReport;

    #[test]
    fn backend_reflects_legacy_selection() {
        let diag = ParseDiagnostics::from_fallback_report(LegacyFallbackReport {
            shadow_mismatch: true,
            legacy_fallback_used: true,
        });
        assert_eq!(diag.backend, ParseBackend::LegacyComrak);
        assert!(diag.shadow_mismatch());
    }

    #[test]
    fn display_includes_backend_and_flags() {
        let diag = ParseDiagnostics::from_fallback_report(LegacyFallbackReport {
            shadow_mismatch: true,
            legacy_fallback_used: false,
        });
        let text = diag.to_string();
        assert!(text.contains("pulldown"));
        assert!(text.contains("shadow_mismatch"));
    }
}
