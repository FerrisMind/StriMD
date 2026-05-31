//! Optional deterministic HTML rewrite layer (`_html_preprocess` / `lol_html`).

use std::borrow::Cow;

/// Apply StriMD's fixed rewrite rules before html5ever parsing.
///
/// When `_html_preprocess` is disabled this is a no-op passthrough except for
/// legacy alignment normalization (`<div align="center">` → `<center>`).
#[must_use]
pub(crate) fn preprocess_raw_html(html: &str) -> Cow<'_, str> {
    let html = normalize_legacy_alignment_wrappers(html);
    #[cfg(feature = "_html_preprocess")]
    {
        match apply_lol_html_rewrites(html.as_ref()) {
            Ok(out) => Cow::Owned(out),
            Err(_) => html,
        }
    }
    #[cfg(not(feature = "_html_preprocess"))]
    {
        html
    }
}

/// html5ever drops obsolete `align` on `<div>`; map to `<center>` so iced alignment matches frostmark.
#[must_use]
pub(crate) fn normalize_legacy_alignment_wrappers(html: &str) -> Cow<'_, str> {
    if !html.contains("<div") || !html.contains("align") {
        return Cow::Borrowed(html);
    }

    let mut out = String::with_capacity(html.len());
    let mut align_div_depth: usize = 0;
    let mut i = 0;
    let bytes = html.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'<' {
            if let Some(tag_end) = html[i..].find('>') {
                let tag = &html[i..=i + tag_end];
                if is_align_center_div_open(tag) {
                    out.push_str("<center>");
                    align_div_depth = align_div_depth.saturating_add(1);
                    i = i + tag_end + 1;
                    continue;
                }
                if tag.starts_with("</div") && align_div_depth > 0 {
                    out.push_str("</center>");
                    align_div_depth -= 1;
                    i = i + tag_end + 1;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }

    if out == html {
        Cow::Borrowed(html)
    } else {
        Cow::Owned(out)
    }
}

fn is_align_center_div_open(tag: &str) -> bool {
    if !tag.starts_with("<div") {
        return false;
    }
    let lower = tag.to_ascii_lowercase();
    lower.contains("align=\"center\"")
        || lower.contains("align='center'")
        || lower.contains("align=\"centre\"")
        || lower.contains("align='centre'")
        || lower.contains("align=center")
        || lower.contains("align=centre")
}

#[cfg(feature = "_html_preprocess")]
fn apply_lol_html_rewrites(html: &str) -> Result<String, lol_html::errors::RewritingError> {
    use lol_html::{element, rewrite_str, RewriteStrSettings};

    const EVENT_ATTRS: &[&str] = &[
        "onclick", "ondblclick", "onmousedown", "onmouseup", "onmouseover", "onmouseout",
        "onkeydown", "onkeyup", "onkeypress", "onload", "onerror", "onfocus", "onblur",
    ];

    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![
                element!("*", |el| {
                    for attr in EVENT_ATTRS {
                        if el.get_attribute(attr).is_some() {
                            el.remove_attribute(attr);
                        }
                    }
                    Ok(())
                }),
                element!("img:not([loading])", |el| {
                    el.set_attribute("loading", "lazy").ok();
                    Ok(())
                }),
                element!("a[href^='http:']", |el| {
                    if let Some(href) = el.get_attribute("href") {
                        el.set_attribute("href", &href.replacen("http:", "https:", 1))
                            .ok();
                    }
                    Ok(())
                }),
            ],
            ..RewriteStrSettings::new()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div_align_center_becomes_center_element() {
        let out = normalize_legacy_alignment_wrappers(
            "<div align=\"center\"><h1>Title</h1></div>",
        );
        assert_eq!(out, "<center><h1>Title</h1></center>");
    }

    #[cfg(feature = "_html_preprocess")]
    #[test]
    fn rewrite_is_deterministic() {
        let input = "<a href=\"http://ex.com\">x</a><img src=\"a.png\">";
        let a = preprocess_raw_html(input);
        let b = preprocess_raw_html(input);
        assert_eq!(a, b);
    }

    #[cfg(feature = "_html_preprocess")]
    #[test]
    fn img_gets_lazy_loading() {
        let out = preprocess_raw_html("<img src=\"x.png\">");
        assert!(out.contains("loading=\"lazy\""), "out: {out}");
    }

    #[cfg(feature = "_html_preprocess")]
    #[test]
    fn strips_inline_event_handlers() {
        let out = preprocess_raw_html("<button onclick=\"alert(1)\">x</button>");
        assert!(!out.contains("onclick"), "out: {out}");
    }

    #[cfg(feature = "_html_preprocess")]
    #[test]
    fn upgrades_insecure_links() {
        let out = preprocess_raw_html("<a href=\"http://example.com\">link</a>");
        assert!(out.contains("https://example.com"), "out: {out}");
    }
}
