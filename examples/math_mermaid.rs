//! Demo: RaTeX math (inline + display) and Mermaid diagrams in MarkWidget.
//!
//! ```sh
//! cargo run --example math_mermaid --features "_iced_backend,math,mermaid"
//! ```

use iced::{Element, Task, widget};
use strimd::{Document, MarkState, MarkWidget, ParseProfile};

const DEMO: &str = r#"# Math and Mermaid

Inline: energy $E = mc^2$ and $\frac{a}{b}$.

Display:

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$

## Diagram

```mermaid
flowchart LR
    A[RaTeX] --> B[StriMD]
    B --> C[iced]
```
"#;

#[derive(Debug, Clone)]
enum Message {}

struct App {
    state: MarkState,
}

impl App {
    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        widget::container(MarkWidget::new(&self.state))
            .padding(20)
            .into()
    }
}

fn main() -> iced::Result {
    iced::application(
        || {
            let doc = Document::parse(DEMO, ParseProfile::GitHubPreview).expect("parse demo");
            App {
                state: MarkState::from_document(&doc),
            }
        },
        App::update,
        App::view,
    )
    .run()
}
