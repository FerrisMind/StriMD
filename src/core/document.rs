use crate::core::block::RenderBlock;
use crate::core::error::{ParseError, RenderError};
use crate::options::ParseOptions;
use crate::parse::pulldown;
use crate::profile::ParseProfile;

/// A fully parsed Markdown document as backend-agnostic blocks.
#[derive(Debug, Clone)]
pub struct Document {
    blocks: Vec<RenderBlock>,
    profile: ParseProfile,
}

impl Document {
    /// Parse `source` with the given profile.
    pub fn parse(source: &str, profile: ParseProfile) -> Result<Self, ParseError> {
        let options = ParseOptions::for_profile(profile);
        let blocks = pulldown::parse_blocks(source, profile, &options)?;
        Ok(Self { blocks, profile })
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

    /// Export the document as HTML (requires `static` feature).
    #[cfg(feature = "static")]
    pub fn to_html(&self) -> Result<String, RenderError> {
        crate::html::writer::blocks_to_html(&self.blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::block::BlockKind;

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
    fn to_html_exports_headings() {
        let doc = Document::parse("# Hi", ParseProfile::GitHubPreview).expect("parse");
        let html = doc.to_html().expect("html");
        assert!(html.contains("<h1>"));
    }
}
