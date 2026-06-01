//! Mini chatbot for incremental Markdown rendering via StriMD streaming.
//!
//! ```bash
//! cargo run --example llm_chat --features stream,iced/tokio
//! ```

use iced::{
    Color, Element, Length, Subscription, Task,
    widget::{self, column, container, row, scrollable, text, text_input},
};
use strimd::{BlockKind, MarkState, MarkWidget, StreamDocument, StreamOptions, Style, UpdateMsg};

#[path = "shared/chat_stream.rs"]
mod chat_stream;
#[path = "shared/openai_compat.rs"]
mod openai_compat;

use chat_stream::{ActiveStream, ChatStreamEvent, chunk_by_words, stream_subscription};
use openai_compat::{ApiConfig, ChatMessage as ApiChatMessage};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

// #region agent log
fn agent_log(hypothesis_id: &str, location: &str, message: &str, data: &str) {
    use std::io::Write;
    let path = "/home/mod479711/Downloads/.cursor/debug-00d1aa.log";
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(
            f,
            r#"{{"sessionId":"00d1aa","hypothesisId":"{hypothesis_id}","location":"{location}","message":"{message}","data":{data},"timestamp":{ts}}}"#
        );
    }
}
// #endregion

/// MarkWidget uses iced theme text by default (dark on dark in a chat bubble).
fn assistant_mark_style() -> Style {
    Style {
        text_color: Some(Color::WHITE),
        link_color: Some(Color::from_rgb(0.55, 0.78, 1.0)),
        highlight_color: Some(Color::from_rgb(0.85, 0.72, 0.25)),
        inline_code_background: Some(Color::from_rgb(0.28, 0.28, 0.34)),
        inline_code_color: Some(Color::from_rgb(0.92, 0.94, 0.98)),
        code_block_background: Some(Color::from_rgb(0.1, 0.1, 0.12)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageRole {
    User,
    Assistant,
}

struct ChatMessage {
    id: u64,
    role: MessageRole,
    plain_text: String,
    stream: Option<StreamDocument>,
    mark_state: Option<MarkState>,
}

#[derive(Debug, Clone)]
enum Message {
    BaseUrlChanged(String),
    ApiKeyChanged(String),
    ModelChanged(String),
    PromptChanged(String),
    ToggleSettings,
    Send,
    SimulateTestMd,
    UpdateMark(u64, UpdateMsg),
    Stream(ChatStreamEvent),
}

struct App {
    base_url: String,
    api_key: String,
    model: String,
    prompt: String,
    settings_open: bool,
    busy: bool,
    next_id: u64,
    messages: Vec<ChatMessage>,
    active_stream: Option<ActiveStream>,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::BaseUrlChanged(value) => self.base_url = value,
            Message::ApiKeyChanged(value) => self.api_key = value,
            Message::ModelChanged(value) => self.model = value,
            Message::PromptChanged(value) => self.prompt = value,
            Message::ToggleSettings => self.settings_open = !self.settings_open,
            Message::Send => return self.send_prompt(),
            Message::SimulateTestMd => return self.simulate_test_md(),
            Message::UpdateMark(id, msg) => {
                if let Some(m) = self.messages.iter_mut().find(|m| m.id == id) {
                    if let Some(state) = &mut m.mark_state {
                        state.update(msg);
                    }
                }
            }
            Message::Stream(event) => return self.handle_stream(event),
        }
        Task::none()
    }

    fn send_prompt(&mut self) -> Task<Message> {
        if self.busy {
            return Task::none();
        }

        let prompt = self.prompt.trim().to_string();
        if prompt.is_empty() {
            return Task::none();
        }

        if self.model.trim().is_empty() {
            return Task::none();
        }

        self.push_user_message(prompt.clone());
        self.prompt.clear();

        let msg_id = self.push_assistant_placeholder();
        let api_messages = self.build_api_history();

        self.active_stream = Some(ActiveStream::Api {
            msg_id,
            config: ApiConfig {
                base_url: self.base_url.clone(),
                api_key: self.api_key.clone(),
                model: self.model.clone(),
            },
            messages: api_messages,
        });
        self.busy = true;
        Task::none()
    }

    fn simulate_test_md(&mut self) -> Task<Message> {
        if self.busy {
            return Task::none();
        }

        self.push_user_message("Simulate TEST.md (offline)".to_string());
        let msg_id = self.push_assistant_placeholder();

        let fixture = include_str!("assets/TEST.md");
        let chunks = chunk_by_words(fixture, 4);

        self.active_stream = Some(ActiveStream::Simulate { msg_id, chunks });
        self.busy = true;
        Task::none()
    }

    fn handle_stream(&mut self, event: ChatStreamEvent) -> Task<Message> {
        match event {
            ChatStreamEvent::Delta { msg_id, chunk } => {
                if let Some(message) = self.messages.iter_mut().find(|m| m.id == msg_id) {
                    if let (Some(stream), Some(mark)) =
                        (&mut message.stream, &mut message.mark_state)
                    {
                        let update = stream.append(&chunk);
                        mark.apply_stream_update(stream, &update);
                        message.plain_text.push_str(&chunk);

                        // #region agent log
                        let has_table = stream.blocks().any(|b| b.kind == BlockKind::Table)
                            || stream.pending().is_some_and(|p| p.kind == BlockKind::Table);
                        if has_table || message.plain_text.contains('|') {
                            let kinds: Vec<String> =
                                stream.blocks().map(|b| format!("{:?}", b.kind)).collect();
                            let pending = stream
                                .pending()
                                .map(|p| format!("{:?}", p.kind))
                                .unwrap_or_else(|| "none".into());
                            agent_log(
                                "A",
                                "llm_chat.rs:handle_stream",
                                "stream_state",
                                &format!(
                                    r#"{{"msg_id":{msg_id},"text_len":{},"has_table":{has_table},"kinds":{:?},"pending":"{pending}","patch":"{:?}"}}"#,
                                    message.plain_text.len(),
                                    kinds,
                                    update.patch
                                ),
                            );
                        }
                        // #endregion
                    }
                }
            }
            ChatStreamEvent::Done { msg_id } => {
                // #region agent log
                if let Some(message) = self.messages.iter().find(|m| m.id == msg_id) {
                    if let Some(stream) = &message.stream {
                        let has_table = stream.blocks().any(|b| b.kind == BlockKind::Table);
                        agent_log(
                            "B",
                            "llm_chat.rs:stream_done",
                            "final_state",
                            &format!(
                                r#"{{"msg_id":{msg_id},"text_len":{},"has_table":{has_table},"block_count":{}}}"#,
                                message.plain_text.len(),
                                stream.blocks().count()
                            ),
                        );
                    }
                }
                // #endregion
                self.active_stream = None;
                self.busy = false;
            }
            ChatStreamEvent::Error { msg_id, error } => {
                if let Some(message) = self.messages.iter_mut().find(|m| m.id == msg_id) {
                    let err_md = format!("\n\n**Error:** {error}");
                    message.plain_text.push_str(&err_md);
                    if let (Some(stream), Some(mark)) =
                        (&mut message.stream, &mut message.mark_state)
                    {
                        let update = stream.append(&err_md);
                        mark.apply_stream_update(stream, &update);
                    }
                }
                self.active_stream = None;
                self.busy = false;
            }
        }
        Task::none()
    }

    fn push_user_message(&mut self, text: String) {
        let id = self.next_id;
        self.next_id += 1;
        self.messages.push(ChatMessage {
            id,
            role: MessageRole::User,
            plain_text: text,
            stream: None,
            mark_state: None,
        });
    }

    fn push_assistant_placeholder(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let stream = StreamDocument::new(StreamOptions::chat());
        let mut mark_state = MarkState::default();
        mark_state.sync_from_stream(&stream);
        self.messages.push(ChatMessage {
            id,
            role: MessageRole::Assistant,
            plain_text: String::new(),
            stream: Some(stream),
            mark_state: Some(mark_state),
        });
        id
    }

    fn build_api_history(&self) -> Vec<ApiChatMessage> {
        self.messages
            .iter()
            .filter(|m| !m.plain_text.is_empty() || m.role == MessageRole::User)
            .map(|m| ApiChatMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: m.plain_text.clone(),
            })
            .collect()
    }

    fn view(&self) -> Element<'_, Message> {
        let settings = if self.settings_open {
            column![
                text("API settings").size(16),
                text_input("Base URL", &self.base_url)
                    .on_input(Message::BaseUrlChanged)
                    .padding(6),
                text_input("API key", &self.api_key)
                    .on_input(Message::ApiKeyChanged)
                    .secure(true)
                    .padding(6),
                text_input("Model", &self.model)
                    .on_input(Message::ModelChanged)
                    .padding(6),
                text("Examples: OpenAI default; Ollama http://localhost:11434/v1").size(12),
            ]
            .spacing(8)
            .padding(8)
        } else {
            column![]
        };

        let chat_items: Vec<Element<'_, Message>> = self
            .messages
            .iter()
            .map(|m| self.message_bubble(m))
            .collect();

        let chat = scrollable(
            column(chat_items)
                .spacing(12)
                .padding(8)
                .width(Length::Fill),
        )
        .height(Length::Fill);

        let prompt_row = row![
            text_input("Message…", &self.prompt)
                .on_input(Message::PromptChanged)
                .on_submit(Message::Send)
                .padding(8)
                .width(Length::Fill),
            widget::button("Send").on_press_maybe((!self.busy).then_some(Message::Send)),
            widget::button("Simulate TEST.md")
                .on_press_maybe((!self.busy).then_some(Message::SimulateTestMd)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        column![
            row![
                text("StriMD LLM chat — incremental Markdown").size(18),
                widget::button(if self.settings_open {
                    "Hide settings"
                } else {
                    "Settings"
                })
                .on_press(Message::ToggleSettings),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
            settings,
            chat,
            prompt_row,
        ]
        .spacing(10)
        .padding(12)
        .into()
    }

    fn message_bubble<'a>(&'a self, message: &'a ChatMessage) -> Element<'a, Message> {
        let align = match message.role {
            MessageRole::User => iced::Alignment::End,
            MessageRole::Assistant => iced::Alignment::Start,
        };

        let (bg, fg) = match message.role {
            MessageRole::User => (Color::from_rgb(0.2, 0.45, 0.85), Color::WHITE),
            MessageRole::Assistant => (Color::from_rgb(0.18, 0.18, 0.2), Color::WHITE),
        };

        let content: Element<'a, Message> = match message.role {
            MessageRole::User => text(&message.plain_text).color(fg).into(),
            MessageRole::Assistant => {
                if let Some(state) = &message.mark_state {
                    let msg_id = message.id;
                    container(
                        MarkWidget::new(state)
                            .style(assistant_mark_style())
                            .on_updating_state(move |msg| Message::UpdateMark(msg_id, msg)),
                    )
                    .width(Length::Fill)
                    .into()
                } else {
                    text(&message.plain_text).color(fg).into()
                }
            }
        };

        let bubble_width = match message.role {
            MessageRole::User => Length::Shrink,
            // Tables use Length::Fill cells; Shrink parent collapses them to a vertical strip.
            MessageRole::Assistant => Length::Fill,
        };

        container(content)
            .padding(12)
            .max_width(720.0)
            .style(move |_theme| container::Style {
                background: Some(bg.into()),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .width(bubble_width)
            .align_x(align)
            .into()
    }
}

fn app_subscription(app: &App) -> Subscription<Message> {
    stream_subscription(&app.active_stream).map(Message::Stream)
}

fn main() -> iced::Result {
    iced::application(
        || App {
            base_url: DEFAULT_BASE_URL.to_string(),
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            model: DEFAULT_MODEL.to_string(),
            prompt: String::new(),
            settings_open: true,
            busy: false,
            next_id: 1,
            messages: Vec::new(),
            active_stream: None,
        },
        App::update,
        App::view,
    )
    .subscription(app_subscription)
    .window_size([960.0, 720.0])
    .run()
}
