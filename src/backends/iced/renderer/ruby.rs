use iced::widget;

use crate::backends::iced::{
    MarkWidget, RubyMode,
    dom::DomRef,
    renderer::ValidTheme,
    structs::{ChildData, Emp, RenderedSpan},
};

struct RubyUnit<'a, M, T> {
    base: RenderedSpan<'a, M, T>,
    annotations: Vec<RenderedSpan<'a, M, T>>,
}

impl<M, T> Default for RubyUnit<'_, M, T> {
    fn default() -> Self {
        Self {
            base: RenderedSpan::None,
            annotations: Vec::new(),
        }
    }
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    pub(crate) fn draw_ruby(
        &mut self,
        node: DomRef<'_>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let units = self.ruby_collect_units(node, data);
        self.draw_ruby_units(units)
    }

    fn draw_ruby_units(&mut self, units: Vec<RubyUnit<'a, M, T>>) -> RenderedSpan<'a, M, T> {
        match self.ruby_mode {
            RubyMode::Ignore => units
                .into_iter()
                .fold(RenderedSpan::None, |acc, u| acc + u.base),

            RubyMode::Fallback => units.into_iter().fold(RenderedSpan::None, |acc, u| {
                let ann = u
                    .annotations
                    .into_iter()
                    .fold(RenderedSpan::None, |a, b| a + b);
                acc + u.base + ann
            }),

            RubyMode::Full => units.into_iter().fold(RenderedSpan::None, |acc, u| {
                let ann_block = u
                    .annotations
                    .into_iter()
                    .fold(RenderedSpan::None, |a, b| a + b);

                let unit = RenderedSpan::Elem(
                    widget::column![ann_block.render(), u.base.render()]
                        .align_x(iced::Alignment::Center)
                        .into(),
                    Emp::NonEmpty,
                );

                acc + unit
            }),
        }
    }

    fn ruby_collect_units(&mut self, node: DomRef<'_>, data: ChildData) -> Vec<RubyUnit<'a, M, T>> {
        let mut units: Vec<RubyUnit<'a, M, T>> = Vec::new();
        let mut current = RubyUnit::default();

        for child in node.children() {
            if child.is_useless() {
                continue;
            }

            match child.tag_name() {
                Some("rb") => {
                    if !matches!(current.base, RenderedSpan::None) {
                        units.push(current);
                        current = RubyUnit::default();
                    }
                    current.base = self.render_children(child, data);
                }
                Some("rt") => {
                    current.annotations.push(self.traverse_dom(child, data));
                }
                Some("rp") => {}
                _ => {
                    if !matches!(current.base, RenderedSpan::None) {
                        units.push(current);
                        current = RubyUnit::default();
                    }
                    current.base = self.traverse_dom(child, data);
                }
            }
        }

        if !matches!(current.base, RenderedSpan::None) {
            units.push(current);
        }
        units
    }
}
