use std::sync::Arc;

use pulldown_cmark::{CodeBlockKind, Event, Tag};

use crate::core::block::{BlockContent, CompiledMarkdown};
use crate::html::sanitize;
use crate::options::RawHtmlPolicy;

/// True when the slice is a single fenced or indented code block.
pub(crate) fn is_code_fence_slice(slice: &[Event<'static>]) -> bool {
    matches!(slice.first(), Some(Event::Start(Tag::CodeBlock(_))))
}

/// Concatenate `Text` events inside a code block slice.
pub(crate) fn code_text_from_events(slice: &[Event<'static>]) -> String {
    slice
        .iter()
        .filter_map(|event| match event {
            Event::Text(text) => Some(text.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Language tag from a fenced code block, if present.
pub(crate) fn code_lang_from_events(slice: &[Event<'static>]) -> Option<String> {
    let Event::Start(Tag::CodeBlock(kind)) = slice.first()? else {
        return None;
    };
    match kind {
        CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
        _ => None,
    }
}

/// Derive block content from pulldown events, routing raw HTML per [`RawHtmlPolicy`].
pub fn block_content_from_events(
    slice: &[Event<'static>],
    source: Arc<str>,
    raw_html: RawHtmlPolicy,
) -> BlockContent {
    if is_code_fence_slice(slice) {
        return BlockContent::Code {
            lang: code_lang_from_events(slice),
            complete: true,
        };
    }
    if is_standalone_html_block(slice)
        && let Some(html) = extract_html_from_events(slice)
    {
        return sanitize::block_content_from_raw_html(&html, raw_html);
    }
    BlockContent::Markdown(CompiledMarkdown::new(source, slice.to_vec()))
}

/// True when the slice is a raw HTML block, not Markdown with embedded inline HTML.
fn is_standalone_html_block(slice: &[Event<'static>]) -> bool {
    let Some(first) = slice.first() else {
        return false;
    };
    match first {
        Event::Start(Tag::HtmlBlock) | Event::Html(_) => {
            !slice.iter().any(|event| {
                matches!(
                    event,
                    Event::Start(Tag::Paragraph)
                        | Event::Start(Tag::Heading { .. })
                        | Event::Start(Tag::List(_))
                        | Event::Start(Tag::BlockQuote(_))
                        | Event::Start(Tag::Table(_))
                        | Event::Start(Tag::CodeBlock(_))
                        | Event::Start(Tag::FootnoteDefinition(_))
                        | Event::Start(Tag::Item)
                        | Event::Start(Tag::DefinitionList)
                        | Event::Start(Tag::DefinitionListTitle)
                        | Event::Start(Tag::DefinitionListDefinition)
                        | Event::Start(Tag::MetadataBlock(_))
                )
            })
        }
        _ => false,
    }
}

pub(crate) fn extract_html_from_events(slice: &[Event<'static>]) -> Option<String> {
    let mut html = String::new();
    for event in slice {
        match event {
            Event::Html(text) | Event::InlineHtml(text) => html.push_str(text),
            Event::Text(text) if matches!(slice.first(), Some(Event::Start(Tag::HtmlBlock))) => {
                html.push_str(text);
            }
            _ => {}
        }
    }
    if html.is_empty() { None } else { Some(html) }
}

/// Detect whether events or block kind indicate raw HTML content.
#[cfg(feature = "stream")]
pub fn events_contain_html(events: &[Event<'static>]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            Event::Html(_) | Event::InlineHtml(_) | Event::Start(Tag::HtmlBlock)
        )
    })
}

/// Build HTML fragment content from raw source when no compiled events exist.
#[cfg(feature = "stream")]
pub fn html_block_content(source: Arc<str>, raw_html: RawHtmlPolicy) -> BlockContent {
    sanitize::block_content_from_raw_html(&source, raw_html)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    #[test]
    fn multiline_html_block_preserves_details_children() {
        let source = "<details>\n<summary>Summary</summary>\nBody\n</details>\n";
        let events: Vec<_> = Parser::new_ext(source, pulldown_cmark::Options::all())
            .map(|e| e.into_static())
            .collect();
        let extracted = super::extract_html_from_events(&events).expect("html bytes");
        assert!(extracted.contains("summary"), "extracted: {extracted:?}");
        let content = block_content_from_events(
            &events,
            Arc::from(source),
            crate::options::RawHtmlPolicy::Preserve,
        );
        assert!(matches!(content, BlockContent::Html(_)));
    }

    #[cfg(feature = "static")]
    #[test]
    fn html_block_fixture_preserves_details_children() {
        let source = "<details><summary>Summary</summary>Body</details>";
        let events: Vec<_> = Parser::new_ext(source, pulldown_cmark::Options::all())
            .map(|e| e.into_static())
            .collect();
        let content = block_content_from_events(
            &events,
            Arc::from(source),
            crate::options::RawHtmlPolicy::Preserve,
        );
        let BlockContent::Html(fragment) = content else {
            panic!("expected html fragment");
        };
        let html = {
            #[cfg(feature = "static")]
            {
                use crate::html::writer;
                use crate::core::block::{BlockKind, BlockStatus, RenderBlock};
                use crate::core::ids::BlockId;
                let block = RenderBlock {
                    id: BlockId::new(1),
                    status: BlockStatus::Committed,
                    kind: BlockKind::HtmlBlock,
                    source: Arc::from(source),
                    content: BlockContent::Html(fragment),
                };
                writer::blocks_to_html(&[block]).expect("html")
            }
            #[cfg(not(feature = "static"))]
            String::new()
        };
        assert!(html.contains("summary"), "html: {html}");
    }

    /// Regression: README/doctest sample must stay Markdown (not a lone `<b>` Html block).
    #[test]
    fn readme_doctest_sample_stays_markdown() {
        let text = "Hello from **markdown** and <b>HTML</b>!";
        let events: Vec<_> = Parser::new_ext(text, pulldown_cmark::Options::all())
            .map(|e| e.into_static())
            .collect();
        let slice = events.as_slice();
        let content = block_content_from_events(
            slice,
            Arc::from(text),
            crate::options::RawHtmlPolicy::Preserve,
        );
        let BlockContent::Markdown(compiled) = content else {
            panic!("expected markdown block for readme sample, got {content:?}");
        };
        let html = {
            let mut buf = String::new();
            pulldown_cmark::html::push_html(&mut buf, compiled.events().iter().cloned());
            buf
        };
        assert!(html.contains("Hello"));
        assert!(html.contains("markdown"));
        assert!(html.contains("HTML"));
    }

    #[test]
    fn inline_html_stays_in_markdown_block() {
        let source = "text <span>x</span> and more";
        let events: Vec<_> = Parser::new_ext(source, Options::empty())
            .map(|e| e.into_static())
            .collect();
        let content = block_content_from_events(
            &events,
            Arc::from(source),
            crate::options::RawHtmlPolicy::Preserve,
        );
        assert!(matches!(content, BlockContent::Markdown(_)));
    }

    #[test]
    fn code_fence_routes_to_block_content_code() {
        let source = "```rust\nfn main() {}\n```\n";
        let events: Vec<_> = Parser::new_ext(source, Options::all())
            .map(|e| e.into_static())
            .collect();
        let content = block_content_from_events(
            &events,
            Arc::from(source),
            crate::options::RawHtmlPolicy::Preserve,
        );
        assert!(matches!(
            content,
            BlockContent::Code {
                lang: Some(ref l),
                complete: true,
            } if l == "rust"
        ));
        assert_eq!(code_text_from_events(&events), "fn main() {}\n");
    }

    #[test]
    fn inline_html_routes_to_fragment_for_html_only_block() {
        let source = "<details><summary>x</summary></details>";
        let events: Vec<_> = Parser::new_ext(source, Options::all())
            .map(|e| e.into_static())
            .collect();
        let content = block_content_from_events(
            &events,
            Arc::from(source),
            crate::options::RawHtmlPolicy::Preserve,
        );
        assert!(matches!(content, BlockContent::Html(_)));
    }
}
