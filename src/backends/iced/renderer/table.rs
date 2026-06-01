use iced::{Length, widget};

use crate::backends::iced::{
    MarkWidget,
    dom::DomRef,
    renderer::ValidTheme,
    structs::{ChildAlignment, ChildData, ChildDataFlags, RenderedSpan},
};

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T>
where
    <T as widget::button::Catalog>::Class<'a>: From<widget::button::StyleFn<'a, T>>,
{
    pub(crate) fn draw_table(
        &mut self,
        node: DomRef<'_>,
        data: ChildData,
    ) -> RenderedSpan<'a, M, T> {
        let mut header_cells: Vec<RenderedSpan<'a, M, T>> = Vec::new();
        let mut column_alignments: Vec<Option<ChildAlignment>> = Vec::new();
        let mut body_rows: Vec<Vec<RenderedSpan<'a, M, T>>> = Vec::new();

        for section in node.children() {
            let Some(section_name) = section.tag_name() else {
                continue;
            };

            for row in section.children() {
                if row.tag_name() != Some("tr") {
                    continue;
                }

                let cells: Vec<_> = row
                    .children()
                    .into_iter()
                    .filter(|cell| matches!(cell.tag_name(), Some("th" | "td")))
                    .collect();

                if section_name == "thead" || (header_cells.is_empty() && body_rows.is_empty()) {
                    self.table_add_header_cell(
                        data,
                        &mut header_cells,
                        &mut column_alignments,
                        &cells,
                    );
                } else {
                    body_rows.push(
                        cells
                            .into_iter()
                            .map(|cell| self.render_children(cell, data))
                            .collect(),
                    );
                }
            }
        }

        let body: iced::Element<'a, M, T> = widget::column(
            body_rows
                .into_iter()
                .map(|row| draw_row(row, &column_alignments).into()),
        )
        .spacing(2)
        .into();

        widget::column![
            draw_row(header_cells, &column_alignments),
            widget::rule::horizontal(1),
            body,
        ]
        .spacing(4)
        .into()
    }

    fn table_add_header_cell(
        &mut self,
        data: ChildData,
        header_cells: &mut Vec<RenderedSpan<'a, M, T>>,
        column_alignments: &mut Vec<Option<ChildAlignment>>,
        cells: &[DomRef<'_>],
    ) {
        *column_alignments = cells
            .iter()
            .map(|cell| {
                let mut align = None;
                cell.for_each_attr(|name, value| {
                    if name == "align" {
                        align = match value {
                            "right" => Some(ChildAlignment::Right),
                            "center" | "centre" => Some(ChildAlignment::Center),
                            _ => None,
                        };
                    }
                });
                align
            })
            .collect();

        *header_cells = cells
            .iter()
            .map(|cell| self.render_children(*cell, data.insert(ChildDataFlags::BOLD)))
            .collect();
    }
}

fn draw_row<'a, M: Clone + 'static, T: ValidTheme + 'a>(
    cells: Vec<RenderedSpan<'a, M, T>>,
    column_alignments: &[Option<ChildAlignment>],
) -> widget::Row<'a, M, T> {
    widget::row(
        cells
            .into_iter()
            .enumerate()
            .map(|(i, cell)| make_cell(cell, column_alignments.get(i).copied().flatten()).into()),
    )
    .spacing(2)
}

fn make_cell<'a, M: Clone + 'static, T: ValidTheme + 'a>(
    content: RenderedSpan<'a, M, T>,
    align: Option<ChildAlignment>,
) -> widget::Column<'a, M, T> {
    let alignment: iced::Alignment = align.map_or(iced::Alignment::Start, ChildAlignment::into);

    widget::column![content.render()]
        .align_x(alignment)
        .padding(5)
        .width(Length::Fill)
}
