use crate::profile::ParseProfile;

/// Markdown parse and render policy.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    pub pulldown: pulldown_cmark::Options,
    pub raw_html: RawHtmlPolicy,
    pub legacy_fallback: LegacyFallbackPolicy,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self::for_profile(ParseProfile::GitHubPreview)
    }
}

impl ParseOptions {
    #[must_use]
    pub fn for_profile(profile: ParseProfile) -> Self {
        match profile {
            ParseProfile::GitHubPreview | ParseProfile::ChatStream => Self {
                pulldown: profile.pulldown_options(),
                raw_html: RawHtmlPolicy::Preserve,
                legacy_fallback: LegacyFallbackPolicy::ShadowCompare,
            },
            ParseProfile::StrictCommonMark => Self {
                pulldown: pulldown_cmark::Options::empty(),
                raw_html: RawHtmlPolicy::Escape,
                legacy_fallback: LegacyFallbackPolicy::Disabled,
            },
        }
    }
}

/// How raw HTML embedded in Markdown is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawHtmlPolicy {
    Preserve,
    Escape,
    StripUnsupported,
}

/// Whether comrak may shadow-compare or temporarily serve legacy output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyFallbackPolicy {
    Disabled,
    ShadowCompare,
    PreferLegacyUntilParity,
}
