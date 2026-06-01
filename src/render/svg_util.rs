//! SVG intrinsic size helpers for diagram and math widgets.

/// Parsed SVG payload ready for iced `widget::svg` or rasterization.
#[derive(Debug, Clone)]
pub struct SvgArtifact {
    pub bytes: Vec<u8>,
    pub width: f32,
    pub height: f32,
}

impl SvgArtifact {
    #[must_use]
    pub fn from_svg_string(svg: String) -> Self {
        let (width, height) = svg_intrinsic_size(&svg).unwrap_or((400.0, 200.0));
        Self {
            bytes: svg.into_bytes(),
            width,
            height,
        }
    }
}

/// Scale intrinsic SVG size to a target height (preserving aspect ratio).
#[must_use]
pub fn svg_dimensions_for_height(width: f32, height: f32, target_height: f32) -> (f32, f32) {
    if width <= 0.0 || height <= 0.0 || target_height <= 0.0 {
        return (width, height);
    }
    let scale = target_height / height;
    (width * scale, target_height)
}

/// Scale down (never up) so the SVG fits inside `max_width` × `max_height`.
#[must_use]
pub fn svg_dimensions_to_fit(
    width: f32,
    height: f32,
    max_width: f32,
    max_height: f32,
) -> (f32, f32) {
    if width <= 0.0 || height <= 0.0 || max_width <= 0.0 || max_height <= 0.0 {
        return (width, height);
    }
    let scale = (max_width / width).min(max_height / height).min(1.0);
    (width * scale, height * scale)
}

#[must_use]
pub fn svg_intrinsic_size(svg: &str) -> Option<(f32, f32)> {
    let bytes = svg.as_bytes();
    svg_intrinsic_size_bytes(bytes)
}

#[must_use]
pub fn svg_intrinsic_size_bytes(bytes: &[u8]) -> Option<(f32, f32)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_to_target_height() {
        assert_eq!(svg_dimensions_for_height(80.0, 40.0, 16.0), (32.0, 16.0));
    }

    #[test]
    fn fits_inside_box_without_upscaling() {
        assert_eq!(svg_dimensions_to_fit(100.0, 50.0, 200.0, 200.0), (100.0, 50.0));
        assert_eq!(svg_dimensions_to_fit(400.0, 200.0, 200.0, 100.0), (200.0, 100.0));
    }

    #[test]
    fn intrinsic_size_from_view_box() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50"></svg>"#;
        assert_eq!(svg_intrinsic_size(svg), Some((100.0, 50.0)));
    }
}
