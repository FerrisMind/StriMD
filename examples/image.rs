use std::collections::{HashMap, HashSet};

use iced::{
    Element, Length, Task,
    widget::{self, image, text_editor::Content},
};
use strimd::{MarkState, MarkWidget};

use crate::image_loader::Image;

const TEXT: &str = r"Put some *image links* here. For example:

![](https://github.com/Mrmayman/quantumlauncher/raw/main/assets/icon/ql_logo.png)

> Note: For SVG support check the `large_readme` example

---

";

#[path = "shared/image_loader.rs"]
mod image_loader;

#[derive(Debug, Clone)]
enum Message {
    EditedText(widget::text_editor::Action),
    UpdateState(strimd::UpdateMsg),
    ImageDownloaded(Result<Image, String>),
}

#[derive(Clone)]
struct CachedImage {
    handle: image::Handle,
    intrinsic_size: Option<(f32, f32)>,
}

struct App {
    state: MarkState,
    editor: Content,

    images: HashMap<String, CachedImage>,
    images_in_progress: HashSet<String>,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EditedText(a) => {
                let is_edit = a.is_edit();
                self.editor.perform(a);
                if is_edit {
                    return self.reparse();
                }
            }
            Message::UpdateState(msg) => {
                self.state.update(msg);
            }
            Message::ImageDownloaded(res) => match res {
                Ok(image) => {
                    if image.is_svg {
                        eprintln!(
                            "SVG skipped in `image` example (see `large_readme` for SVG): {}",
                            image.url
                        );
                    } else {
                        self.images.insert(
                            image.url,
                            CachedImage {
                                handle: image::Handle::from_bytes(image.bytes),
                                intrinsic_size: image.intrinsic_size,
                            },
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Couldn't download image: {err}");
                }
            },
        }
        Task::none()
    }

    fn reparse(&mut self) -> Task<Message> {
        self.state = MarkState::with_html_and_markdown(&self.editor.text());
        self.download_images()
    }

    fn download_images(&mut self) -> Task<Message> {
        Task::batch(self.state.find_image_links().into_iter().map(|url| {
            if self.images_in_progress.insert(url.clone()) {
                Task::perform(
                    image_loader::download_image(url, Vec::new()),
                    Message::ImageDownloaded,
                )
            } else {
                Task::none()
            }
        }))
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        let editor = widget::text_editor(&self.editor)
            .on_action(Message::EditedText)
            .height(Length::Fill);

        widget::row![
            editor,
            widget::scrollable(
                MarkWidget::new(&self.state)
                    .on_updating_state(Message::UpdateState)
                    .on_drawing_image(|info| {
                        // Note: This example doesn't handle SVG images
                        // but they are possible to implement.
                        // - Check if url ends with ".svg"
                        // - Download to `widget::svg::Handle` and have a second HashMap
                        // - Usse the same logic elsewhere

                        if let Some(image) = self.images.get(info.url).cloned() {
                            let mut width = info.width;
                            let mut height = info.height;
                            if let Some((intrinsic_w, intrinsic_h)) = image.intrinsic_size
                                && intrinsic_w > 0.0
                                && intrinsic_h > 0.0
                            {
                                match (width, height) {
                                    (Some(w), None) => {
                                        height = Some(w * intrinsic_h / intrinsic_w);
                                    }
                                    (None, Some(h)) => {
                                        width = Some(h * intrinsic_w / intrinsic_h);
                                    }
                                    _ => {}
                                }
                            }
                            let mut img = widget::image(image.handle);
                            if let Some(w) = width {
                                img = img.width(w);
                            }
                            if let Some(h) = height {
                                img = img.height(h);
                            }
                            img.into()
                        } else {
                            widget::text(info.alt.unwrap_or(info.url).to_string()).into()
                        }
                    })
            )
            .width(Length::Fill),
        ]
        .spacing(10)
        .padding(10)
        .into()
    }
}

fn main() -> iced::Result {
    iced::application(
        || {
            let mut app = App {
                editor: Content::with_text(TEXT),
                state: MarkState::with_html_and_markdown(TEXT),
                images: HashMap::new(),
                images_in_progress: HashSet::new(),
            };
            let t = app.download_images();
            (app, t)
        },
        App::update,
        App::view,
    )
    .run()
}
