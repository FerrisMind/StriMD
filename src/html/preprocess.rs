//! Optional deterministic HTML rewrite layer (`_html_preprocess` / `lol_html`).

use std::borrow::Cow;

/// Apply StriMD's fixed rewrite rules before html5ever parsing.
///
/// When `_html_preprocess` is disabled this is a no-op passthrough.
#[must_use]
pub(crate) fn preprocess_raw_html(html: &str) -> Cow<'_, str> {
    #[cfg(feature = "_html_preprocess")]
    {
        match apply_lol_html_rewrites(html) {
            Ok(out) => Cow::Owned(out),
            Err(_) => Cow::Borrowed(html),
        }
    }
    #[cfg(not(feature = "_html_preprocess"))]
    {
        Cow::Borrowed(html)
    }
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

#[cfg(all(test, feature = "_html_preprocess"))]
mod tests {
    use super::*;

    #[test]
    fn rewrite_is_deterministic() {
        let input = "<a href=\"http://ex.com\">x</a><img src=\"a.png\">";
        let a = preprocess_raw_html(input);
        let b = preprocess_raw_html(input);
        assert_eq!(a, b);
    }

    #[test]
    fn img_gets_lazy_loading() {
        let out = preprocess_raw_html("<img src=\"x.png\">");
        assert!(out.contains("loading=\"lazy\""), "out: {out}");
    }

    #[test]
    fn strips_inline_event_handlers() {
        let out = preprocess_raw_html("<button onclick=\"alert(1)\">x</button>");
        assert!(!out.contains("onclick"), "out: {out}");
    }

    #[test]
    fn upgrades_insecure_links() {
        let out = preprocess_raw_html("<a href=\"http://example.com\">link</a>");
        assert!(out.contains("https://example.com"), "out: {out}");
    }
}
