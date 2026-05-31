use std::sync::Arc;

use pulldown_cmark::{Event, Tag};

use crate::core::block::{BlockContent, CompiledMarkdown};
use crate::html::fragment::HtmlFragment;

/// Derive block content from pulldown events, routing raw HTML to [`HtmlFragment`].
pub fn block_content_from_events(
    slice: &[Event<'static>],
    source: Arc<str>,
) -> BlockContent {
    for event in slice {
        match event {
            Event::Html(html) | Event::InlineHtml(html) => {
                return BlockContent::Html(HtmlFragment::from_html(html));
            }
            _ => {}
        }
    }
    if slice
        .first()
        .is_some_and(|e| matches!(e, Event::Start(Tag::HtmlBlock)))
    {
        return BlockContent::Html(HtmlFragment::from_html(&source));
    }
    BlockContent::Markdown(CompiledMarkdown::new(source, slice.to_vec()))
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
pub fn html_block_content(source: Arc<str>) -> BlockContent {
    BlockContent::Html(HtmlFragment::from_html(&source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    #[test]
    fn inline_html_routes_to_fragment() {
        let source = "text <span>x</span>";
        let events: Vec<_> = Parser::new_ext(source, Options::empty())
            .map(|e| e.into_static())
            .collect();
        let content = block_content_from_events(&events, Arc::from(source));
        assert!(matches!(content, BlockContent::Html(_)));
    }
}
