use pulldown_cmark::html;

use crate::core::block::{BlockContent, RenderBlock};
use crate::core::error::RenderError;

/// Write a slice of blocks to an HTML string using pulldown's HTML writer.
pub fn blocks_to_html(blocks: &[RenderBlock]) -> Result<String, RenderError> {
    let mut out = String::new();
    for block in blocks {
        match &block.content {
            BlockContent::Markdown(compiled) => {
                html::push_html(&mut out, compiled.events().iter().cloned());
            }
            BlockContent::Html(fragment) => {
                for root in fragment.roots() {
                    write_fragment_node(&mut out, fragment, *root)?;
                }
            }
            BlockContent::Code { lang, complete: _ } => {
                let lang_attr = lang
                    .as_deref()
                    .map(|l| format!(" class=\"language-{l}\""))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "<pre><code{lang_attr}>{}</code></pre>",
                    html_escape_text(&block.source)
                ));
            }
            BlockContent::PendingMarkdown => {}
            BlockContent::Unsupported { reason } => {
                out.push_str(&format!(
                    "<!-- unsupported: {} -->",
                    html_escape_text(&format!("{reason:?}"))
                ));
            }
            #[cfg(feature = "_legacy_comrak")]
            BlockContent::LegacyHtml(html) => out.push_str(html),
        }
    }
    Ok(out)
}

fn write_fragment_node(
    out: &mut String,
    fragment: &crate::html::fragment::HtmlFragment,
    id: crate::html::fragment::NodeId,
) -> Result<(), RenderError> {
    use crate::html::fragment::HtmlNode;
    let Some(node) = fragment.node(id) else {
        return Err(RenderError::new(format!("missing HtmlFragment node {id:?}")));
    };
    match node {
        HtmlNode::Text(text) => out.push_str(text),
        HtmlNode::Comment(comment) => {
            out.push_str("<!--");
            out.push_str(comment);
            out.push_str("-->");
        }
        HtmlNode::Element { tag, attrs, children } => {
            out.push('<');
            out.push_str(tag.as_str());
            for attr in attrs {
                out.push(' ');
                out.push_str(&attr.name);
                out.push_str("=\"");
                out.push_str(&html_escape_text(&attr.value));
                out.push('"');
            }
            out.push('>');
            for child in children {
                write_fragment_node(out, fragment, *child)?;
            }
            out.push_str("</");
            out.push_str(tag.as_str());
            out.push('>');
        }
    }
    Ok(())
}

fn html_escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}
