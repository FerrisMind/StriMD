/// Named parse profiles for static and streaming Markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseProfile {
    GitHubPreview,
    ChatStream,
    StrictCommonMark,
}

impl ParseProfile {
    /// GFM extensions beyond strict CommonMark (tagfilter, extended autolinks).
    #[must_use]
    pub const fn uses_gfm_extensions(self) -> bool {
        matches!(self, Self::GitHubPreview | Self::ChatStream)
    }

    #[must_use]
    pub fn pulldown_options(self) -> pulldown_cmark::Options {
        match self {
            Self::StrictCommonMark => pulldown_cmark::Options::empty(),
            Self::GitHubPreview | Self::ChatStream => {
                pulldown_cmark::Options::ENABLE_TABLES
                    | pulldown_cmark::Options::ENABLE_TASKLISTS
                    | pulldown_cmark::Options::ENABLE_STRIKETHROUGH
                    | pulldown_cmark::Options::ENABLE_FOOTNOTES
                    | pulldown_cmark::Options::ENABLE_GFM
                    | pulldown_cmark::Options::ENABLE_MATH
                    | pulldown_cmark::Options::ENABLE_WIKILINKS
            }
        }
    }
}
