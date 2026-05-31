use crate::core::block::RenderBlock;
use crate::core::error::ParseError;
use crate::options::LegacyFallbackPolicy;

/// Diagnostics from optional comrak shadow-compare or fallback during migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LegacyFallbackReport {
    /// Pulldown HTML export differed from comrak for the same source.
    pub shadow_mismatch: bool,
    /// Legacy comrak output was selected for one or more blocks.
    pub legacy_fallback_used: bool,
}

impl LegacyFallbackReport {
    #[must_use]
    pub fn used_legacy(&self) -> bool {
        self.legacy_fallback_used
    }
}

/// Apply legacy comrak policy after pulldown parsing.
///
/// Returns possibly rewritten blocks and a migration report.
#[cfg(feature = "_legacy_comrak")]
pub fn apply_legacy_policy(
    source: &str,
    blocks: Vec<RenderBlock>,
    policy: LegacyFallbackPolicy,
) -> Result<(Vec<RenderBlock>, LegacyFallbackReport), ParseError> {
    match policy {
        LegacyFallbackPolicy::Disabled => Ok((blocks, LegacyFallbackReport::default())),
        LegacyFallbackPolicy::ShadowCompare => {
            let report = shadow_compare(source, &blocks)?;
            Ok((blocks, report))
        }
        LegacyFallbackPolicy::PreferLegacyUntilParity => {
            let report = shadow_compare(source, &blocks)?;
            if report.shadow_mismatch {
                let legacy_blocks = legacy_html_blocks(source)?;
                Ok((
                    legacy_blocks,
                    LegacyFallbackReport {
                        shadow_mismatch: true,
                        legacy_fallback_used: true,
                    },
                ))
            } else {
                Ok((blocks, report))
            }
        }
    }
}

#[cfg(not(feature = "_legacy_comrak"))]
pub fn apply_legacy_policy(
    _source: &str,
    blocks: Vec<RenderBlock>,
    policy: LegacyFallbackPolicy,
) -> Result<(Vec<RenderBlock>, LegacyFallbackReport), ParseError> {
    if matches!(policy, LegacyFallbackPolicy::PreferLegacyUntilParity) {
        return Err(ParseError::new(
            "legacy fallback requested but `_legacy_comrak` feature is disabled",
        ));
    }
    Ok((blocks, LegacyFallbackReport::default()))
}

#[cfg(all(feature = "_legacy_comrak", feature = "static"))]
fn shadow_compare(source: &str, blocks: &[RenderBlock]) -> Result<LegacyFallbackReport, ParseError> {
    use crate::html::writer;

    let pulldown_html = writer::blocks_to_html(blocks).map_err(|e| ParseError::new(e.to_string()))?;
    let legacy_html = crate::parse::comrak_migration::markdown_to_html(source);
    Ok(LegacyFallbackReport {
        shadow_mismatch: normalize_html(&pulldown_html) != normalize_html(&legacy_html),
        legacy_fallback_used: false,
    })
}

#[cfg(all(feature = "_legacy_comrak", not(feature = "static")))]
fn shadow_compare(_source: &str, _blocks: &[RenderBlock]) -> Result<LegacyFallbackReport, ParseError> {
    Ok(LegacyFallbackReport::default())
}

#[cfg(feature = "_legacy_comrak")]
fn legacy_html_blocks(source: &str) -> Result<Vec<RenderBlock>, ParseError> {
    use std::sync::Arc;

    use crate::core::block::{BlockContent, BlockKind, BlockStatus};
    use crate::core::ids::BlockId;

    let html = crate::parse::comrak_migration::markdown_to_html(source);
    Ok(vec![RenderBlock {
        id: BlockId::new(1),
        status: BlockStatus::Committed,
        kind: BlockKind::Unknown,
        source: Arc::from(source),
        content: BlockContent::LegacyHtml(html),
    }])
}

#[cfg(all(feature = "_legacy_comrak", feature = "static"))]
fn normalize_html(html: &str) -> String {
    html.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(all(test, feature = "_legacy_comrak", feature = "static"))]
mod tests {
    use super::*;
    use crate::core::block::BlockContent;
    use crate::core::document::Document;
    use crate::options::ParseOptions;
    use crate::profile::ParseProfile;

    #[test]
    fn shadow_compare_detects_mismatch_for_wikilinks() {
        let source = "See [[WikiPage]] for details.\n";
        let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
        let report = shadow_compare(source, doc.blocks()).expect("compare");
        assert!(report.shadow_mismatch);
        assert!(!report.legacy_fallback_used);
    }

    #[test]
    fn prefer_legacy_replaces_blocks_when_mismatch() {
        let source = "See [[WikiPage]] for details.\n";
        let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
        let (blocks, report) = apply_legacy_policy(
            source,
            doc.blocks().to_vec(),
            LegacyFallbackPolicy::PreferLegacyUntilParity,
        )
        .expect("fallback");
        assert!(report.legacy_fallback_used);
        assert!(matches!(
            blocks[0].content,
            BlockContent::LegacyHtml(_)
        ));
    }

    #[test]
    fn disabled_policy_leaves_blocks_unchanged() {
        let source = "# Hi";
        let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
        let original_len = doc.blocks().len();
        let (blocks, report) = apply_legacy_policy(
            source,
            doc.blocks().to_vec(),
            LegacyFallbackPolicy::Disabled,
        )
        .expect("fallback");
        assert!(!report.shadow_mismatch);
        assert_eq!(blocks.len(), original_len);
    }

    #[test]
    fn parse_options_default_enables_shadow_compare() {
        let opts = ParseOptions::for_profile(ParseProfile::GitHubPreview);
        assert_eq!(opts.legacy_fallback, LegacyFallbackPolicy::ShadowCompare);
    }

    #[test]
    fn fixture_fallback_usage_is_observable() {
        use crate::core::document::Document;
        use crate::parse::diagnostics::ParseBackend;

        let source = "# Stable\n";
        let doc = Document::parse(source, ParseProfile::GitHubPreview).expect("parse");
        assert_eq!(doc.parse_backend(), ParseBackend::Pulldown);
        assert!(!doc.diagnostics().legacy_fallback_used());
        assert_eq!(doc.diagnostics().to_string(), "backend=pulldown");
    }
}
