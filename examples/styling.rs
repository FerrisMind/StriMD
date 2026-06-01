use iced::{Element, Task, widget};
use strimd::{MarkState, MarkWidget, Style};

#[derive(Debug, Clone)]
enum Message {
    OpenLink(String),
}

struct App {
    state: MarkState,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenLink(url) => {
                let _ = open::that(url);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        widget::container(
            MarkWidget::new(&self.state)
                .on_clicking_link(Message::OpenLink)
                .style_link_button(|t, s| {
                    // Link buttons (non-span link content) styled as text links.
                    widget::button::text(t, s)
                })
                .style(Style {
                    text_color: Some(iced::Color::from_rgb8(220, 40, 40)),
                    link_color: Some(iced::Color::from_rgb8(200, 0, 200)),
                    highlight_color: Some(iced::Color::from_rgb8(0, 200, 80)),
                    inline_code_background: Some(iced::Color::from_rgb8(40, 40, 48)),
                    inline_code_color: Some(iced::Color::from_rgb8(180, 220, 255)),
                    code_block_background: Some(iced::Color::from_rgb8(32, 32, 40)),
                })
                .paragraph_spacing(20.0),
        )
        .padding(10)
        .into()
    }
}

fn main() {
    iced::application(
        || App {
            state: MarkState::with_html_and_markdown(STYLING_TEXT),
        },
        App::update,
        App::view,
    )
    .run()
    .unwrap();
}

const STYLING_TEXT: &str = include_str!("fixtures/styling.md");
