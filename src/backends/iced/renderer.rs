use iced::{Element, Font, Length, Padding, Pixels, border, widget};
use markup5ever_rcdom::{Node, NodeData};

use crate::html::block_cache::CachedBlock;

use super::{
    structs::{
        ChildAlignment, ChildData, ChildDataFlags, ImageInfo, MarkWidget, RenderedSpan, UpdateMsg,
        UpdateMsgKind,
    },
    widgets::{link, link_text, underline},
};
use crate::html::block_cache::CachedCodeBlock;

mod ruby;
mod table;

// Add everything to one place
pub trait ValidTheme:
    widget::button::Catalog
    + widget::text::Catalog
    + widget::rule::Catalog
    + widget::checkbox::Catalog
{
}
impl<T> ValidTheme for T where
    T: widget::button::Catalog
        + widget::text::Catalog
        + widget::rule::Catalog
        + widget::checkbox::Catalog
{
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    pub(crate) fn traverse_node(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
        match &node.data {
            markup5ever_rcdom::NodeData::Document => self.render_children(node, data),

            markup5ever_rcdom::NodeData::Text { contents } => {
                fn calc_size(text_size: f32, scaling: f32, factor: f32) -> f32 {
                    text_size * (1.0 + ((scaling - 1.0) * factor))
                }

                let text = contents.borrow();
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
                    self.codeblock(
                        text.to_string(),
                        size,
                        !data.flags.contains(ChildDataFlags::KEEP_WHITESPACE),
                    )
                } else {
                    let mut t =
                        widget::span(if data.flags.contains(ChildDataFlags::KEEP_WHITESPACE) {
                            text.to_string()
                        } else {
                            clean_whitespace(&text)
                        })
                        .size(size);

                    RenderedSpan::Spans(vec![{
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
                        t
                    }])
                }
            }
            markup5ever_rcdom::NodeData::Element { name, attrs, .. } => {
                self.render_html_inner(name, attrs, node, data)
            }
            _ => RenderedSpan::None,
        }
    }

    fn render_html_inner(
        &mut self,
        name: &html5ever::QualName,
        attrs: &std::cell::RefCell<Vec<html5ever::Attribute>>,
        node: &Node,
        mut data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let name = name.local.to_string();
        let attrs = attrs.borrow();

        let block_element = is_block_element(node);
        if block_element {
            alignment_read(&mut data, &attrs);
        }

        let e = match name.as_str() {
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
            "a" => self.draw_link(node, &attrs, data),
            "img" => self.draw_image(&attrs),

            "br" => {
                if data.flags.contains(ChildDataFlags::INSIDE_RUBY) {
                    RenderedSpan::None
                } else {
                    widget::Column::new().into()
                }
            }
            "hr" => widget::rule::horizontal(1.0).into(),
            "head" | "title" | "meta" | "rtc" | "rp" | "rb" => RenderedSpan::None,

            "input" => match get_attr(&attrs, "type").unwrap_or("text") {
                "checkbox" => {
                    let checked = attrs.iter().any(|attr| &*attr.name.local == "checked");
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
                let start = get_attr(&attrs, "start")
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

    fn draw_details(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
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

    fn get_summary_elements(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
        node.children
            .borrow()
            .iter()
            .find(|elem| {
                if let NodeData::Element { name, .. } = &elem.data {
                    &*name.local == "summary"
                } else {
                    false
                }
            })
            .map(|n| self.traverse_node(n, data))
            .unwrap_or_default()
    }

    fn draw_image(&self, attrs: &[html5ever::Attribute]) -> RenderedSpan<'a, M, T> {
        if let Some(attr) = attrs.iter().find(|attr| &*attr.name.local == "src") {
            let url = &*attr.value;

            let width = get_attr_num(attrs, "width");
            let height = get_attr_num(attrs, "height");

            if let Some(func) = self.fn_drawing_image.as_deref() {
                return func(ImageInfo { url, width, height }).into();
            }
        }
        // Error, no `src` tag in `<img>`
        RenderedSpan::None
    }

    fn draw_link(
        &mut self,
        node: &Node,
        attrs: &std::cell::Ref<'_, Vec<html5ever::Attribute>>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let link_col = self
            .style
            .and_then(|n| n.link_color)
            .unwrap_or_else(|| iced::Color::from_rgb8(0x5A, 0x6B, 0x9E));

        let children = self.render_children(node, data);

        if let Some(attr) = attrs
            .iter()
            .find(|attr| attr.name.local.to_string().as_str() == "href")
        {
            let url = attr.value.to_string();
            let children_empty = { node.children.borrow().is_empty() };

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
        // This will never run, don't worry
        panic!()
    }

    fn render_children(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
        let children = node.children.borrow();

        let mut column = Vec::new();
        let mut row = RenderedSpan::None;

        let mut skipped_summary = false;
        let original_start = data.li_ordered_number;

        let mut i = 0;
        for item in children.iter() {
            if is_node_useless(item) {
                continue;
            }

            if let NodeData::Element { name, .. } = &item.data
                && !skipped_summary
                && data.flags.contains(ChildDataFlags::SKIP_SUMMARY)
                && &*name.local == "summary"
            {
                // Skip the first <summary> inside <details>
                // as it's already drawn
                skipped_summary = true;
                continue;
            }

            let mut data = data;
            if let Some(base) = original_start {
                data.li_ordered_number = Some(base + i);
            }
            let element = self.traverse_node(item, data);

            if !data.flags.contains(ChildDataFlags::INSIDE_RUBY) && is_block_element(item) {
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

    fn render_list(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
        let indent = self.paragraph_spacing.unwrap_or(5.0);
        let items = self.render_children(node, data);

        widget::column![items.render()]
            .padding(Padding::default().left(indent))
            .into()
    }

    fn render_list_item(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
        let marker_gap = self.paragraph_spacing.unwrap_or(5.0);
        let content = self.render_list_item_content(node, data);

        if list_item_has_task_marker(node) {
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
        node: &Node,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let children = node.children.borrow();
        let meaningful: Vec<_> = children
            .iter()
            .filter(|child| !is_node_useless(child))
            .collect();

        if meaningful.len() == 1
            && matches!(
                &meaningful[0].data,
                NodeData::Element { name, .. } if &*name.local == "p"
            )
        {
            self.render_children(meaningful[0], data)
        } else {
            self.render_children(node, data)
        }
    }

    fn render_pre_block(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
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
            // VS Code webview: padding 1×3px, radius 4px, textPreformat colors.
            // iced expands highlight padding outward and overlaps adjacent spaces, so we
            // add thin external spacers and keep the pill height tight with absolute line height.
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
            // VS Code markdown.css: pre code has no pill — mono + editor foreground only.
            let mut span = widget::span(code).size(size).font(self.font_mono);
            if let Some(color) = text_color.or(inline_color) {
                span = span.color(color);
            }
            RenderedSpan::Spans(vec![span])
        }
    }
}

fn alignment_read(data: &mut ChildData, attrs: &[html5ever::Attribute]) {
    let Some(align) = get_attr(attrs, "align") else {
        return;
    };

    if let "right" | "center" | "centre" = align {
        data.alignment = Some(if align == "right" {
            ChildAlignment::Right
        } else {
            ChildAlignment::Center
        });
    } else if align == "left" {
        data.alignment = None;
    }
}

fn get_attr_num(attrs: &[html5ever::Attribute], attr_name: &str) -> Option<f32> {
    get_attr(attrs, attr_name).and_then(|n| n.parse::<f32>().ok())
}

fn get_attr<'a>(attrs: &'a [html5ever::Attribute], attr_name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|attr| {
            let name = &*attr.name.local;
            name == attr_name
        })
        .map(|n| &*n.value)
}

fn is_node_useless(node: &Node) -> bool {
    if let markup5ever_rcdom::NodeData::Text { contents } = &node.data {
        let contents = contents.borrow();
        let contents = contents.to_string();
        contents.trim().is_empty()
    } else {
        false
    }
}

fn is_block_element(node: &Node) -> bool {
    let markup5ever_rcdom::NodeData::Element { name, .. } = &node.data else {
        return false;
    };
    let n: &str = &name.local;

    matches!(
        n,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "canvas"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "noscript"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "tfoot"
            | "ul"
            | "video"
            | "br"
            | "details"
            | "summary"
    )
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    fn render_from_block_cache(&mut self) -> Element<'a, M, T> {
        use super::state::MarkStateSource;

        let spacing = self.paragraph_spacing.unwrap_or(5.0);
        let cache = match &self.state.source {
            MarkStateSource::Blocks(cache) => cache,
            MarkStateSource::LegacyDom(dom) => {
                return self
                    .traverse_node(&dom.document, ChildData::default())
                    .render();
            }
        };

        let mut column = widget::Column::new().spacing(spacing).width(Length::Fill);
        for index in 0..cache.len() {
            let span = match cache.entry(index) {
                Some(CachedBlock::Dom(dom)) => {
                    self.traverse_node(&dom.document, ChildData::default())
                }
                Some(CachedBlock::Code(code)) => self.render_fenced_code_block(code),
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

    fn render_fenced_code_block(&self, block: &CachedCodeBlock) -> RenderedSpan<'a, M, T> {
        let size = self.text_size;
        let code = block.code.clone();
        if let Some(draw) = &self.fn_drawing_pre_block {
            draw(self.codeblock(code, size, false).render()).into()
        } else {
            self.codeblock(code, size, false)
        }
    }
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> From<MarkWidget<'a, M, T>> for Element<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    fn from(mut value: MarkWidget<'a, M, T>) -> Self {
        use super::state::MarkStateSource;

        match &value.state.source {
            MarkStateSource::LegacyDom(dom) => {
                value
                    .traverse_node(&dom.document, ChildData::default())
                    .render()
            }
            MarkStateSource::Blocks(_) => value.render_from_block_cache(),
        }
    }
}

fn list_item_has_task_marker(node: &Node) -> bool {
    node.children.borrow().iter().any(|child| {
        let NodeData::Element { name, attrs, .. } = &child.data else {
            return false;
        };

        &*name.local == "input" && get_attr(&attrs.borrow(), "type") == Some("checkbox")
    })
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
