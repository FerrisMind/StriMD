#![cfg_attr(
    all(feature = "no_iced", not(feature = "_iced_backend")),
    doc = include_str!("../docs/README_HEADLESS.md")
)]
#![cfg_attr(
    not(all(feature = "no_iced", not(feature = "_iced_backend"))),
    doc = include_str!("../README.md")
)]

//! # Feature contract
//!
//! Supported user-facing Cargo features:
//!
//! | Feature | Purpose |
//! |---------|---------|
//! | `no_iced` | Headless mode: disable the default iced backend |
//! | `static` | Full-document parse and HTML export |
//! | `stream` | Incremental LLM streaming via vendored mdstream |
//!
//! Implementation-detail features (`_iced_backend`, `_html_preprocess`,
//! `_rcdom_compat`) exist for migration only and are **not** part of the stable public contract.
//!
//! See [`docs/API.md`](https://github.com/FerrisMind/strimd/blob/main/docs/API.md) for the full
//! public API reference.

#[cfg(all(feature = "no_iced", feature = "_iced_backend"))]
compile_error!(
    "`no_iced` requires disabling StriMD default features: \
     use `default-features = false, features = [\"no_iced\", ...]`"
);

#[cfg(not(any(feature = "no_iced", feature = "_iced_backend")))]
compile_error!("Select either the default iced backend or explicit headless mode via `no_iced`.");

pub mod core;
pub mod html;
pub mod options;
pub mod parse;
pub mod profile;

#[cfg(all(feature = "_iced_backend", not(feature = "no_iced")))]
pub mod backends;

pub use core::{
    BlockContent, BlockId, BlockKind, BlockStatus, CompiledMarkdown, Document, HtmlFragmentError,
    ParseError, RenderBlock, RenderError, UnsupportedReason,
};
pub use options::{ParseOptions, RawHtmlPolicy};
pub use parse::{ParseBackend, ParseDiagnostics};
pub use profile::ParseProfile;

#[cfg(feature = "static")]
pub use core::markdown_to_html;

#[cfg(feature = "stream")]
pub use core::{PendingPolicy, StreamDocument, StreamOptions, StreamPatch, StreamUpdate};

#[cfg(feature = "static")]
pub use html::fragment::{HtmlAttr, HtmlFragment, HtmlNode, HtmlTag, NodeId};

// Iced API (default builds)
#[cfg(all(feature = "_iced_backend", not(feature = "no_iced")))]
pub use backends::iced::{
    DEFAULT_INLINE_CODE_BACKGROUND, DEFAULT_INLINE_CODE_FOREGROUND, ImageInfo, MarkState,
    MarkWidget, RubyMode, Style, UpdateMsg,
};

#[cfg(all(
    feature = "_iced_backend",
    feature = "static",
    not(feature = "no_iced")
))]
pub use backends::iced::{CodeBlock, MarkDocument, MarkSegment};
