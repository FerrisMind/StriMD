//! Optional SVG renderers for math (RaTeX) and Mermaid diagrams.

pub mod svg_util;

#[cfg(feature = "math")]
pub mod ratex;

#[cfg(feature = "mermaid")]
pub mod mermaid;

pub use svg_util::SvgArtifact;

#[cfg(feature = "math")]
pub use ratex::{LatexCache, latex_to_svg};

#[cfg(feature = "mermaid")]
pub use mermaid::{MermaidCache, mermaid_to_svg};
