use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Display,
    path::PathBuf,
    process::Command,
};

use iced::{
    Alignment, Element, Task,
    widget::{self, image, svg},
};
use strimd::{MarkState, MarkWidget, UpdateMsg};

use crate::image_loader::Image;

#[path = "shared/image_loader.rs"]
mod image_loader;

fn main() -> iced::Result {
    let mut args = env::args().skip(1);
    let initial_page = args
        .next()
        .as_deref()
        .and_then(Page::from_arg)
        .unwrap_or(Page::TestSuite);
    let initial_marker = args.next();
    iced::application(
        move || {
            let page = initial_page;
            let mut app = App {
                page,
                section_marker: initial_marker.clone(),
                state: MarkState::with_html_and_markdown(page.contents(initial_marker.as_deref())),
                images_normal: HashMap::new(),
                images_svg: HashMap::new(),
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

#[derive(Debug, Clone)]
enum Message {
    UpdateState(UpdateMsg),
    OpenLink(String),
    ChangePage(Page),
    ImageDownloaded(Result<Image, String>),
}

struct App {
    page: Page,
    section_marker: Option<String>,
    state: MarkState,
    images_normal: HashMap<String, RasterImage>,
    images_svg: HashMap<String, SvgImage>,
    images_in_progress: HashSet<String>,
}

#[derive(Clone)]
struct RasterImage {
    handle: image::Handle,
    intrinsic_size: Option<(f32, f32)>,
}

#[derive(Clone)]
struct SvgImage {
    handle: svg::Handle,
    intrinsic_size: Option<(f32, f32)>,
}

impl App {
    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::UpdateState(msg) => self.state.update(msg),
            Message::OpenLink(link) => {
                _ = open_link(&link);
            }
            Message::ChangePage(page) => {
                self.page = page;
                return self.reload();
            }
            Message::ImageDownloaded(res) => match res {
                Ok(image) => {
                    if image.is_svg {
                        self.images_svg.insert(
                            image.url,
                            SvgImage {
                                handle: svg::Handle::from_memory(image.bytes),
                                intrinsic_size: image.intrinsic_size,
                            },
                        );
                    } else {
                        self.images_normal.insert(
                            image.url,
                            RasterImage {
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

    fn view<'a>(&'a self) -> Element<'a, Message> {
        let page_selector = widget::row![
            "Page:",
            widget::pick_list(Page::ALL, Some(self.page), |s| Message::ChangePage(s))
        ]
        .align_y(Alignment::Center)
        .spacing(10);

        widget::scrollable(
            widget::column![
                page_selector,
                widget::rule::horizontal(2),
                MarkWidget::new(&self.state)
                    .on_updating_state(|msg| Message::UpdateState(msg))
                    .on_clicking_link(Message::OpenLink)
                    .on_drawing_image(|info| self.draw_image(info)),
            ]
            .spacing(10)
            .padding(10),
        )
        .into()
    }

    fn reload(&mut self) -> Task<Message> {
        self.state =
            MarkState::with_html_and_markdown(self.page.contents(self.section_marker.as_deref()));
        self.download_images()
    }

    fn draw_image(&self, info: strimd::ImageInfo) -> Element<'static, Message> {
        if badge_label(&info).is_some() {
            return self.draw_badge(info);
        }

        if let Some(image) = self.images_normal.get(info.url).cloned() {
            let mut width = info.width;
            let mut height = info.height;
            if let Some((intrinsic_w, intrinsic_h)) = image.intrinsic_size
                && intrinsic_w > 0.0
                && intrinsic_h > 0.0
            {
                match (width, height) {
                    (Some(w), None) => height = Some(w * intrinsic_h / intrinsic_w),
                    (None, Some(h)) => width = Some(h * intrinsic_w / intrinsic_h),
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
        } else if let Some(image) = self.images_svg.get(info.url).cloned() {
            let mut width = info.width;
            let mut height = info.height;
            if let Some((intrinsic_w, intrinsic_h)) = image.intrinsic_size
                && intrinsic_w > 0.0
                && intrinsic_h > 0.0
            {
                match (width, height) {
                    (Some(w), None) => height = Some(w * intrinsic_h / intrinsic_w),
                    (None, Some(h)) => width = Some(h * intrinsic_w / intrinsic_h),
                    _ => {}
                }
            }

            let mut img = widget::svg(image.handle);
            if let Some(w) = width {
                img = img.width(w);
            }
            if let Some(h) = height {
                img = img.height(h);
            }
            img.into()
        } else {
            widget::text(missing_image_fallback(info)).into()
        }
    }

    fn draw_badge(&self, info: strimd::ImageInfo) -> Element<'static, Message> {
        let label = badge_label(&info).unwrap_or("badge");
        let (left, right) = badge_colors(info.url);
        let left_text = if label.eq_ignore_ascii_case("gfm badge") {
            "GFM"
        } else {
            "Docs"
        };
        let right_text = if label.eq_ignore_ascii_case("gfm badge") {
            "spec"
        } else {
            "guide"
        };

        let left_chip =
            widget::container(widget::text(left_text).size(11).color(iced::Color::WHITE))
                .padding([2, 6])
                .style(move |_| widget::container::Style {
                    background: Some(left.into()),
                    text_color: Some(iced::Color::WHITE),
                    border: iced::Border {
                        radius: iced::border::top_left(4.0)
                            .bottom_left(4.0)
                            .top_right(0.0)
                            .bottom_right(0.0),
                        width: 0.0,
                        color: iced::Color::TRANSPARENT,
                    },
                    ..widget::container::Style::default()
                });
        let right_chip =
            widget::container(widget::text(right_text).size(11).color(iced::Color::WHITE))
                .padding([2, 6])
                .style(move |_| widget::container::Style {
                    background: Some(right.into()),
                    text_color: Some(iced::Color::WHITE),
                    border: iced::Border {
                        radius: iced::border::top_left(0.0)
                            .bottom_left(0.0)
                            .top_right(4.0)
                            .bottom_right(4.0),
                        width: 0.0,
                        color: iced::Color::TRANSPARENT,
                    },
                    ..widget::container::Style::default()
                });

        widget::row![left_chip, right_chip].spacing(0).into()
    }

    fn download_images(&mut self) -> Task<Message> {
        let roots = self.page.image_search_roots();
        Task::batch(self.state.find_image_links().into_iter().map(|url| {
            if self.images_in_progress.insert(url.clone()) {
                Task::perform(
                    image_loader::download_image(url, roots.clone()),
                    Message::ImageDownloaded,
                )
            } else {
                Task::none()
            }
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    TestSuite,
    QuantumLauncher,
    Mpf,
}

impl Page {
    const ALL: [Self; 3] = [Self::TestSuite, Self::QuantumLauncher, Self::Mpf];

    fn from_arg(arg: &str) -> Option<Self> {
        match arg.to_ascii_lowercase().as_str() {
            "test" | "testsuite" | "test-suite" => Some(Self::TestSuite),
            "ql" | "quantumlauncher" | "quantum-launcher" => Some(Self::QuantumLauncher),
            "mpf" => Some(Self::Mpf),
            _ => None,
        }
    }

    fn raw_contents(&self) -> &'static str {
        match self {
            Page::TestSuite => include_str!("assets/TEST.md"),
            Page::QuantumLauncher => include_str!("assets/QL_README.md"),
            Page::Mpf => include_str!("assets/MPF.md"),
        }
    }

    fn contents(&self, marker: Option<&str>) -> &str {
        let source = self.raw_contents();
        let Some(marker) = marker else {
            return source;
        };
        source
            .find(marker)
            .map(|index| &source[index..])
            .unwrap_or(source)
    }

    fn image_search_roots(&self) -> Vec<PathBuf> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let asset_dir = manifest_dir.join("examples/assets");
        let mut roots = vec![asset_dir];

        for dir in manifest_dir.ancestors() {
            roots.push(dir.to_path_buf());
            let sibling_nova = dir.join("nova");
            if sibling_nova.exists() {
                roots.push(sibling_nova);
            }
        }

        if let Ok(current_dir) = env::current_dir() {
            for dir in current_dir.ancestors() {
                roots.push(dir.to_path_buf());
                let sibling_nova = dir.join("nova");
                if sibling_nova.exists() {
                    roots.push(sibling_nova);
                }
            }
        }

        roots.sort();
        roots.dedup();
        roots
    }
}

impl Display for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Page::TestSuite => "Test Suite",
                Page::QuantumLauncher => "QuantumLauncher",
                Page::Mpf => "MPF",
            }
        )
    }
}

fn missing_image_fallback(info: strimd::ImageInfo<'_>) -> String {
    let label = info
        .alt
        .filter(|alt| !alt.trim().is_empty())
        .unwrap_or(info.url);
    format!("[image unavailable: {label}]")
}

fn badge_label<'a>(info: &'a strimd::ImageInfo<'a>) -> Option<&'a str> {
    let lower = info.url.to_ascii_lowercase();
    let looks_like_badge = lower.contains("badge")
        || lower.contains("img.shields.io")
        || lower.contains("shields.io/");
    if !looks_like_badge {
        return None;
    }

    info.alt
        .filter(|label| !label.trim().is_empty())
        .or_else(|| {
            if lower.contains("badge-gfm") {
                Some("GFM badge")
            } else if lower.contains("badge-docs") {
                Some("Docs badge")
            } else {
                None
            }
        })
}

fn badge_colors(url: &str) -> (iced::Color, iced::Color) {
    if url.contains("badge-gfm") {
        (
            iced::Color::from_rgb8(0x1F, 0x29, 0x37),
            iced::Color::from_rgb8(0x25, 0x63, 0xEB),
        )
    } else {
        (
            iced::Color::from_rgb8(0x33, 0x41, 0x55),
            iced::Color::from_rgb8(0x16, 0xA3, 0x4A),
        )
    }
}

fn open_link(url: &str) -> Result<(), String> {
    for browser in ["google-chrome", "chromium", "chromium-browser", "xdg-open"] {
        if Command::new(browser).arg(url).spawn().is_ok() {
            return Ok(());
        }
    }
    open::that(url).map_err(|err| err.to_string())
}
