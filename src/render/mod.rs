//! Optional SVG renderers for math (RaTeX) and Mermaid diagrams.

pub mod svg_util;

#[cfg(feature = "math")]
pub mod ratex;

#[cfg(feature = "mermaid")]
pub mod mermaid;

pub use svg_util::SvgArtifact;

#[cfg(feature = "math")]
pub use ratex::{latex_to_svg, LatexCache};

#[cfg(feature = "mermaid")]
pub use mermaid::{mermaid_to_svg, MermaidCache};
