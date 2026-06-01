use iced::widget::markdown::{self};
use iced::{Element, Font, Length, Padding, Pixels, border, padding, widget};

use crate::html::block_alignment::BlockAlignment;
use crate::html::block_cache::CachedBlock;

use super::{
    dom::{self, DomRef},
    state::MarkState,
    structs::{
        ChildAlignment, ChildData, ChildDataFlags, Emp, ImageInfo, MarkWidget, RenderedSpan,
        UpdateMsg, UpdateMsgKind,
    },
    style::{DEFAULT_INLINE_CODE_BACKGROUND, DEFAULT_INLINE_CODE_FOREGROUND},
    widgets::{link, link_text, underline},
};
use crate::html::block_cache::CachedCodeBlock;
use crate::html::fragment::{HtmlFragment, HtmlNode};

mod ruby;
mod table;

// Add everything to one place
pub trait ValidTheme:
    widget::button::Catalog
    + widget::text::Catalog
    + widget::text_editor::Catalog
    + widget::rule::Catalog
    + widget::checkbox::Catalog
    + widget::markdown::Catalog
    + widget::container::Catalog
{
}
impl<T> ValidTheme for T where
    T: widget::button::Catalog
        + widget::text::Catalog
        + widget::text_editor::Catalog
        + widget::rule::Catalog
        + widget::checkbox::Catalog
        + widget::markdown::Catalog
        + widget::container::Catalog
{
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    pub(crate) fn traverse_dom(
        &mut self,
        node: DomRef<'_>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
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
                let inline = !data.flags.contains(ChildDataFlags::KEEP_WHITESPACE);
                return self.codeblock(text.into_owned(), size, inline);
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

        match name {
            "summary" | "kbd" | "span" | "html" | "body" | "p" | "div" | "thead" | "tbody"
            | "tfoot" => self.render_children(node, data),

            "center" => {
                data.alignment = Some(ChildAlignment::Center);
                let inner = self.render_children(node, data);
                if inner.is_empty() {
                    return inner;
                }
                widget::column![inner.render()]
                    .width(Length::Fill)
                    .spacing(self.paragraph_spacing.unwrap_or(5.0))
                    .into()
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
                    widget::container(self.render_children(node, data).render())
                        .width(Length::Fill)
                ]
                .width(Length::Fill),
                widget::rule::vertical(2)
            )
            .width(Length::Fill)
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
            "img" => self.draw_image(node, data),

            "br" => {
                if data.flags.contains(ChildDataFlags::INSIDE_RUBY) {
                    RenderedSpan::None
                } else {
                    widget::Column::new().into()
                }
            }
            "hr" => widget::rule::horizontal(1.0).into(),
            "head" | "title" | "meta" | "rtc" | "rp" | "rb" => RenderedSpan::None,

            "input" => match node.get_attr("type").unwrap_or("text") {
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

    fn get_summary_elements(
        &mut self,
        node: DomRef<'_>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        node.children()
            .into_iter()
            .find(|child| child.tag_name() == Some("summary"))
            .map(|child| self.traverse_dom(child, data))
            .unwrap_or_default()
    }

    fn draw_image(&self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        let Some(url) = node.get_attr("src") else {
            return RenderedSpan::None;
        };
        if url.is_empty() {
            return RenderedSpan::None;
        }

        let base_size = text_size_for_data(self.text_size, self.heading_scale, data.heading_weight);

        let is_badge = {
            let lower = url.to_ascii_lowercase();
            lower.contains("img.shields.io")
                || lower.contains("shields.io/")
                || lower.contains("badge.svg")
                || lower.contains("/badge")
        };

        let width = node
            .get_attr("width")
            .and_then(|n| n.parse::<f32>().ok())
            .or_else(|| {
                node.get_attr("style")
                    .as_ref()
                    .and_then(|s| css_dimension(s, "width"))
                    .map(|v| em_to_pixels(v, base_size))
            });
        let mut height = node
            .get_attr("height")
            .and_then(|n| n.parse::<f32>().ok())
            .or_else(|| {
                node.get_attr("style")
                    .as_ref()
                    .and_then(|s| css_dimension(s, "height"))
                    .map(|v| em_to_pixels(v, base_size))
            });
        if is_badge && width.is_none() && height.is_none() {
            height = Some(20.0);
        }

        if let Some(func) = self.fn_drawing_image.as_deref() {
            return func(ImageInfo { url, width, height }).into();
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
                    link_text(widget::span(url.to_string()), url.to_string(), msg).color(link_col),
                ])
            } else if let RenderedSpan::Spans(n) = children {
                RenderedSpan::Spans(
                    n.into_iter()
                        .map(|n| {
                            link_text(n, url.to_string(), msg)
                                .color(link_col)
                                .underline(true)
                        })
                        .collect(),
                )
            } else if let Some(handler) = msg {
                widget::mouse_area(children.render())
                    .on_press(handler(url.to_string()))
                    .into()
            } else {
                children.render().into()
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

        let meaningful: Vec<_> = children.into_iter().filter(|c| !c.is_useless()).collect();
        let mut skipped_summary = false;
        let original_start = data.li_ordered_number;

        let mut block_index = 0usize;
        let mut idx = 0usize;
        while idx < meaningful.len() {
            let item = meaningful[idx];

            if !skipped_summary
                && data.flags.contains(ChildDataFlags::SKIP_SUMMARY)
                && item.tag_name() == Some("summary")
            {
                skipped_summary = true;
                idx += 1;
                continue;
            }

            if item.is_shield_paragraph() {
                if !row.is_empty() {
                    let mut old_row = RenderedSpan::None;
                    std::mem::swap(&mut row, &mut old_row);
                    column.push(Self::apply_alignment_to_block(old_row, data.alignment));
                }
                let mut shield_row = widget::Row::new().spacing(6.0);
                while idx < meaningful.len() && meaningful[idx].is_shield_paragraph() {
                    shield_row = shield_row.push(self.render_shield_badge(meaningful[idx], data));
                    idx += 1;
                    block_index += 1;
                }
                let badge_row: Element<'a, M, T> = shield_row.width(Length::Shrink).into();
                column.push(if let Some(align) = data.alignment {
                    RenderedSpan::Elem(
                        Self::stack_align_in_viewport(badge_row, align),
                        Emp::NonEmpty,
                    )
                } else {
                    RenderedSpan::from(badge_row)
                });
                continue;
            }

            let mut child_data = data;
            if data.alignment.is_some() && item.is_block_element() {
                child_data.alignment = None;
            }
            if let Some(base) = original_start {
                child_data.li_ordered_number = Some(base + block_index);
            }
            let element = self.traverse_dom(item, child_data);

            if !data.flags.contains(ChildDataFlags::INSIDE_RUBY) && item.is_block_element() {
                if !row.is_empty() {
                    let mut old_row = RenderedSpan::None;
                    std::mem::swap(&mut row, &mut old_row);
                    column.push(Self::apply_alignment_to_block(old_row, data.alignment));
                }

                column.push(Self::apply_alignment_to_block(element, data.alignment));
                block_index += 1;
            } else {
                row = row + element;
            }

            idx += 1;
        }

        if !row.is_empty() {
            column.push(Self::apply_alignment_to_block(row, data.alignment));
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
            .width(Length::Fill)
            .into()
        }
    }

    /// `<center>` / `align="center"`: text lines use `rich_text` centering; block widgets centered in viewport.
    fn apply_alignment_to_block(
        span: RenderedSpan<'a, M, T>,
        alignment: Option<ChildAlignment>,
    ) -> RenderedSpan<'a, M, T> {
        let Some(align) = alignment else {
            return span;
        };
        if span.is_empty() {
            return span;
        }
        match span {
            RenderedSpan::Spans(spans) => RenderedSpan::Elem(
                widget::container(widget::rich_text(spans).on_link_click(|url| url))
                    .width(Length::Fill)
                    .align_x(align.to_horizontal())
                    .into(),
                Emp::NonEmpty,
            ),
            other => RenderedSpan::Elem(
                Self::stack_align_in_viewport(other.render(), align),
                Emp::NonEmpty,
            ),
        }
    }

    fn stack_align_in_viewport(
        element: Element<'a, M, T>,
        align: ChildAlignment,
    ) -> Element<'a, M, T> {
        let horizontal = match align {
            ChildAlignment::Center => iced::alignment::Horizontal::Center,
            ChildAlignment::Right => iced::alignment::Horizontal::Right,
        };
        widget::column![element]
            .width(Length::Fill)
            .align_x(horizontal)
            .into()
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

        if let Some(checkbox) = node.direct_task_checkbox() {
            let checked = checkbox.get_attr("checked").is_some();
            let body = self.render_list_item_body_excluding_inputs(node, data);
            return widget::row![
                widget::text("•").size(self.text_size),
                widget::checkbox(checked),
                body.render(),
            ]
            .spacing(marker_gap)
            .align_y(iced::Alignment::Start)
            .into();
        }

        let content = self.render_list_item_content(node, data);

        if let Some(num) = data.li_ordered_number {
            widget::row![
                widget::text(format!("{num}.")).size(self.text_size),
                content.render(),
            ]
            .spacing(marker_gap)
            .align_y(iced::Alignment::Start)
            .into()
        } else {
            widget::row![widget::text("•").size(self.text_size), content.render(),]
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
            self.render_list_item_content(meaningful[0], data)
        } else {
            self.render_list_item_body_excluding_inputs(node, data)
        }
    }

    fn render_list_item_body_excluding_inputs(
        &mut self,
        node: DomRef<'_>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let mut column = Vec::new();
        let mut row = RenderedSpan::None;

        for child in node.children() {
            if child.is_useless() || DomRef::is_task_checkbox(child) {
                continue;
            }
            let element = if child.tag_name() == Some("p") {
                self.render_list_item_body_excluding_inputs(child, data)
            } else {
                self.traverse_dom(child, data)
            };
            if child.is_block_element() {
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

        let len = column.len();
        if len == 0 {
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
            .width(Length::Fill)
            .into()
        }
    }

    /// Badge `<p>` with only `img` or `a>img` — render the graphic directly (no extra block wrap).
    fn render_shield_badge(&mut self, paragraph: DomRef<'_>, data: ChildData) -> Element<'a, M, T> {
        let mut row = widget::Row::new().spacing(6.0);
        let mut has_badge = false;
        for child in paragraph.children() {
            if child.is_useless() {
                continue;
            }
            match child.tag_name() {
                Some("img") | Some("a") => {
                    row = row.push(self.traverse_dom(child, data).render());
                    has_badge = true;
                }
                _ => {}
            }
        }
        if has_badge {
            row.into()
        } else {
            self.traverse_dom(paragraph, data).render()
        }
    }

    fn render_pre_block(&mut self, node: DomRef<'_>, data: ChildData) -> RenderedSpan<'a, M, T> {
        if let Some(code_node) = node
            .children()
            .into_iter()
            .find(|child| child.tag_name() == Some("code"))
        {
            let text = code_node.accumulated_text();
            if !text.trim().is_empty() {
                let lang = code_node
                    .get_attr("class")
                    .and_then(|c| c.strip_prefix("language-").map(str::to_string));
                let items = crate::backends::iced::iced_markdown_items_for_codeblock(
                    lang.as_deref(),
                    text.trim_end_matches('\n'),
                );
                if let Some(lines) = items.iter().find_map(|item| match item {
                    markdown::Item::CodeBlock { lines, .. } => Some(lines.as_slice()),
                    _ => None,
                }) {
                    let settings = self.markdown_code_settings();
                    return self
                        .wrap_fenced_code_container(self.fenced_code_inner(lines, &settings));
                }
            }
        }

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
        let inline_background = style.and_then(|s| s.inline_code_background).or(if inline {
            Some(DEFAULT_INLINE_CODE_BACKGROUND)
        } else {
            None
        });
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

    fn wrap_fenced_code_container(&self, inner: Element<'a, M, T>) -> RenderedSpan<'a, M, T> {
        let settings = self.markdown_code_settings();
        let shell = widget::container(widget::container(inner).padding(settings.code_size))
            .width(Length::Fill)
            .padding(settings.code_size / 4.0)
            .class(<T as markdown::Catalog>::code_block());
        let element = if let Some(draw) = &self.fn_drawing_pre_block {
            draw(shell.into())
        } else {
            shell.into()
        };
        element.into()
    }

    fn highlighted_codeblock_lines(
        &self,
        lines: &[markdown::Text],
        settings: &markdown::Settings,
    ) -> Element<'a, M, T> {
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
        .spacing(Pixels(0.0))
        .into()
    }

    fn fenced_code_inner(
        &self,
        lines: &[markdown::Text],
        settings: &markdown::Settings,
    ) -> Element<'a, M, T> {
        widget::container(self.highlighted_codeblock_lines(lines, settings)).into()
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
        let mut index = 0;
        while index < cache.len() {
            let block_data = child_data_for_block_alignment(cache.entry_alignment(index));
            if let Some(CachedBlock::Fragment(fragment)) = cache.entry(index)
                && Self::fragment_is_shield_paragraph(fragment)
            {
                let mut row = widget::Row::new().spacing(6.0);
                while index < cache.len() {
                    match cache.entry(index) {
                        Some(CachedBlock::Empty) => {
                            index += 1;
                            continue;
                        }
                        Some(CachedBlock::Fragment(f)) if Self::fragment_is_shield_paragraph(f) => {
                            let roots = DomRef::fragment_roots(f);
                            if roots.len() == 1 {
                                row = row.push(self.render_shield_badge(
                                    roots[0],
                                    child_data_for_block_alignment(cache.entry_alignment(index)),
                                ));
                            } else {
                                row = row.push(
                                    self.render_fragment_roots(
                                        f,
                                        child_data_for_block_alignment(
                                            cache.entry_alignment(index),
                                        ),
                                    )
                                    .render(),
                                );
                            }
                            index += 1;
                        }
                        _ => break,
                    }
                }
                let badge_row: Element<'a, M, T> = row.width(Length::Shrink).into();
                column = column.push(if let Some(align) = block_data.alignment {
                    Self::stack_align_in_viewport(badge_row, align)
                } else {
                    badge_row
                });
                continue;
            }

            let mut center_wrapper_fragment = false;
            let span = match cache.entry(index) {
                Some(CachedBlock::Fragment(fragment)) => {
                    center_wrapper_fragment = Self::fragment_roots_are_center_wrapper(fragment);
                    let render_data = if center_wrapper_fragment {
                        block_data
                    } else {
                        ChildData::default()
                    };
                    self.render_fragment_roots(fragment, render_data)
                }
                Some(CachedBlock::Code(code)) => Self::render_fenced_code_block(self, code),
                Some(CachedBlock::Empty) => RenderedSpan::None,
                None => RenderedSpan::None,
            };
            index += 1;
            if span.is_empty() {
                continue;
            }
            let element = span.render();
            column = column.push(if let Some(align) = block_data.alignment {
                if center_wrapper_fragment {
                    element
                } else {
                    Self::stack_align_in_viewport(
                        widget::container(element).width(Length::Shrink).into(),
                        align,
                    )
                }
            } else {
                element
            });
        }
        column.into()
    }

    fn fragment_is_shield_paragraph(fragment: &HtmlFragment) -> bool {
        let roots = fragment.roots();
        if roots.len() != 1 {
            return false;
        }
        let Some(HtmlNode::Element { tag, children, .. }) = fragment.node(roots[0]) else {
            return false;
        };
        if tag.as_str() != "p" {
            return false;
        }
        let mut has_badge = false;
        for &child in children {
            match fragment.node(child) {
                Some(HtmlNode::Element { tag, attrs, .. }) if tag.as_str() == "img" => {
                    if !Self::fragment_img_is_badge(attrs) {
                        return false;
                    }
                    has_badge = true;
                }
                Some(HtmlNode::Element { tag, children, .. }) if tag.as_str() == "a" => {
                    let only_img = children.len() == 1
                        && children.iter().all(|&c| {
                            matches!(
                                fragment.node(c),
                                Some(HtmlNode::Element { tag, .. }) if tag.as_str() == "img"
                            )
                        });
                    if !only_img {
                        return false;
                    }
                    let Some(HtmlNode::Element { attrs, .. }) = fragment.node(children[0]) else {
                        return false;
                    };
                    if !Self::fragment_img_is_badge(attrs) {
                        return false;
                    }
                    has_badge = true;
                }
                Some(HtmlNode::Text(t)) if t.trim().is_empty() => {}
                _ => return false,
            }
        }
        has_badge
    }

    fn fragment_img_is_badge(attrs: &[crate::html::fragment::HtmlAttr]) -> bool {
        let Some(src) = attrs
            .iter()
            .find(|a| a.name.as_ref() == "src")
            .map(|a| a.value.as_ref())
        else {
            return false;
        };
        let lower = src.to_ascii_lowercase();
        lower.contains("img.shields.io")
            || lower.contains("shields.io/")
            || lower.contains("badge.svg")
            || lower.contains("/badge")
    }

    fn fragment_roots_are_center_wrapper(fragment: &HtmlFragment) -> bool {
        !fragment.roots().is_empty()
            && fragment.roots().iter().all(|&root| {
                matches!(
                    fragment.node(root),
                    Some(HtmlNode::Element { tag, .. }) if tag.as_str() == "center"
                )
            })
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
                    .unwrap_or(DEFAULT_INLINE_CODE_BACKGROUND)
                    .into(),
                border: border::rounded(4.0),
            },
            inline_code_padding: padding::left(3).right(3).top(1).bottom(1),
            inline_code_color: inline_code_color.unwrap_or(DEFAULT_INLINE_CODE_FOREGROUND),
            inline_code_font: self.font_mono,
            code_block_font: self.font_mono,
            link_color,
        };
        let mut settings = markdown::Settings::with_text_size(Pixels(self.text_size), style);
        settings.spacing = Pixels(self.paragraph_spacing.unwrap_or(5.0));
        settings.code_size = Pixels(self.text_size);
        settings
    }

    fn render_fenced_code_block(&self, block: &'a CachedCodeBlock) -> RenderedSpan<'a, M, T> {
        if let Some(lines) = block.markdown_items.iter().find_map(|item| match item {
            markdown::Item::CodeBlock { lines, .. } => Some(lines.as_slice()),
            _ => None,
        }) {
            let settings = self.markdown_code_settings();
            return self.wrap_fenced_code_container(self.fenced_code_inner(lines, &settings));
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

fn child_data_for_block_alignment(align: Option<BlockAlignment>) -> ChildData {
    ChildData {
        alignment: align.map(|a| match a {
            BlockAlignment::Center => ChildAlignment::Center,
            BlockAlignment::Right => ChildAlignment::Right,
        }),
        ..Default::default()
    }
}

fn text_size_for_data(text_size: f32, heading_scale: f32, heading_weight: u16) -> f32 {
    if heading_weight == 0 {
        return text_size;
    }
    let scaling = match heading_weight {
        1 => 1.8,
        2 => 1.5,
        3 => 1.25,
        4 => 1.15,
        5 => 0.875,
        6 => 0.75,
        7 => 0.625,
        _ => 1.0,
    };
    text_size * (1.0 + ((scaling - 1.0) * heading_scale))
}

fn css_dimension(style: &str, name: &str) -> Option<f32> {
    let lower = style.to_ascii_lowercase();
    let needle = format!("{name}:");
    let start = lower.find(&needle)?;
    let rest = style.get(start + needle.len()..)?;
    let value = rest.split(';').next()?.trim();
    parse_css_length(value)
}

fn parse_css_length(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(em) = value.strip_suffix("em") {
        em.trim().parse().ok()
    } else if let Some(px) = value.strip_suffix("px") {
        px.trim().parse().ok()
    } else {
        value.parse().ok()
    }
}

fn em_to_pixels(em: f32, text_size: f32) -> f32 {
    em * text_size
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

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::html::fragment::HtmlFragment;

    #[test]
    fn fragment_root_text_nodes_are_rendered() {
        let fragment = HtmlFragment::from_html("Here's<b> bold text, </b>and\n<i> italics!</i>");
        let state = MarkState::from_blocks(&[]);
        let mut widget = MarkWidget::<(), iced::Theme>::new(&state);
        let rendered = widget.render_fragment_roots(&fragment, ChildData::default());
        let debug = format!("{rendered:?}");
        assert!(
            debug.contains("Here's"),
            "expected root text 'Here's' in render output, got: {debug}"
        );
        assert!(
            debug.contains("and"),
            "expected root text 'and' in render output, got: {debug}"
        );
        assert!(
            debug.contains("bold text"),
            "expected bold text in render output, got: {debug}"
        );
    }

    #[test]
    fn centered_inline_markdown_paragraph_stays_single_spans_row() {
        let fragment = HtmlFragment::from_html(
            "<center><p>Normal, <em>italic</em>, <strong>bold</strong>, <del>strikethrough</del>, <strong>underline</strong>, <code>code</code>, <a href=\"https://example.com\">link</a></p></center>",
        );
        let state = MarkState::from_blocks(&[]);
        let mut widget = MarkWidget::<(), iced::Theme>::new(&state);
        let center = DomRef::fragment_roots(&fragment)[0];
        let paragraph = center
            .children()
            .into_iter()
            .find(|child| child.tag_name() == Some("p"))
            .expect("paragraph");
        let child_tags: Vec<_> = paragraph
            .children()
            .into_iter()
            .map(|child| {
                child
                    .tag_name()
                    .map(str::to_string)
                    .unwrap_or_else(|| "#text".to_string())
            })
            .collect();

        let rendered = widget.render_children(paragraph, ChildData::default());
        let debug = format!("{rendered:?}");
        eprintln!("paragraph child tags: {child_tags:?}");
        eprintln!("paragraph debug: {debug}");
        assert!(
            matches!(rendered, RenderedSpan::Spans(_)),
            "expected inline paragraph to stay spans, got {debug}"
        );
    }
}
