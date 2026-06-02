use iced::{
    Background, Border, Color, Element, Font, Length, Padding, Rectangle, Size, Vector,
    advanced, alignment, widget,
};

use super::structs::FStyleLinkButton;

pub fn link<'a, M: 'a, T, R: advanced::Renderer + 'a, F>(
    e: impl Into<Element<'a, M, T, R>>,
    url: &str,
    msg: Option<&F>,
    f: Option<FStyleLinkButton<T>>,
) -> widget::Button<'a, M, T, R>
where
    T: widget::button::Catalog + widget::rule::Catalog + 'a,
    F: Fn(String) -> M,
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    let mut b = widget::button(underline(e))
        .on_press_maybe(msg.map(|n| n(url.to_owned())))
        .padding(0);
    if let Some(f) = f {
        b = b.style(move |t, s| f(t, s));
    }
    b.width(Length::Shrink)
}

pub fn link_text<'a, M: 'a, F>(
    e: widget::text::Span<'a, M, Font>,
    url: String,
    msg: Option<&F>,
) -> widget::text::Span<'a, M, Font>
where
    F: Fn(String) -> M,
{
    e.link_maybe(msg.map(|n| n(url)))
}

pub fn underline<'a, M: 'a, T: widget::rule::Catalog + 'a, R: advanced::Renderer + 'a>(
    e: impl Into<Element<'a, M, T, R>>,
) -> widget::Stack<'a, M, T, R> {
    widget::stack!(
        widget::column![e.into()],
        widget::column![
            widget::space().height(Length::Fill),
            widget::rule::horizontal(1),
            widget::space().height(1),
        ]
    )
}

#[derive(Debug, Clone, Copy)]
pub struct KbdStyle {
    pub background: Color,
    pub text_color: Color,
    pub border_color: Color,
    pub shadow_color: Color,
    pub font: Font,
    pub font_size: f32,
    pub padding: [f32; 2],
    pub radius: f32,
    pub min_width: f32,
}

impl KbdStyle {
    pub fn size2(
        background: Color,
        text_color: Color,
        border_color: Color,
        shadow_color: Color,
        font: Font,
    ) -> Self {
        Self {
            background,
            text_color,
            border_color,
            shadow_color,
            font,
            font_size: 11.0,
            padding: [3.0, 7.0],
            radius: 4.0,
            min_width: 22.0,
        }
    }
}

pub fn kbd<'a, M: 'a, T: 'a>(label: impl Into<String>, style: KbdStyle) -> Element<'a, M, T> {
    Element::new(KbdWidget::new(label.into(), style))
}

pub fn quote_block<'a, M: 'a, T: 'a, R: advanced::Renderer + 'a>(
    content: impl Into<Element<'a, M, T, R>>,
    color: Color,
    bar_width: f32,
    content_inset: f32,
) -> Element<'a, M, T, R> {
    Element::new(QuoteBlockWidget::new(
        content.into(),
        color,
        bar_width,
        content_inset,
    ))
}

use iced::advanced::layout;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::text;
use iced::advanced::widget::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::{Event, mouse};

struct KbdWidget {
    label: String,
    style: KbdStyle,
}

impl KbdWidget {
    fn new(label: String, style: KbdStyle) -> Self {
        Self { label, style }
    }
}

struct QuoteBlockWidget<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    color: Color,
    bar_width: f32,
    content_inset: f32,
}

impl<'a, Message, Theme, Renderer> QuoteBlockWidget<'a, Message, Theme, Renderer> {
    fn new(
        content: Element<'a, Message, Theme, Renderer>,
        color: Color,
        bar_width: f32,
        content_inset: f32,
    ) -> Self {
        Self {
            content,
            color,
            bar_width,
            content_inset,
        }
    }
}

impl<Message, AppTheme, Renderer> Widget<Message, AppTheme, Renderer> for KbdWidget
where
    Renderer: renderer::Renderer + text::Renderer<Font = Font>,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        let font_size = self.style.font_size;
        let padding = self.style.padding;
        let text_width = self.label.chars().count() as f32 * font_size * 0.6;
        let text_height = font_size * 1.3;
        let total_width = (text_width + padding[1] * 2.0).max(self.style.min_width);
        let total_height = text_height + padding[0] * 2.0;

        layout::Node::new(Size::new(total_width, total_height))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &AppTheme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    color: self.style.border_color,
                    width: 1.0,
                    radius: self.style.radius.into(),
                },
                shadow: iced::Shadow {
                    color: self.style.shadow_color,
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 2.0,
                },
                ..renderer::Quad::default()
            },
            Background::Color(self.style.background),
        );

        let font_size: iced::Pixels = self.style.font_size.into();
        renderer.fill_text(
            text::Text {
                content: self.label.clone(),
                font: self.style.font,
                size: font_size,
                line_height: text::LineHeight::Absolute(font_size),
                bounds: bounds.size(),
                align_x: text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            },
            bounds.center(),
            self.style.text_color,
            bounds,
        );
    }
}

impl<'a, Message, AppTheme, Renderer> From<KbdWidget> for Element<'a, Message, AppTheme, Renderer>
where
    Renderer: renderer::Renderer + text::Renderer<Font = Font> + 'a,
    Message: 'a,
{
    fn from(widget: KbdWidget) -> Element<'a, Message, AppTheme, Renderer> {
        Element::new(widget)
    }
}

impl<Message, AppTheme, Renderer> Widget<Message, AppTheme, Renderer>
    for QuoteBlockWidget<'_, Message, AppTheme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::positioned(
            limits,
            Length::Fill,
            Length::Shrink,
            Padding::default().left(self.content_inset),
            |limits| {
                self.content
                    .as_widget_mut()
                    .layout(&mut tree.children[0], renderer, limits)
            },
            |content, _| content,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        if let Some(content_layout) = layout.children().next() {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                content_layout,
                renderer,
                operation,
            );
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(content_layout) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            content_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(content_layout) = layout.children().next() else {
            return mouse::Interaction::None;
        };
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            content_layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &AppTheme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y,
                    width: self.bar_width,
                    height: bounds.height,
                },
                ..renderer::Quad::default()
            },
            Background::Color(self.color),
        );

        if let Some(content_layout) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                content_layout,
                cursor,
                viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, AppTheme, Renderer>> {
        let content_layout = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            content_layout,
            renderer,
            viewport,
            translation,
        )
    }
}
