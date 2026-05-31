use std::sync::LazyLock;

use reqwest::Client;

// Just a quick and dirty setup for showcase

#[derive(Debug, Clone)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub url: String,
    #[allow(unused)]
    pub is_svg: bool,
    pub intrinsic_size: Option<(f32, f32)>,
}

pub async fn download_image(url: String) -> Result<Image, String> {
    static CLIENT: LazyLock<Client> = LazyLock::new(|| Client::new());
    let response = CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if !response.status().is_success() {
        Err(format!("Error {} from url: {url}", response.status()))
    } else {
        let mut bytes = response
            .bytes()
            .await
            .map_err(|err| err.to_string())?
            .to_vec();
        let mut is_svg = looks_like_svg(&bytes);
        let intrinsic_size = is_svg.then(|| svg_intrinsic_size(&bytes)).flatten();
        if is_svg
            && let Ok(rasterized) = rasterize_svg(&bytes)
        {
            bytes = rasterized;
            is_svg = false;
        }
        Ok(Image {
            intrinsic_size,
            is_svg,
            url,
            bytes,
        })
    }
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    trimmed.starts_with("<svg") || trimmed.starts_with("<?xml")
}

fn svg_intrinsic_size(bytes: &[u8]) -> Option<(f32, f32)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let svg_start = text.find("<svg")?;
    let svg_tail = &text[svg_start..];
    let tag_end = svg_tail.find('>')?;
    let head = &svg_tail[..tag_end];

    let width = svg_attr(head, "width").and_then(parse_svg_length);
    let height = svg_attr(head, "height").and_then(parse_svg_length);
    let view_box = svg_attr(head, "viewBox").and_then(parse_view_box);

    match (width, height, view_box) {
        (Some(w), Some(h), _) if w > 0.0 && h > 0.0 => Some((w, h)),
        (Some(w), None, Some((vb_w, vb_h))) if w > 0.0 && vb_w > 0.0 && vb_h > 0.0 => {
            Some((w, w * vb_h / vb_w))
        }
        (None, Some(h), Some((vb_w, vb_h))) if h > 0.0 && vb_w > 0.0 && vb_h > 0.0 => {
            Some((h * vb_w / vb_h, h))
        }
        (None, None, Some((vb_w, vb_h))) if vb_w > 0.0 && vb_h > 0.0 => Some((vb_w, vb_h)),
        _ => None,
    }
}

fn svg_attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=");
    let start = tag.find(&needle)? + needle.len();
    let rest = &tag[start..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = rest[1..].find(quote)? + 1;
    Some(&rest[1..end])
}

fn parse_svg_length(value: &str) -> Option<f32> {
    let numeric: String = value
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '+' | '-'))
        .collect();
    if numeric.is_empty() {
        None
    } else {
        numeric.parse().ok()
    }
}

fn parse_view_box(value: &str) -> Option<(f32, f32)> {
    let mut parts = value
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|part| !part.is_empty());
    let _min_x: f32 = parts.next()?.parse().ok()?;
    let _min_y: f32 = parts.next()?.parse().ok()?;
    let width: f32 = parts.next()?.parse().ok()?;
    let height: f32 = parts.next()?.parse().ok()?;
    Some((width, height))
}

fn rasterize_svg(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(bytes, &options)
        .map_err(|err| format!("svg parse failed: {err}"))?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| "svg pixmap allocation failed".to_string())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|err| format!("svg png encode failed: {err}"))
}
