//! Mini chatbot for incremental Markdown rendering via StriMD streaming.
//!
//! ```bash
//! cargo run --example llm_chat
//! ```

use iced::{
    Color, Element, Length, Subscription, Task, clipboard, time,
    widget::{self, column, container, row, scrollable, text, text_input},
};
use std::time::Duration;
use strimd::{MarkState, MarkWidget, StreamDocument, StreamOptions, Style, UpdateMsg};
use tracing::{debug_span, info_span};

#[path = "shared/chat_stream.rs"]
mod chat_stream;
#[path = "shared/openai_compat.rs"]
mod openai_compat;
#[path = "shared/profiling.rs"]
mod profiling;

use chat_stream::{ActiveStream, ChatStreamEvent, chunk_by_words, stream_subscription};
use openai_compat::{ApiConfig, ChatMessage as ApiChatMessage};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const STREAM_FLUSH_INTERVAL_MS: u64 = 200;
const STREAM_FLUSH_BYTES: usize = 1024;

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
    CopyText(String),
    FlushPending,
    UpdateMark(u64, UpdateMsg),
    Stream(ChatStreamEvent),
}

impl Message {
    fn kind(&self) -> &'static str {
        match self {
            Self::BaseUrlChanged(_) => "base_url_changed",
            Self::ApiKeyChanged(_) => "api_key_changed",
            Self::ModelChanged(_) => "model_changed",
            Self::PromptChanged(_) => "prompt_changed",
            Self::ToggleSettings => "toggle_settings",
            Self::Send => "send",
            Self::SimulateTestMd => "simulate_test_md",
            Self::CopyText(_) => "copy_text",
            Self::FlushPending => "flush_pending",
            Self::UpdateMark(_, _) => "update_mark",
            Self::Stream(_) => "stream",
        }
    }
}

struct PendingDelta {
    msg_id: u64,
    text: String,
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
    pending_delta: Option<PendingDelta>,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        let _span = debug_span!("llm_chat.update", message = message.kind()).entered();
        match message {
            Message::BaseUrlChanged(value) => self.base_url = value,
            Message::ApiKeyChanged(value) => self.api_key = value,
            Message::ModelChanged(value) => self.model = value,
            Message::PromptChanged(value) => self.prompt = value,
            Message::ToggleSettings => self.settings_open = !self.settings_open,
            Message::Send => return self.send_prompt(),
            Message::SimulateTestMd => return self.simulate_test_md(),
            Message::CopyText(content) => return Self::copy_text(content),
            Message::FlushPending => self.flush_pending_delta(),
            Message::UpdateMark(id, msg) => {
                if let Some(m) = self.messages.iter_mut().find(|m| m.id == id)
                    && let Some(state) = &mut m.mark_state
                {
                    state.update(msg);
                }
            }
            Message::Stream(event) => return self.handle_stream(event),
        }
        Task::none()
    }

    fn send_prompt(&mut self) -> Task<Message> {
        let _span = info_span!("llm_chat.send_prompt").entered();
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
        self.pending_delta = None;
        Task::none()
    }

    fn simulate_test_md(&mut self) -> Task<Message> {
        let _span = info_span!("llm_chat.simulate_test_md").entered();
        if self.busy {
            return Task::none();
        }

        self.push_user_message("Simulate TEST.md (offline)".to_string());
        let msg_id = self.push_assistant_placeholder();

        let fixture = include_str!("assets/TEST.md");
        let chunks = chunk_by_words(fixture, 4);

        self.active_stream = Some(ActiveStream::Simulate { msg_id, chunks });
        self.busy = true;
        self.pending_delta = None;
        Task::none()
    }

    fn copy_text(content: String) -> Task<Message> {
        clipboard::write::<Message>(content)
    }

    fn handle_stream(&mut self, event: ChatStreamEvent) -> Task<Message> {
        let event_kind = match &event {
            ChatStreamEvent::Delta { .. } => "delta",
            ChatStreamEvent::Done => "done",
            ChatStreamEvent::Error { .. } => "error",
        };
        let _span = debug_span!("llm_chat.handle_stream", event = event_kind).entered();
        match event {
            ChatStreamEvent::Delta { msg_id, chunk } => {
                self.buffer_delta(msg_id, &chunk);
                if self
                    .pending_delta
                    .as_ref()
                    .is_some_and(|pending| pending.text.len() >= STREAM_FLUSH_BYTES)
                {
                    self.flush_pending_delta();
                }
            }
            ChatStreamEvent::Done => {
                self.flush_pending_delta();
                self.active_stream = None;
                self.busy = false;
            }
            ChatStreamEvent::Error { msg_id, error } => {
                self.flush_pending_delta();
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

    fn buffer_delta(&mut self, msg_id: u64, chunk: &str) {
        match &mut self.pending_delta {
            Some(pending) if pending.msg_id == msg_id => pending.text.push_str(chunk),
            Some(_) => {
                self.flush_pending_delta();
                self.pending_delta = Some(PendingDelta {
                    msg_id,
                    text: chunk.to_string(),
                });
            }
            None => {
                self.pending_delta = Some(PendingDelta {
                    msg_id,
                    text: chunk.to_string(),
                });
            }
        }
    }

    fn flush_pending_delta(&mut self) {
        let _span = debug_span!(
            "llm_chat.flush_pending_delta",
            has_pending = self.pending_delta.is_some()
        )
        .entered();
        let Some(PendingDelta { msg_id, text }) = self.pending_delta.take() else {
            return;
        };

        if let Some(message) = self.messages.iter_mut().find(|m| m.id == msg_id)
            && let (Some(stream), Some(mark)) = (&mut message.stream, &mut message.mark_state)
        {
            let update = stream.append(&text);
            mark.apply_stream_update(stream, &update);
            message.plain_text.push_str(&text);
        }
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
        let _span = debug_span!(
            "llm_chat.view",
            messages = self.messages.len(),
            busy = self.busy
        )
        .entered();
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
                let body: Element<'a, Message> = if let Some(state) = &message.mark_state {
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
                };

                column![
                    row![
                        widget::Space::new().width(Length::Fill),
                        widget::button("Copy")
                            .on_press(Message::CopyText(message.plain_text.clone()))
                    ]
                    .align_y(iced::Alignment::Center),
                    body,
                ]
                .spacing(8)
                .into()
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
    let stream = stream_subscription(&app.active_stream).map(Message::Stream);
    if app.busy {
        Subscription::batch([
            stream,
            time::every(Duration::from_millis(STREAM_FLUSH_INTERVAL_MS)).map(|_| Message::FlushPending),
        ])
    } else {
        stream
    }
}

fn main() -> iced::Result {
    let profiling = profiling::init_from_env("llm_chat=info,strimd=info");
    if !profiling.positional.is_empty() {
        eprintln!(
            "Ignoring unsupported llm_chat args: {}",
            profiling.positional.join(" ")
        );
    }

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
            pending_delta: None,
        },
        App::update,
        App::view,
    )
    .subscription(app_subscription)
    .window_size([960.0, 720.0])
    .run()
}
