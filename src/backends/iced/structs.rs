use std::{ops::Add, sync::Arc};

use bitflags::bitflags;
use iced::{Element, Font, widget};

use crate::core::ids::BlockId;

use super::state::MarkState;

#[derive(Debug, Default, Clone, Copy)]
pub struct ChildData {
    pub heading_weight: u16,
    pub flags: ChildDataFlags,
    pub alignment: Option<ChildAlignment>,

    pub li_ordered_number: Option<usize>,
}

impl ChildData {
    pub fn heading(mut self, weight: u16) -> Self {
        self.heading_weight = weight;
        self
    }

    pub fn insert(mut self, flags: ChildDataFlags) -> Self {
        self.flags.insert(flags);
        self
    }

    pub fn ordered_from(mut self, start: usize) -> Self {
        self.li_ordered_number = Some(start);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChildAlignment {
    Center,
    Right,
}

impl From<ChildAlignment> for iced::Alignment {
    fn from(val: ChildAlignment) -> Self {
        match val {
            ChildAlignment::Center => Self::Center,
            ChildAlignment::Right => Self::End,
        }
    }
}

impl ChildAlignment {
    pub(crate) fn to_horizontal(self) -> iced::alignment::Horizontal {
        match self {
            ChildAlignment::Center => iced::alignment::Horizontal::Center,
            ChildAlignment::Right => iced::alignment::Horizontal::Right,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ChildDataFlags: u16 {
        const BOLD = 1 << 0;
        const ITALIC = 1 << 1;
        const UNDERLINE = 1 << 2;
        const STRIKETHROUGH = 1 << 3;
        const KEEP_WHITESPACE = 1 << 4;
        const MONOSPACE = 1 << 5;
        const SKIP_SUMMARY = 1 << 6;
        const HIGHLIGHT = 1 << 7;
        const INSIDE_RUBY = 1 << 8;
    }
}

/// The message that's sent when a widget is updated.
///
/// See [`MarkWidget::on_updating_state`] for more info.
#[derive(Debug, Clone)]
pub struct UpdateMsg {
    pub(crate) kind: UpdateMsgKind,
}

impl UpdateMsg {
    /// Checks if the message is a request to copy code/text to the clipboard.
    #[must_use]
    pub fn as_copy_to_clipboard(&self) -> Option<&str> {
        match &self.kind {
            UpdateMsgKind::CopyToClipboard(text) => Some(text),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UpdateMsgKind {
    DetailsToggle(usize, bool),
    CopyToClipboard(Arc<str>),
}

type FClickLink<M> = Box<dyn Fn(String) -> M>;
type FDrawImage<'a, M, T> = Box<dyn Fn(ImageInfo) -> Element<'static, M, T> + 'a>;
type FDrawPreBlock<'a, M, T> = Box<dyn Fn(Element<'a, M, T>) -> Element<'a, M, T> + 'a>;
pub(crate) type FUpdate<M> = Arc<dyn Fn(UpdateMsg) -> M>;
pub(crate) type FStyleLinkButton<T> =
    Arc<dyn Fn(&T, widget::button::Status) -> widget::button::Style + 'static>;
pub(crate) type FGitHubAlertIcon = Arc<dyn Fn(&str) -> String + 'static>;

/// The widget to be constructed every frame.
///
/// ```no_run
/// // inside your view function
/// # use strimd::{MarkWidget, MarkState};
/// # struct E { mark_state: MarkState }
/// # #[derive(Clone)]
/// # enum Message {}
/// # impl E { fn e(&self) {
/// # let m: MarkWidget<'_, Message>  =
/// MarkWidget::new(&self.mark_state)
/// # ; } }
/// ```
///
/// You can put this inside a [`iced::widget::Container`]
/// or [`iced::widget::Column`] or anywhere you like.
/// To render this, call `Into<iced::Element<_>>`.
///
/// There are many methods you can call on this to customize its behavior.
pub struct MarkWidget<'a, Message, Theme = iced::Theme> {
    pub(crate) state: &'a MarkState,

    pub(crate) font: Font,
    pub(crate) font_mono: Font,
    pub(crate) style: Option<crate::Style>,
    pub(crate) text_size: f32,
    pub(crate) heading_scale: f32,

    pub(crate) fn_clicking_link: Option<FClickLink<Message>>,
    pub(crate) fn_drawing_image: Option<FDrawImage<'a, Message, Theme>>,
    pub(crate) fn_drawing_pre_block: Option<FDrawPreBlock<'a, Message, Theme>>,
    pub(crate) fn_update: Option<FUpdate<Message>>,
    pub(crate) fn_style_link_button: Option<FStyleLinkButton<Theme>>,
    pub(crate) fn_github_alert_icon: Option<FGitHubAlertIcon>,

    pub(crate) paragraph_spacing: Option<f32>,

    pub(crate) current_dropdown_id: usize,
    pub(crate) current_block_id: Option<BlockId>,

    pub(crate) ruby_mode: RubyMode,
}

impl<'a, M: 'a, T: 'a> MarkWidget<'a, M, T> {
    /// Creates a new [`MarkWidget`] from the given [`MarkState`].
    ///
    /// The state would usually be stored inside your main application state struct.
    #[must_use]
    pub fn new(state: &'a MarkState) -> Self {
        Self {
            state,
            font: Font::DEFAULT,
            font_mono: Font::MONOSPACE,
            fn_clicking_link: None,
            fn_drawing_image: None,
            fn_drawing_pre_block: None,
            fn_update: None,
            fn_style_link_button: None,
            fn_github_alert_icon: None,
            style: None,
            current_dropdown_id: 0,
            current_block_id: None,
            text_size: 16.0,
            heading_scale: 1.0,
            paragraph_spacing: None,
            ruby_mode: RubyMode::default(),
        }
    }

    /// Sets the default font when rendering documents.
    ///
    /// > **Note**: Variations of this font will be
    /// > used for bold and italic.
    #[must_use]
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the monospaced font used
    /// for rendering codeblocks and code snippets.
    #[must_use]
    pub fn font_mono(mut self, font: Font) -> Self {
        self.font_mono = font;
        self
    }

    /// Sets the size of text.
    ///
    /// Headings will be scaled as a multiple of this,
    /// although you can fine-tune their relative scale
    /// using [`MarkWidget::heading_scale`].
    #[must_use]
    pub fn text_size(mut self, size: impl Into<iced::Pixels>) -> Self {
        self.text_size = size.into().0;
        self
    }

    /// Sets the scaling factor of headings relative to text,
    /// as a scale from **0.0 to 1.0 to ...**.
    ///
    /// This is relative to the base size of the text
    /// which you can set using [`MarkWidget::text_size`].
    ///
    /// If it's
    /// - 0.0: headings won't be bigger than regular text.
    /// - 0.x: headings will be slightly bigger
    /// - 1.0: default scale (somewhat bigger)
    /// - above 1.0: headings will be **much bigger** than regular text.
    ///
    /// For reference, in scale 1.0, `<h1>` headings are 1.8x bigger than regular text.
    #[must_use]
    pub fn heading_scale(mut self, scale: f32) -> Self {
        debug_assert!(scale >= 0.0);
        if scale >= 0.0 {
            self.heading_scale = scale;
        }
        self
    }

    /// When clicking a link, send a message to handle it.
    ///
    /// ```no_run
    /// # use strimd::{MarkWidget, MarkState};
    /// # #[derive(Clone)]
    /// # enum Message { OpenLink(String) }
    /// # struct E {mark_state: MarkState} impl E { fn e(&self) {
    /// # let m: MarkWidget<'_, Message> =
    /// MarkWidget::new(&self.mark_state)
    ///     .on_clicking_link(|url| Message::OpenLink(url))
    /// # ; } }
    /// ```
    #[must_use]
    pub fn on_clicking_link(mut self, f: impl Fn(String) -> M + 'static) -> Self {
        self.fn_clicking_link = Some(Box::new(f));
        self
    }

    /// Customizes how images are drawn in your widget.
    ///
    /// ```ignore
    /// MarkWidget::new(&self.mark_state)
    ///     .on_drawing_image(|info| {
    ///         // Pseudocode example to give you an idea
    ///         if let Some(image) = self.cache.get(info.url) {
    ///             let mut i = iced::widget::image(image.clone());
    ///             if let Some(width) = info.width {
    ///                 i = i.width(width);
    ///             }
    ///             if let Some(height) = info.height {
    ///                 i = i.height(height);
    ///             }
    ///             i.into()
    ///         } else {
    ///             widget::Column::new().into()
    ///         }
    ///     })
    /// ```
    ///
    /// The closure takes in [`ImageInfo`], and should return
    /// some element representing the rendered image,
    /// or a placeholder/loading indicator if no image is found.
    ///
    /// # Notes:
    /// - The returned `Element` **must** be `'static`.
    ///   - If you're calling helper functions inside this,
    ///     make sure to annotate them with `Element<'static, ...>`
    ///   - Clone your `Handle` every frame. Don't return anything
    ///     referencing your app struct.
    /// - **Image URL List**: To get a list of image URLs in the document,
    ///   use [`MarkState::find_image_links`], and download everything in it.
    /// - **Custom Downloader**: You’ll need to implement your own
    ///   downloader and load it with `iced::widget::image::Handle::from_bytes`
    ///   (or the SVG equivalent).
    /// - **Why?**: StriMD does not provide built-in
    ///   HTTP client functionality or async runtimes for image downloading,
    ///   as these are out of scope. The app must handle these responsibilities.
    #[must_use]
    pub fn on_drawing_image(
        mut self,
        f: impl Fn(ImageInfo) -> Element<'static, M, T> + 'a,
    ) -> Self {
        self.fn_drawing_image = Some(Box::new(f));
        self
    }

    /// Wrap rendered `<pre><code>` block content with app-specific styling.
    #[must_use]
    pub fn on_drawing_pre_block(
        mut self,
        f: impl Fn(Element<'a, M, T>) -> Element<'a, M, T> + 'a,
    ) -> Self {
        self.fn_drawing_pre_block = Some(Box::new(f));
        self
    }

    /// Passes a message when the internal state of the document is updated.
    ///
    /// # Usage:
    ///
    /// When the internal state of the document changes,
    /// this callback is triggered, and you should call [`MarkState::update`]
    /// in your `update()` function to apply the changes.
    ///
    /// ```no_run
    /// use strimd::{MarkWidget, MarkState, UpdateMsg};
    ///
    /// struct App { mark_state: MarkState }
    /// #[derive(Clone)]
    /// enum Message { UpdateDocument(UpdateMsg) }
    ///
    /// impl App {
    ///     fn view(&self) -> iced::Element<'_, Message> {
    ///         iced::widget::container(
    ///             MarkWidget::new(&self.mark_state)
    ///                 .on_updating_state(|n| Message::UpdateDocument(n))
    ///         ).padding(10).into()
    ///     }
    ///
    ///     fn update(&mut self, msg: Message) {
    ///         match msg {
    ///             Message::UpdateDocument(n) => self.mark_state.update(n),
    /// # _ => {}
    ///             // ...
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Notes:
    /// - This feature is optional but recommended.
    ///   Without it, `<details>` toggles will not work.
    /// - It takes in a closure that returns the message to pass when the state is updated.
    #[must_use]
    pub fn on_updating_state(mut self, f: impl Fn(UpdateMsg) -> M + 'static) -> Self {
        self.fn_update = Some(Arc::new(f));
        self
    }

    /// Change the color of different kinds of text
    /// in the document using [`crate::Style`].
    #[must_use]
    pub fn style(mut self, style: crate::Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Styles link buttons.
    ///
    /// Link buttons are links with non-text content (eg: images).
    /// and unlike text-only links they are rendered with `iced::widget::button`.
    ///
    /// For example, you could pass in `iced::widget::button::text`
    /// if you use iced's built-in theme, or have your own function here.
    #[must_use]
    pub fn style_link_button(
        mut self,
        f: impl Fn(&T, widget::button::Status) -> widget::button::Style + 'static,
    ) -> Self {
        self.fn_style_link_button = Some(Arc::new(f));
        self
    }

    /// Overrides the icon shown before GitHub alert titles (`[!NOTE]`, `[!WARNING]`, ...).
    ///
    /// The closure receives the alert label (`"Note"`, `"Tip"`, ...).
    #[must_use]
    pub fn github_alert_icon(mut self, f: impl Fn(&str) -> String + 'static) -> Self {
        self.fn_github_alert_icon = Some(Arc::new(f));
        self
    }

    /// Spacing between paragraphs or block elements. (default: 5.0)
    ///
    /// Visualization with an example document:
    ///
    /// ```txt
    /// # My document
    ///
    /// --<spacing>--
    ///
    /// Lorem ipsum dolor sit amet, consectetur adipiscing elit,
    /// sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
    ///
    /// --<spacing>--
    ///
    /// Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris
    /// nisi ut aliquip ex ea commodo consequat.
    ///
    /// --<spacing>--
    ///
    /// <image here>
    /// ```
    #[must_use]
    pub fn paragraph_spacing(mut self, spacing: f32) -> Self {
        self.paragraph_spacing = Some(spacing);
        self
    }

    /// Controls the rendering behavior of ruby annotations.
    ///
    /// By default, annotations are rendered with proper layout.
    /// Use [`RubyMode::Fallback`] for basic inline rendering (performance),
    /// or [`RubyMode::Ignore`] to only render the base text.
    #[must_use]
    pub fn ruby_mode(mut self, mode: RubyMode) -> Self {
        self.ruby_mode = mode;
        self
    }
}

#[derive(Default)]
pub enum RenderedSpan<'a, M, T> {
    Spans(Vec<widget::text::Span<'a, M, Font>>),
    Elem(Element<'a, M, T>, Emp, f32),
    #[default]
    None,
}

impl<M, T> std::fmt::Debug for RenderedSpan<'_, M, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderedSpan::Spans(spans) => {
                write!(f, "Rs::Spans ")?;
                f.debug_list()
                    .entries(spans.iter().map(|n| &*n.text))
                    .finish()
            }
            RenderedSpan::Elem(_, emp, gap) => write!(f, "Rs::Elem({emp:?}, gap={gap})"),
            RenderedSpan::None => write!(f, "Rs::None"),
        }
    }
}

impl<'a, M, T> RenderedSpan<'a, M, T>
where
    M: Clone + 'static,
    T: widget::text::Catalog + 'a,
{
    pub fn is_empty(&self) -> bool {
        match self {
            RenderedSpan::Spans(spans) => spans.is_empty(),
            RenderedSpan::Elem(_, e, _) => matches!(e, Emp::Empty),
            RenderedSpan::None => true,
        }
    }

    pub fn with_gap(self, gap: f32) -> Self {
        match self {
            Self::Elem(element, emp, _) => Self::Elem(element, emp, gap),
            other => other,
        }
    }

    // btw it supports clone so it's fine if we dont ref
    pub fn render(self) -> Element<'a, M, T> {
        match self {
            RenderedSpan::Spans(spans) => widget::rich_text(spans).on_link_click(|url| url).into(),
            RenderedSpan::Elem(element, _, _) => element,
            RenderedSpan::None => widget::Column::new().into(),
        }
    }
}

impl<'a, M, T> Add for RenderedSpan<'a, M, T>
where
    M: Clone + 'static,
    T: widget::text::Catalog + 'a,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        use RenderedSpan as Rs;
        match (self, rhs) {
            (Rs::None, rhs) => rhs,
            (lhs, Rs::None) => lhs,

            (Rs::Spans(mut spans1), Rs::Spans(spans2)) => {
                spans1.extend(spans2);
                Rs::Spans(spans1)
            }

            (r @ Rs::Spans(_), Rs::Elem(element, e, gap)) => Rs::Elem(
                widget::row![r.render()]
                    .push(e.has_something().then_some(element))
                    .spacing(gap)
                    .align_y(iced::Alignment::Center)
                    .wrap()
                    .into(),
                Emp::NonEmpty,
                gap,
            ),
            (Rs::Elem(element, e, gap), r @ Rs::Spans(_)) => Rs::Elem(
                widget::Row::new()
                    .push(e.has_something().then_some(element))
                    .push(r.render())
                    .spacing(gap)
                    .align_y(iced::Alignment::Center)
                    .wrap()
                    .into(),
                Emp::NonEmpty,
                gap,
            ),
            (Rs::Elem(e1, em1, gap1), Rs::Elem(e2, em2, gap2)) => Rs::Elem(
                widget::Row::new()
                    .push(em1.has_something().then_some(e1))
                    .push(em2.has_something().then_some(e2))
                    .spacing(gap1.max(gap2))
                    .align_y(iced::Alignment::Center)
                    .wrap()
                    .into(),
                Emp::NonEmpty,
                gap1.max(gap2),
            ),
        }
    }
}

impl<'a, M, T, E> From<E> for RenderedSpan<'a, M, T>
where
    M: Clone,
    T: widget::text::Catalog + 'a,
    E: Into<Element<'a, M, T>>,
{
    fn from(value: E) -> Self {
        Self::Elem(value.into(), Emp::NonEmpty, 5.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Emp {
    #[allow(unused)]
    Empty,
    NonEmpty,
}

impl Emp {
    pub fn is_empty(self) -> bool {
        match self {
            Emp::Empty => true,
            Emp::NonEmpty => false,
        }
    }

    pub fn has_something(self) -> bool {
        !self.is_empty()
    }
}

/// Information about the image to help you render it
/// in [`MarkWidget::on_drawing_image`].
#[non_exhaustive]
pub struct ImageInfo<'a> {
    pub url: &'a str,
    pub alt: Option<&'a str>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// Controls how ruby annotations are rendered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RubyMode {
    /// Layout horizontally with proper annotation support
    #[default]
    Full,
    /// Primitive inline layout, for better performance
    Fallback,
    /// Ignore ruby annotations entirely
    Ignore,
}
