use crate::profile::ParseProfile;

/// Markdown parse and render policy.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    pub pulldown: pulldown_cmark::Options,
    pub raw_html: RawHtmlPolicy,
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
            },
            ParseProfile::StrictCommonMark => Self {
                pulldown: pulldown_cmark::Options::empty(),
                raw_html: RawHtmlPolicy::Escape,
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
