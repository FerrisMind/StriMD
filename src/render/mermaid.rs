//! Mermaid diagram → SVG via mermaid-rs-renderer.

use std::collections::HashMap;
use std::sync::Arc;

use mermaid_rs_renderer::{RenderOptions, render_with_options};

use crate::core::error::RenderError;
use crate::render::svg_util::SvgArtifact;

const CACHE_MAX_ENTRIES: usize = 64;

/// Memoized Mermaid renders keyed by source text.
#[derive(Debug, Default)]
pub struct MermaidCache {
    entries: HashMap<Arc<str>, Arc<SvgArtifact>>,
}

impl MermaidCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&mut self, source: &str) -> Result<Arc<SvgArtifact>, RenderError> {
        let key = Arc::from(source);
        if let Some(hit) = self.entries.get(&key) {
            return Ok(Arc::clone(hit));
        }
        let artifact = Arc::new(mermaid_to_svg(source)?);
        if self.entries.len() >= CACHE_MAX_ENTRIES {
            self.entries.clear();
        }
        self.entries.insert(key, Arc::clone(&artifact));
        Ok(artifact)
    }
}

/// Render Mermaid source to SVG.
pub fn mermaid_to_svg(source: &str) -> Result<SvgArtifact, RenderError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(RenderError::new("empty Mermaid source"));
    }

    let svg = render_with_options(trimmed, RenderOptions::modern())
        .map_err(|e| RenderError::new(format!("mermaid render: {e}")))?;

    if !svg.contains("<svg") {
        return Err(RenderError::new("mermaid produced no SVG root"));
    }
    Ok(SvgArtifact::from_svg_string(svg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_flowchart() {
        let src = "flowchart LR\n  A-->B\n";
        let art = mermaid_to_svg(src).expect("svg");
        assert!(art.bytes.starts_with(b"<svg"));
    }
}
