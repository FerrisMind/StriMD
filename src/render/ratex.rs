//! LaTeX → SVG via RaTeX (KaTeX-compatible).

use std::collections::HashMap;
use std::sync::Arc;

use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_parser::parser::parse;
use ratex_svg::{SvgOptions, render_to_svg};

use crate::core::error::RenderError;
use crate::render::svg_util::SvgArtifact;

const CACHE_MAX_ENTRIES: usize = 256;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CacheKey {
    latex: Arc<str>,
    display: bool,
}

/// Memoized RaTeX renders keyed by LaTeX source and display mode.
#[derive(Debug, Default)]
pub struct LatexCache {
    entries: HashMap<CacheKey, Arc<SvgArtifact>>,
}

impl LatexCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&mut self, latex: &str, display: bool) -> Result<Arc<SvgArtifact>, RenderError> {
        let key = CacheKey {
            latex: Arc::from(latex),
            display,
        };
        if let Some(hit) = self.entries.get(&key) {
            return Ok(Arc::clone(hit));
        }
        let artifact = Arc::new(latex_to_svg(latex, display)?);
        if self.entries.len() >= CACHE_MAX_ENTRIES {
            self.entries.clear();
        }
        self.entries.insert(key, Arc::clone(&artifact));
        Ok(artifact)
    }
}

/// Render LaTeX to a self-contained SVG string (embedded KaTeX fonts).
pub fn latex_to_svg(latex: &str, _display: bool) -> Result<SvgArtifact, RenderError> {
    let trimmed = latex.trim();
    if trimmed.is_empty() {
        return Err(RenderError::new("empty LaTeX input"));
    }

    let ast = parse(trimmed).map_err(|e| RenderError::new(format!("ratex parse: {e:?}")))?;
    let lbox = layout(&ast, &LayoutOptions::default());
    let dl = to_display_list(&lbox);

    let mut opts = SvgOptions::default();
    opts.embed_glyphs = true;
    // Slightly below default 40 — final size is matched to iced `text_size` at draw time.
    opts.font_size = if _display { 28.0 } else { 20.0 };
    opts.padding = if _display { 8.0 } else { 2.0 };

    let svg = render_to_svg(&dl, &opts);
    if !svg.contains("<svg") {
        return Err(RenderError::new("ratex produced no SVG root"));
    }
    Ok(SvgArtifact::from_svg_string(svg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_simple_fraction() {
        let art = latex_to_svg(r"\frac{1}{2}", true).expect("svg");
        assert!(art.bytes.starts_with(b"<svg"));
        assert!(art.width > 0.0 && art.height > 0.0);
    }

    #[test]
    fn cache_returns_same_arc() {
        let mut cache = LatexCache::new();
        let a = cache.render("x^2", false).expect("a");
        let b = cache.render("x^2", false).expect("b");
        assert!(Arc::ptr_eq(&a, &b));
    }
}
