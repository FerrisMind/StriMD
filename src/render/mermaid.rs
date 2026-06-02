//! Mermaid diagram → SVG via mermaid-rs-renderer.

use std::collections::HashMap;
use std::sync::Arc;

use mermaid_rs_renderer::{
    RenderOptions, compute_layout, parse_mermaid, render_svg, render_with_options,
};

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

    let options = RenderOptions::modern();
    let svg = match render_with_options(trimmed, options.clone()) {
        Ok(svg) => svg,
        Err(err) if should_retry_with_public_pipeline(trimmed, &err.to_string()) => {
            render_with_public_pipeline(trimmed, &options)
                .map_err(|e| RenderError::new(format!("mermaid render: {e}")))?
        }
        Err(err) => return Err(RenderError::new(format!("mermaid render: {err}"))),
    };

    if !svg.contains("<svg") {
        return Err(RenderError::new("mermaid produced no SVG root"));
    }
    Ok(SvgArtifact::from_svg_string(svg))
}

fn render_with_public_pipeline(input: &str, options: &RenderOptions) -> Result<String, String> {
    let parsed = parse_mermaid(input).map_err(|err| err.to_string())?;
    let layout = compute_layout(&parsed.graph, &options.theme, &options.layout);
    Ok(render_svg(&layout, &options.theme, &options.layout))
}

fn should_retry_with_public_pipeline(input: &str, error: &str) -> bool {
    let lower_input = input.to_ascii_lowercase();
    lower_input.starts_with("sequencediagram")
        && lower_input.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed == "alt"
                || trimmed.starts_with("alt ")
                || trimmed == "opt"
                || trimmed.starts_with("opt ")
                || trimmed == "loop"
                || trimmed.starts_with("loop ")
                || trimmed == "par"
                || trimmed.starts_with("par ")
                || trimmed == "rect"
                || trimmed.starts_with("rect ")
                || trimmed == "critical"
                || trimmed.starts_with("critical ")
                || trimmed == "break"
                || trimmed.starts_with("break ")
                || trimmed == "box"
                || trimmed.starts_with("box ")
        })
        && error.contains("matching subgraph")
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

    #[test]
    fn sequence_alt_uses_wrapper_fallback() {
        let src = "sequenceDiagram\nA->>B: req\nalt ok\nB-->>A: yes\nend\n";
        let art = mermaid_to_svg(src).expect("sequence svg");
        assert!(art.bytes.starts_with(b"<svg"));
    }
}
