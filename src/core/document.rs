use crate::core::block::RenderBlock;
use crate::core::error::ParseError;
use crate::options::ParseOptions;
use crate::parse::diagnostics::ParseDiagnostics;
use crate::parse::legacy_fallback::{self, LegacyFallbackReport};
use crate::parse::pulldown;
use crate::profile::ParseProfile;

/// A fully parsed Markdown document as backend-agnostic blocks.
#[derive(Debug, Clone)]
pub struct Document {
    blocks: Vec<RenderBlock>,
    profile: ParseProfile,
    diagnostics: ParseDiagnostics,
}

impl Document {
    /// Parse `source` with the given profile.
    pub fn parse(source: &str, profile: ParseProfile) -> Result<Self, ParseError> {
        Self::parse_with_options(source, profile, &ParseOptions::for_profile(profile))
    }

    /// Parse with explicit options and return legacy migration diagnostics.
    pub fn parse_with_options(
        source: &str,
        profile: ParseProfile,
        options: &ParseOptions,
    ) -> Result<Self, ParseError> {
        let blocks = pulldown::parse_blocks(source, profile, options)?;
        let (blocks, fallback_report) =
            legacy_fallback::apply_legacy_policy(source, blocks, options.legacy_fallback)?;
        Ok(Self {
            blocks,
            profile,
            diagnostics: ParseDiagnostics::from_fallback_report(fallback_report),
        })
    }

    /// All committed render blocks in document order.
    #[must_use]
    pub fn blocks(&self) -> &[RenderBlock] {
        &self.blocks
    }

    /// Profile used to parse this document.
    #[must_use]
    pub fn profile(&self) -> ParseProfile {
        self.profile
    }

    /// Parse diagnostics: active backend and migration flags.
    #[must_use]
    pub fn diagnostics(&self) -> ParseDiagnostics {
        self.diagnostics
    }

    /// Active Markdown parser backend for this document.
    #[must_use]
    pub fn parse_backend(&self) -> crate::parse::ParseBackend {
        self.diagnostics.backend
    }

    /// Whether comrak fallback was used during parse (migration builds only).
    #[must_use]
    pub fn legacy_fallback_used(&self) -> bool {
        self.diagnostics.legacy_fallback_used()
    }

    /// Whether pulldown and comrak HTML differed during shadow compare.
    #[must_use]
    pub fn shadow_mismatch(&self) -> bool {
        self.diagnostics.shadow_mismatch()
    }

    /// Full legacy migration report from the last parse.
    #[must_use]
    pub fn fallback_report(&self) -> LegacyFallbackReport {
        self.diagnostics.fallback
    }

    /// Export the document as HTML (requires `static` feature).
    #[cfg(feature = "static")]
    pub fn to_html(&self) -> Result<String, crate::core::error::RenderError> {
        crate::html::writer::blocks_to_html(&self.blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::block::BlockKind;
    #[test]
    fn hello_example_text_parses_nonempty() {
        const YOUR_TEXT: &str = r"
# Hello, World!
This is a markdown renderer <b>with inline HTML support!</b>
- You can mix and match markdown and HTML together
<hr>

```rust
App {
    state: MarkState::with_html_and_markdown(YOUR_TEXT)
}
```

## Note

> <b>Fun fact</b>: This is all built on top of existing iced widgets.
>
> No new widgets were made for this.
";
        let doc = Document::parse(YOUR_TEXT, ParseProfile::GitHubPreview).expect("parse");
        assert!(
            !doc.blocks().is_empty(),
            "expected blocks, got {}",
            doc.blocks().len()
        );
    }

    #[test]
    fn document_blocks_are_stable() {
        let doc = Document::parse("# Hi\n\nParagraph.", ParseProfile::GitHubPreview)
            .expect("parse");
        let ids: Vec<_> = doc.blocks().iter().map(|b| b.id).collect();
        assert_eq!(ids.len(), doc.blocks().len());
        assert!(doc.blocks().iter().any(|b| b.kind == BlockKind::Heading));
    }

    #[cfg(feature = "static")]
    #[test]
    fn raw_details_fixture_exports_children() {
        let source = include_str!("../../tests/fixtures/raw_details.md");
        let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
        let html = doc.to_html().expect("html");
        assert!(
            html.contains("summary") || html.contains("Summary"),
            "html: {html}"
        );
    }

    #[cfg(feature = "static")]
    #[test]
    fn to_html_exports_headings() {
        let doc = Document::parse("# Hi", ParseProfile::GitHubPreview).expect("parse");
        let html = doc.to_html().expect("html");
        assert!(html.contains("<h1>"));
    }

    #[test]
    fn diagnostics_expose_pulldown_backend_by_default() {
        let doc = Document::parse("# Hi", ParseProfile::GitHubPreview).expect("parse");
        assert_eq!(doc.parse_backend(), crate::parse::ParseBackend::Pulldown);
        assert!(!doc.legacy_fallback_used());
    }

    #[cfg(feature = "static")]
    #[test]
    fn strip_unsupported_rejects_script_html() {
        use crate::options::RawHtmlPolicy;

        let mut options = ParseOptions::for_profile(ParseProfile::GitHubPreview);
        options.raw_html = RawHtmlPolicy::StripUnsupported;
        let doc = Document::parse_with_options(
            "<script>alert(1)</script>",
            ParseProfile::GitHubPreview,
            &options,
        )
        .expect("parse");
        assert!(doc.blocks().iter().any(|b| matches!(
            b.content,
            crate::core::block::BlockContent::Unsupported { .. }
        )));
        let html = doc.to_html().expect("html");
        assert!(!html.contains("<script"));
    }

    #[cfg(all(feature = "_legacy_comrak", feature = "static"))]
    #[test]
    fn parse_reports_shadow_mismatch_without_using_fallback() {
        let source = "See [[WikiPage]] for details.\n";
        let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
        assert!(doc.shadow_mismatch());
        assert!(!doc.legacy_fallback_used());
    }

    #[cfg(feature = "_legacy_comrak")]
    #[test]
    fn prefer_legacy_policy_can_be_selected_via_options() {
        use crate::options::LegacyFallbackPolicy;

        let source = "See [[WikiPage]] for details.\n";
        let mut options = ParseOptions::for_profile(ParseProfile::GitHubPreview);
        options.legacy_fallback = LegacyFallbackPolicy::PreferLegacyUntilParity;
        let doc = Document::parse_with_options(source, ParseProfile::GitHubPreview, &options)
            .expect("parse");
        assert!(doc.legacy_fallback_used());
    }
}
