use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
};

use iced::{
    Alignment, Element, Task,
    widget::{self, button, image, svg},
};
use strimd::{MarkState, MarkWidget, UpdateMsg};
use tracing::{debug_span, info_span};

use crate::image_loader::Image;

#[path = "shared/image_loader.rs"]
mod image_loader;
#[path = "shared/profiling.rs"]
mod profiling;

fn main() -> iced::Result {
    let profiling = profiling::init_from_env("large_readme=info,strimd=info");
    let mut args = profiling.positional.into_iter();
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
                custom_source: None,
                state: MarkState::with_html_and_markdown(page.contents(initial_marker.as_deref())),
                images_normal: HashMap::new(),
                images_svg: HashMap::new(),
                images_in_progress: HashSet::new(),
                source_generation: 0,
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
    OpenLocalFile,
    LocalFilePicked(Option<Result<(PathBuf, String), String>>),
    ImageDownloaded {
        generation: u64,
        result: Result<Image, String>,
    },
}

impl Message {
    fn kind(&self) -> &'static str {
        match self {
            Self::UpdateState(_) => "update_state",
            Self::OpenLink(_) => "open_link",
            Self::ChangePage(_) => "change_page",
            Self::OpenLocalFile => "open_local_file",
            Self::LocalFilePicked(_) => "local_file_picked",
            Self::ImageDownloaded { .. } => "image_downloaded",
        }
    }
}

struct App {
    page: Page,
    section_marker: Option<String>,
    custom_source: Option<(PathBuf, String)>,
    state: MarkState,
    images_normal: HashMap<String, RasterImage>,
    images_svg: HashMap<String, SvgImage>,
    images_in_progress: HashSet<String>,
    source_generation: u64,
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
        let _span = debug_span!("large_readme.update", message = msg.kind()).entered();
        match msg {
            Message::UpdateState(msg) => self.state.update(msg),
            Message::OpenLink(link) => {
                _ = open_link(&link, self.link_base_dir());
            }
            Message::ChangePage(page) => {
                self.page = page;
                self.custom_source = None;
                return self.reload();
            }
            Message::OpenLocalFile => {
                return Task::perform(
                    async {
                        let picked = rfd::FileDialog::new()
                            .set_title("Open a local markdown file")
                            .add_filter("Text", &["md", "markdown", "txt", "html", "htm"])
                            .pick_file();

                        match picked {
                            Some(path) => {
                                let path_display = path.display().to_string();
                                Some(
                                    std::fs::read_to_string(&path)
                                        .map(|source| (path, source))
                                        .map_err(|err| format!("read {path_display}: {err}")),
                                )
                            }
                            None => None,
                        }
                    },
                    Message::LocalFilePicked,
                );
            }
            Message::LocalFilePicked(result) => {
                let Some(result) = result else {
                    return Task::none();
                };
                match result {
                    Ok((path, source)) => {
                        self.custom_source = Some((path, source));
                        return self.reload();
                    }
                    Err(err) => {
                        eprintln!("Couldn't open local file: {err}");
                    }
                }
            }
            Message::ImageDownloaded { generation, result } => {
                if generation != self.source_generation {
                    return Task::none();
                }
                match result {
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
                }
            }
        }
        Task::none()
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        let _span = debug_span!(
            "large_readme.view",
            page = %self.page,
            custom_source = self.custom_source.is_some()
        )
        .entered();
        let source_label = self.source_label();
        let page_selector = widget::row![
            "Page:",
            widget::pick_list(Page::ALL, Some(self.page), Message::ChangePage)
        ]
        .align_y(Alignment::Center)
        .spacing(10);

        widget::scrollable(
            widget::column![
                widget::row![
                    button("Open file...").on_press(Message::OpenLocalFile),
                    button("Reset sample").on_press(Message::ChangePage(self.page)),
                    widget::text(source_label).size(14),
                ]
                .align_y(Alignment::Center)
                .spacing(10),
                page_selector,
                widget::rule::horizontal(2),
                MarkWidget::new(&self.state)
                    .on_updating_state(Message::UpdateState)
                    .on_clicking_link(Message::OpenLink)
                    .on_drawing_image(|info| self.draw_image(info)),
            ]
            .spacing(10)
            .padding(10),
        )
        .into()
    }

    fn reload(&mut self) -> Task<Message> {
        let _span = info_span!(
            "large_readme.reload",
            page = %self.page,
            custom_source = self.custom_source.is_some(),
            generation = self.source_generation
        )
        .entered();
        self.source_generation = self.source_generation.wrapping_add(1);
        self.images_normal.clear();
        self.images_svg.clear();
        self.images_in_progress.clear();
        self.state = MarkState::with_html_and_markdown(self.source_text());
        self.download_images()
    }

    fn draw_image(&self, info: strimd::ImageInfo) -> Element<'static, Message> {
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

    fn download_images(&mut self) -> Task<Message> {
        let links: Vec<_> = self.state.find_image_links().into_iter().collect();
        let _span = info_span!(
            "large_readme.download_images",
            links = links.len(),
            generation = self.source_generation
        )
        .entered();
        let roots = self.image_search_roots();
        let generation = self.source_generation;
        Task::batch(links.into_iter().map(move |url| {
            if self.images_in_progress.insert(url.clone()) {
                Task::perform(
                    image_loader::download_image(url, roots.clone()),
                    move |result| Message::ImageDownloaded { generation, result },
                )
            } else {
                Task::none()
            }
        }))
    }

    fn source_text(&self) -> &str {
        if let Some((_, source)) = &self.custom_source {
            source.as_str()
        } else {
            self.page.contents(self.section_marker.as_deref())
        }
    }

    fn source_label(&self) -> String {
        if let Some((path, _)) = &self.custom_source {
            format!("Loaded: {}", path.display())
        } else {
            format!("Sample: {}", self.page)
        }
    }

    fn image_search_roots(&self) -> Vec<PathBuf> {
        if let Some((path, _)) = &self.custom_source {
            image_roots_from_path(path)
        } else {
            self.page.image_search_roots()
        }
    }

    fn link_base_dir(&self) -> Option<&Path> {
        self.custom_source
            .as_ref()
            .and_then(|(path, _)| path.parent())
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

fn image_roots_from_path(path: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(parent) = path.parent() {
        roots.push(parent.to_path_buf());

        for dir in parent.ancestors() {
            roots.push(dir.to_path_buf());
            let sibling_nova = dir.join("nova");
            if sibling_nova.exists() {
                roots.push(sibling_nova);
            }
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

fn open_link(url: &str, base_dir: Option<&Path>) -> Result<(), String> {
    let target = resolve_link_target(url, base_dir);
    for browser in ["google-chrome", "chromium", "chromium-browser", "xdg-open"] {
        if Command::new(browser).arg(&target).spawn().is_ok() {
            return Ok(());
        }
    }
    open::that(target).map_err(|err| err.to_string())
}

fn resolve_link_target(url: &str, base_dir: Option<&Path>) -> String {
    if looks_like_external_link(url) || url.starts_with('#') {
        return url.to_string();
    }

    let Some(base_dir) = base_dir else {
        return url.to_string();
    };

    base_dir.join(url).to_string_lossy().into_owned()
}

fn looks_like_external_link(url: &str) -> bool {
    url.starts_with("//")
        || url.starts_with("mailto:")
        || url.starts_with("tel:")
        || url.starts_with("file:")
        || url.contains("://")
}
