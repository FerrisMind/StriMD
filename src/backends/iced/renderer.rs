use iced::{Element, Font, Length, Padding, Pixels, border, padding, widget};
use iced::widget::markdown::{self};

use crate::html::block_cache::CachedBlock;

use super::{
    dom::{self, DomRef},
    state::MarkState,
    structs::{
        ChildAlignment, ChildData, ChildDataFlags, ImageInfo, MarkWidget, RenderedSpan, UpdateMsg,
        UpdateMsgKind,
    },
    widgets::{link, link_text, underline},
};
use crate::html::block_cache::CachedCodeBlock;
use crate::html::fragment::HtmlFragment;

mod ruby;
mod table;

// Add everything to one place
pub trait ValidTheme:
    widget::button::Catalog
    + widget::text::Catalog
    + widget::rule::Catalog
    + widget::checkbox::Catalog
    + widget::markdown::Catalog
{
}
impl<T> ValidTheme for T where
    T: widget::button::Catalog
        + widget::text::Catalog
        + widget::rule::Catalog
        + widget::checkbox::Catalog
        + widget::markdown::Catalog
{
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    pub(crate) fn traverse_dom(&mut self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        if node.is_document_root() {
            if let Some(name) = node.tag_name() {
                return self.render_html_inner(name, node, data);
            }
            return self.render_children(node, data);
        }

        if let Some(text) = node.text_contents() {
            fn calc_size(text_size: f32, scaling: f32, factor: f32) -> f32 {
                text_size * (1.0 + ((scaling - 1.0) * factor))
            }

            let weight = data.heading_weight;
            let scaling = match weight {
                1 => 1.8,
                2 => 1.5,
                3 => 1.25,
                4 => 1.15,
                5 => 0.875,
                6 => 0.75,
                7 => 0.625,
                _ => 1.0,
            };
            let size = calc_size(self.text_size, scaling, self.heading_scale);

            if data.flags.contains(ChildDataFlags::MONOSPACE) {
                return self.codeblock(
                    text.into_owned(),
                    size,
                    !data.flags.contains(ChildDataFlags::KEEP_WHITESPACE),
                );
            }

            let display = if data.flags.contains(ChildDataFlags::KEEP_WHITESPACE) {
                text.into_owned()
            } else {
                clean_whitespace(&text)
            };

            let mut t = widget::span(display).size(size);
            return RenderedSpan::Spans(vec![{
                t = t.font({
                    let mut f = self.font;
                    if data.flags.contains(ChildDataFlags::BOLD) {
                        f.weight = iced::font::Weight::Bold;
                    }
                    if data.flags.contains(ChildDataFlags::ITALIC) {
                        f.style = iced::font::Style::Italic;
                    }
                    f
                });
                if data.flags.contains(ChildDataFlags::STRIKETHROUGH) {
                    t = t.strikethrough(true);
                }
                if data.flags.contains(ChildDataFlags::UNDERLINE) {
                    t = t.underline(true);
                }
                if data.flags.contains(ChildDataFlags::HIGHLIGHT) {
                    let highlight_color = self
                        .style
                        .and_then(|n| n.highlight_color)
                        .unwrap_or_else(|| iced::Color::from_rgb8(0xF7, 0xD8, 0x4B));
                    t = t.background(highlight_color);
                }
                if let Some(color) = self.style.and_then(|s| s.text_color) {
                    t = t.color(color);
                }
                t
            }]);
        }

        if let Some(name) = node.tag_name() {
            return self.render_html_inner(name, node, data);
        }

        RenderedSpan::None
    }

    fn render_html_inner(
        &mut self,
        name: &str,
        node: DomRef<'_>,
        mut data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let block_element = node.is_block_element();
        if block_element {
            dom::alignment_read(&mut data, node);
        }

        let e = match name {
            "summary" | "kbd" | "span" | "html" | "body" | "p" | "div" | "thead" | "tbody"
            | "tfoot" => self.render_children(node, data),

            "center" => {
                data.alignment = Some(ChildAlignment::Center);
                self.render_children(node, data)
            }
            "pre" => self.render_pre_block(node, data),

            "h1" => self.render_children(node, data.heading(1)),
            "h2" => self.render_children(node, data.heading(2)),
            "h3" => self.render_children(node, data.heading(3)),
            "h4" => self.render_children(node, data.heading(4)),
            "h5" => self.render_children(node, data.heading(5)),
            "h6" => self.render_children(node, data.heading(6)),
            "sub" => self.render_children(node, data.heading(7)),
            "rt" => self.render_children(node, data.heading(7).insert(ChildDataFlags::INSIDE_RUBY)),

            "blockquote" => widget::stack!(
                widget::row![
                    widget::space().width(10),
                    self.render_children(node, data).render()
                ],
                widget::rule::vertical(2)
            )
            .into(),

            "b" | "strong" => self.render_children(node, data.insert(ChildDataFlags::BOLD)),
            "em" | "i" => self.render_children(node, data.insert(ChildDataFlags::ITALIC)),
            "u" => self.render_children(node, data.insert(ChildDataFlags::UNDERLINE)),
            "del" | "s" | "strike" => {
                self.render_children(node, data.insert(ChildDataFlags::STRIKETHROUGH))
            }
            "code" => self.render_children(node, data.insert(ChildDataFlags::MONOSPACE)),
            "mark" => self.render_children(node, data.insert(ChildDataFlags::HIGHLIGHT)),

            "details" => self.draw_details(node, data),
            "a" => self.draw_link(node, data),
            "img" => self.draw_image(node),

            "br" => {
                if data.flags.contains(ChildDataFlags::INSIDE_RUBY) {
                    RenderedSpan::None
                } else {
                    widget::Column::new().into()
                }
            }
            "hr" => widget::rule::horizontal(1.0).into(),
            "head" | "title" | "meta" | "rtc" | "rp" | "rb" => RenderedSpan::None,

            "input" => match node.get_attr("type").as_deref().unwrap_or("text") {
                "checkbox" => {
                    let checked = node.get_attr("checked").is_some();
                    widget::checkbox(checked).into()
                }
                kind => RenderedSpan::Spans(vec![
                    widget::span(format!("<input type={kind} (TODO)>")).font(Font {
                        weight: iced::font::Weight::Bold,
                        ..self.font
                    }),
                ]),
            },

            "ul" => {
                data.li_ordered_number = None;
                self.render_list(node, data)
            }
            "ol" => {
                let start = node
                    .get_attr("start")
                    .and_then(|n| n.parse::<usize>().ok())
                    .unwrap_or(1);
                self.render_list(node, data.ordered_from(start))
            }
            "li" => self.render_list_item(node, data),

            "ruby" => self.draw_ruby(node, data),
            "table" => self.draw_table(node, data),

            _ => RenderedSpan::Spans(vec![widget::span(format!("<{name} (TODO)>")).font(Font {
                weight: iced::font::Weight::Bold,
                ..self.font
            })]),
        };

        if let (true, Some(align)) = (block_element, data.alignment) {
            let align: iced::Alignment = align.into();
            widget::column![e.render()]
                .width(Length::Fill)
                .align_x(align)
                .into()
        } else {
            e
        }
    }

    fn draw_details(&mut self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        let e = if let (Some(update), Some(state)) = (
            self.fn_update.clone(),
            self.state
                .dropdown_state
                .get(&self.current_dropdown_id)
                .copied(),
        ) {
            let summary = self.get_summary_elements(node, data);
            let regular_children =
                self.render_children(node, data.insert(ChildDataFlags::SKIP_SUMMARY));

            let umsg = UpdateMsg {
                kind: UpdateMsgKind::DetailsToggle(self.current_dropdown_id, !state),
            };

            let link = if let RenderedSpan::Spans(n) = summary {
                RenderedSpan::Spans(
                    n.into_iter()
                        .map(|n| n.link(update(umsg.clone())).underline(true))
                        .collect(),
                )
                .render()
            } else {
                widget::mouse_area(underline(summary.render()))
                    .on_press(update(umsg))
                    .into()
            };

            widget::stack![
                widget::column![link]
                    .push(state.then_some(regular_children.render()))
                    .padding(Padding::default().left(20).bottom(5)),
                widget::column![if state {
                    widget::text("V").size(12)
                } else {
                    widget::text(">").size(14)
                }]
                .push(state.then_some(widget::rule::vertical(1)))
                .spacing(5)
                .padding(Padding::default().left(5).top(if state { 5 } else { 0 })),
            ]
            .into()
        } else {
            widget::column![
                widget::rule::vertical(1),
                self.render_children(node, data).render(),
                widget::rule::horizontal(1),
            ]
            .padding(10)
            .spacing(10)
            .into()
        };
        self.current_dropdown_id += 1;
        e
    }

    fn get_summary_elements(&mut self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        node.children()
            .into_iter()
            .find(|child| child.tag_name() == Some("summary"))
            .map(|child| self.traverse_dom(child, data))
            .unwrap_or_default()
    }

    fn draw_image(&self, node: DomRef<'_>) -> RenderedSpan<'a, M, T> {
        let Some(url) = node.get_attr("src") else {
            return RenderedSpan::None;
        };
        if url.is_empty() {
            return RenderedSpan::None;
        }

        let width = node
            .get_attr("width")
            .and_then(|n| n.parse::<f32>().ok());
        let height = node
            .get_attr("height")
            .and_then(|n| n.parse::<f32>().ok());

        if let Some(func) = self.fn_drawing_image.as_deref() {
            return func(ImageInfo {
                url: &url,
                width,
                height,
            })
            .into();
        }
        RenderedSpan::None
    }

    fn draw_link(&mut self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        let link_col = self
            .style
            .and_then(|n| n.link_color)
            .unwrap_or_else(|| iced::Color::from_rgb8(0x5A, 0x6B, 0x9E));

        let children = self.render_children(node, data);

        if let Some(url) = node.get_attr("href") {
            let children_empty = node.children().is_empty();
            let msg = self.fn_clicking_link.as_ref();

            if children_empty {
                RenderedSpan::Spans(vec![
                    link_text(widget::span(url.clone()), url, msg).color(link_col),
                ])
            } else if let RenderedSpan::Spans(n) = children {
                RenderedSpan::Spans(
                    n.into_iter()
                        .map(|n| link_text(n, url.clone(), msg).color(link_col))
                        .collect(),
                )
            } else {
                link(
                    children.render(),
                    &url,
                    msg,
                    self.fn_style_link_button.clone(),
                )
                .into()
            }
        } else if let RenderedSpan::Spans(n) = children {
            RenderedSpan::Spans(
                n.into_iter()
                    .map(|n| n.underline(true).color(link_col))
                    .collect(),
            )
        } else {
            link(
                children.render(),
                "",
                Some(&Self::e).filter(|_| false),
                self.fn_style_link_button.clone(),
            )
            .into()
        }
    }

    fn e(_: String) -> M {
        panic!()
    }

    pub(crate) fn render_children(
        &mut self,
        node: DomRef<'_>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let children = node.children();

        let mut column = Vec::new();
        let mut row = RenderedSpan::None;

        let mut skipped_summary = false;
        let original_start = data.li_ordered_number;

        let mut i = 0;
        for item in children {
            if item.is_useless() {
                continue;
            }

            if !skipped_summary
                && data.flags.contains(ChildDataFlags::SKIP_SUMMARY)
                && item.tag_name() == Some("summary")
            {
                skipped_summary = true;
                continue;
            }

            let mut data = data;
            if let Some(base) = original_start {
                data.li_ordered_number = Some(base + i);
            }
            let element = self.traverse_dom(item, data);

            if !data.flags.contains(ChildDataFlags::INSIDE_RUBY) && item.is_block_element() {
                if !row.is_empty() {
                    let mut old_row = RenderedSpan::None;
                    std::mem::swap(&mut row, &mut old_row);
                    column.push(old_row);
                }

                column.push(element);
            } else {
                row = row + element;
            }

            i += 1;
        }

        if !row.is_empty() {
            column.push(row);
        }

        let len = column.len();
        let is_empty = column.is_empty() || column.iter().filter(|n| !n.is_empty()).count() == 0;

        if is_empty {
            RenderedSpan::None
        } else if len == 1 {
            column.into_iter().next().unwrap()
        } else {
            widget::column(
                column
                    .into_iter()
                    .filter(|n| !n.is_empty())
                    .map(RenderedSpan::render),
            )
            .spacing(self.paragraph_spacing.unwrap_or(5.0))
            .into()
        }
    }

    fn render_list(&mut self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        let indent = self.paragraph_spacing.unwrap_or(5.0);
        let items = self.render_children(node, data);

        widget::column![items.render()]
            .padding(Padding::default().left(indent))
            .into()
    }

    fn render_list_item(&mut self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        let marker_gap = self.paragraph_spacing.unwrap_or(5.0);
        let content = self.render_list_item_content(node, data);

        if node.has_task_checkbox_child() {
            content
        } else if let Some(num) = data.li_ordered_number {
            widget::row![
                widget::text(format!("{num}.")).size(self.text_size),
                content.render(),
            ]
            .spacing(marker_gap)
            .align_y(iced::Alignment::Start)
            .into()
        } else {
            widget::row![
                widget::text("•").size(self.text_size),
                content.render(),
            ]
            .spacing(marker_gap)
            .align_y(iced::Alignment::Start)
            .into()
        }
    }

    fn render_list_item_content(
        &mut self,
        node: DomRef<'_>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let meaningful: Vec<_> = node
            .children()
            .into_iter()
            .filter(|child| !child.is_useless())
            .collect();

        if meaningful.len() == 1 && meaningful[0].tag_name() == Some("p") {
            self.render_children(meaningful[0], data)
        } else {
            self.render_children(node, data)
        }
    }

    fn render_pre_block(&mut self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        let content = self
            .render_children(node, data.insert(ChildDataFlags::KEEP_WHITESPACE))
            .render();

        if let Some(draw) = &self.fn_drawing_pre_block {
            draw(content).into()
        } else {
            content.into()
        }
    }

    fn codeblock(&self, code: String, size: f32, inline: bool) -> RenderedSpan<'a, M, T> {
        let style = self.style;
        let inline_background = style.and_then(|s| s.inline_code_background);
        let inline_color = style.and_then(|s| s.inline_code_color);
        let text_color = style.and_then(|s| s.text_color);

        if inline {
            const INLINE_CODE_V_PAD: f32 = 1.0;
            const INLINE_CODE_H_PAD: f32 = 3.0;

            let mut code_span = widget::span(code)
                .size(size)
                .font(self.font_mono)
                .line_height(Pixels(size + INLINE_CODE_V_PAD * 2.0));

            if let Some(color) = inline_color {
                code_span = code_span.color(color);
            }
            if let Some(background) = inline_background {
                code_span = code_span
                    .background(background)
                    .border(border::rounded(4.0))
                    .padding(Padding {
                        top: INLINE_CODE_V_PAD,
                        right: INLINE_CODE_H_PAD,
                        bottom: INLINE_CODE_V_PAD,
                        left: INLINE_CODE_H_PAD,
                    });
            }

            let gap = widget::span("\u{2009}").size(size);
            RenderedSpan::Spans(vec![gap.clone(), code_span, gap])
        } else {
            let mut span = widget::span(code)
                .size(size)
                .font(self.font_mono)
                .line_height(Pixels(size * 1.25));
            if let Some(color) = text_color.or(inline_color) {
                span = span.color(color);
            }
            if let Some(background) = style.and_then(|s| s.code_block_background) {
                span = span
                    .background(background)
                    .padding(Padding::new(8.0))
                    .border(border::rounded(4.0));
            }
            RenderedSpan::Spans(vec![span])
        }
    }

    pub(crate) fn render_fragment_roots(
        &mut self,
        fragment: &HtmlFragment,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let roots = DomRef::fragment_roots(fragment);
        if roots.is_empty() {
            return RenderedSpan::None;
        }
        if roots.len() == 1 {
            return self.traverse_dom(roots[0], data);
        }
        self.render_children_list(roots, data)
    }

    fn render_children_list(
        &mut self,
        children: Vec<DomRef<'_>>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let mut column = Vec::new();
        let mut row = RenderedSpan::None;

        for item in children {
            if item.is_useless() {
                continue;
            }
            let element = self.traverse_dom(item, data);
            if item.is_block_element() {
                if !row.is_empty() {
                    let mut old_row = RenderedSpan::None;
                    std::mem::swap(&mut row, &mut old_row);
                    column.push(old_row);
                }
                column.push(element);
            } else {
                row = row + element;
            }
        }

        if !row.is_empty() {
            column.push(row);
        }

        if column.is_empty() {
            RenderedSpan::None
        } else if column.len() == 1 {
            column.into_iter().next().unwrap()
        } else {
            widget::column(
                column
                    .into_iter()
                    .filter(|n| !n.is_empty())
                    .map(RenderedSpan::render),
            )
            .spacing(self.paragraph_spacing.unwrap_or(5.0))
            .into()
        }
    }
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    fn render_from_block_cache(&mut self) -> Element<'a, M, T> {
        let spacing = self.paragraph_spacing.unwrap_or(5.0);
        let state: &'a MarkState = self.state;
        let cache = match &state.cache {
            Some(cache) => cache,
            None => return widget::Column::new().into(),
        };
        let mut column = widget::Column::new().spacing(spacing).width(Length::Fill);
        for index in 0..cache.len() {
            let span = match cache.entry(index) {
                Some(CachedBlock::Fragment(fragment)) => {
                    self.render_fragment_roots(fragment, ChildData::default())
                }
                Some(CachedBlock::Code(code)) => Self::render_fenced_code_block(&*self, code),
                Some(CachedBlock::Empty) => RenderedSpan::None,
                None => RenderedSpan::None,
            };
            if span.is_empty() {
                continue;
            }
            column = column.push(span.render());
        }
        column.into()
    }

    fn markdown_code_settings(&self) -> markdown::Settings {
        let (inline_code_bg, inline_code_color) = self
            .style
            .map(|s| (s.inline_code_background, s.inline_code_color))
            .unwrap_or((None, None));
        let link_color = self
            .style
            .and_then(|s| s.link_color)
            .unwrap_or_else(|| iced::Color::from_rgb8(0x5A, 0x6B, 0x9E));
        let style = markdown::Style {
            font: self.font,
            inline_code_highlight: markdown::Highlight {
                background: inline_code_bg
                    .unwrap_or_else(|| iced::Color::from_rgb8(0x2a, 0x2a, 0x2a))
                    .into(),
                border: border::rounded(4.0),
            },
            inline_code_padding: padding::left(3).right(3).top(1).bottom(1),
            inline_code_color: inline_code_color.unwrap_or(iced::Color::WHITE),
            inline_code_font: self.font_mono,
            code_block_font: self.font_mono,
            link_color,
        };
        let mut settings = markdown::Settings::with_text_size(Pixels(self.text_size), style);
        settings.spacing = Pixels(self.paragraph_spacing.unwrap_or(5.0));
        settings.code_size = Pixels(self.text_size);
        settings
    }

    fn render_fenced_code_block(
        &self,
        block: &'a CachedCodeBlock,
    ) -> RenderedSpan<'a, M, T> {
        if let Some(lines) = block
            .markdown_items
            .iter()
            .find_map(|item| match item {
                markdown::Item::CodeBlock { lines, .. } => Some(lines.as_slice()),
                _ => None,
            }) {
            let settings = self.markdown_code_settings();
            // Same layout as `iced::widget::markdown::code_block` (without horizontal scroll).
            let inner = widget::container(
                widget::column(
                    lines
                        .iter()
                        .map(|line| {
                            widget::rich_text(line.spans(settings.style))
                                .font(settings.style.code_block_font)
                                .size(settings.code_size)
                                .width(Length::Fill)
                                .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(Pixels(0.0)),
            )
            .padding(settings.code_size);
            let block = widget::container(inner)
                .width(Length::Fill)
                .padding(settings.code_size / 4.0)
                .class(<T as markdown::Catalog>::code_block());
            let element = if let Some(draw) = &self.fn_drawing_pre_block {
                draw(block.into())
            } else {
                block.into()
            };
            return element.into();
        }

        let size = self.text_size;
        let code = block.code.clone();
        if let Some(draw) = &self.fn_drawing_pre_block {
            return draw(self.codeblock(code, size, false).render()).into();
        }
        self.codeblock(code, size, false)
    }
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> From<MarkWidget<'a, M, T>> for Element<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    fn from(mut value: MarkWidget<'a, M, T>) -> Self {
        value.render_from_block_cache()
    }
}

fn clean_whitespace(input: &str) -> String {
    let mut s = input.split_whitespace().collect::<Vec<&str>>().join(" ");
    if let Some(last) = input.chars().next_back()
        && last != '\n'
        && last.is_whitespace()
    {
        s.push(last);
    }
    if let Some(first) = input.chars().next()
        && first != '\n'
        && first.is_whitespace()
    {
        s.insert(0, first);
    }
    s
}
