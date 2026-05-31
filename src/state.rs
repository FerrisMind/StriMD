use std::collections::{HashMap, HashSet};

use html5ever::{ParseOpts, tendril::TendrilSink};
use markup5ever_rcdom::RcDom;

use crate::structs::{UpdateMsg, UpdateMsgKind};

/// The state of the document.
///
/// - Put this in your Application struct.
/// - Use [`Self::with_html`] and [`Self::with_html_and_markdown`]
///   functions to create this.
/// - Create a new one if the document changes
///
/// ```no_run
/// # use frostmark::MarkState;
/// # const YOUR_TEXT: &str = "";
/// # fn e() { let m =
/// MarkState::with_html_and_markdown(YOUR_TEXT)
/// # ;
/// // or if you just want HTML
/// # let m =
/// MarkState::with_html(YOUR_TEXT)
/// # ; }
/// ```
pub struct MarkState {
    pub(crate) dom: RcDom,

    pub(crate) dropdown_state: HashMap<usize, bool>,
}

impl MarkState {
    /// Processes documents containing **pure HTML**,
    /// without any Markdown support.
    ///
    /// Use this if you prioritize performance and
    /// don't need Markdown support,
    /// or if you want to avoid potential artifacts
    /// from mixing HTML and Markdown.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // Will never panic
    pub fn with_html(input: &str) -> Self {
        let dom = html5ever::parse_document(RcDom::default(), ParseOpts::default())
            .from_utf8()
            .read_from(&mut input.as_bytes())
            // Will not panic as reading from &[u8] cannot fail
            .unwrap();

        let mut dropdown_state = HashMap::new();
        let mut dropdown_counter = 0;
        find_state(&dom.document, &mut dropdown_state, &mut dropdown_counter);

        Self {
            dom,
            dropdown_state,
        }
    }

    /// Processes documents containing both
    /// **HTML and Markdown** (or a mix of both).
    ///
    /// Use this method when you need to support
    /// Markdown formatting. However, note that
    /// it may introduce formatting bugs when
    /// dealing with pure HTML documents.
    #[must_use]
    #[cfg(feature = "markdown")]
    pub fn with_html_and_markdown(input: &str) -> Self {
        let html = comrak::markdown_to_html(
            input,
            &comrak::Options {
                extension: comrak::options::Extension {
                    strikethrough: true,
                    cjk_friendly_emphasis: true,
                    tasklist: true,
                    superscript: true,
                    subscript: true,
                    underline: true,
                    table: true,
                    ..Default::default()
                },
                parse: comrak::options::Parse::default(),
                render: comrak::options::Render {
                    // Our renderer doesn't have the
                    // vulnerabilities of a browser
                    r#unsafe: true,
                    ..Default::default()
                },
            },
        );

        Self::with_html(&html)
    }

    /// Processes documents containing **pure Markdown**,
    /// filtering out any HTML content.
    ///
    /// Useful for things like messaging apps.
    #[must_use]
    #[cfg(feature = "markdown")]
    pub fn with_markdown_only(input: &str) -> Self {
        let mut out = String::new();
        _ = comrak::html::escape(&mut out, input);
        Self::with_html_and_markdown(&out)
    }

    /// Updates the internal state of the document.
    ///
    /// Call this method after receiving an update message
    /// from [`crate::MarkWidget::on_updating_state`].
    pub fn update(&mut self, action: UpdateMsg) {
        let UpdateMsgKind::DetailsToggle(id, action) = action.kind;
        self.dropdown_state.insert(id, action);
    }

    /// Retrieves all image URLs that need to be loaded, returned as a [`HashSet<String>`].
    ///
    /// This method gathers all image URLs in the document, which you can:
    /// 1. Download somehow (pass to an async downloader maybe?)
    /// 2. Store using, if SVG image, `iced::widget::svg::Handle::from_memory`.
    ///    - For normal images: `iced::widget::image::Handle::from_bytes`.
    /// 3. Handle the rendering of these images via [`crate::MarkWidget::on_drawing_image`].
    #[must_use]
    pub fn find_image_links(&self) -> HashSet<String> {
        let mut storage = HashSet::new();
        find_image_links(&self.dom.document, &mut storage);
        storage
    }
}

impl Default for MarkState {
    fn default() -> Self {
        Self::with_html("")
    }
}

fn find_state(
    node: &markup5ever_rcdom::Node,
    dropdown_state: &mut HashMap<usize, bool>,
    dropdown_counter: &mut usize,
) {
    let borrow = node.children.borrow();
    match &node.data {
        markup5ever_rcdom::NodeData::Element { name, .. } if &name.local == "details" => {
            dropdown_state.insert(*dropdown_counter, false);
            *dropdown_counter += 1;
            for child in &*borrow {
                find_state(child, dropdown_state, dropdown_counter);
            }
        }
        _ => {
            for child in &*borrow {
                find_state(child, dropdown_state, dropdown_counter);
            }
        }
    }
}

fn find_image_links(node: &markup5ever_rcdom::Node, storage: &mut HashSet<String>) {
    let borrow = node.children.borrow();
    match &node.data {
        markup5ever_rcdom::NodeData::Element { name, attrs, .. } if &name.local == "img" => {
            let attrs = attrs.borrow();
            if let Some(attr) = attrs.iter().find(|attr| &*attr.name.local == "src") {
                let url = &*attr.value;
                if !url.is_empty() {
                    storage.insert(url.to_owned());
                }
            }
        }
        _ => {
            for child in &*borrow {
                find_image_links(child, storage);
            }
        }
    }
}
