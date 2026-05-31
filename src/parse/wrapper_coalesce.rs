//! Extend pulldown block slices through unclosed HTML containers (`<details>`, `<center>`, etc.).

use pulldown_cmark::{Event, Tag};

use crate::html::preprocess::normalize_legacy_alignment_wrappers;
use crate::parse::content::extract_html_from_events;

/// True when the initial HTML chunk opens a container that closes in a later block.
#[must_use]
pub(crate) fn starts_unclosed_html_container(events: &[Event<'static>]) -> bool {
    let Some(html) = extract_html_from_events(events) else {
        return false;
    };
    opens_unclosed_container(&normalize_legacy_alignment_wrappers(&html))
}

fn opens_unclosed_container(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    if lower.contains("<details") && !lower.contains("</details>") {
        return true;
    }
    if lower.contains("<center") && !lower.contains("</center>") {
        return true;
    }
    if lower.contains("<div") && lower.contains("align") && !lower.contains("</div>") {
        return true;
    }
    false
}

/// Extend `start..initial_end` until the combined HTML closes all opened containers.
#[must_use]
pub(crate) fn extend_through_unclosed_container(
    events: &[Event<'static>],
    start: usize,
    initial_end: usize,
) -> usize {
    let mut end = initial_end.max(start + 1);
    while end < events.len() && slice_needs_more(events, start, end) {
        end += next_event_chunk_len(&events[end..]);
    }
    end
}

fn slice_needs_more(events: &[Event<'static>], start: usize, end: usize) -> bool {
    let html = events_to_html(&events[start..end]);
    opens_unclosed_container(&normalize_legacy_alignment_wrappers(&html))
}

fn next_event_chunk_len(events: &[Event<'static>]) -> usize {
    let Some(first) = events.first() else {
        return 1;
    };
    match first {
        Event::Start(tag) => block_extent_for_tag(events, tag),
        Event::Html(_) | Event::InlineHtml(_) => html_event_extent(events),
        Event::Rule => 1,
        _ => paragraph_extent(events),
    }
}

fn block_extent_for_tag(events: &[Event<'static>], tag: &Tag<'_>) -> usize {
    let end_tag: pulldown_cmark::TagEnd = tag.clone().into();
    let mut depth = 0usize;
    for (i, event) in events.iter().enumerate() {
        match event {
            Event::Start(t) if t == tag => depth += 1,
            Event::End(t) if *t == end_tag => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
    }
    events.len()
}

fn html_event_extent(events: &[Event<'static>]) -> usize {
    let mut extent = 0usize;
    for event in events {
        match event {
            Event::Html(_) | Event::InlineHtml(_) => extent += 1,
            _ => break,
        }
    }
    extent.max(1)
}

fn paragraph_extent(events: &[Event<'static>]) -> usize {
    for (i, event) in events.iter().enumerate().skip(1) {
        if matches!(
            event,
            Event::Start(Tag::Paragraph)
                | Event::Start(Tag::Heading { .. })
                | Event::Start(Tag::CodeBlock(_))
                | Event::Start(Tag::List(_))
                | Event::Html(_)
                | Event::Rule
        ) {
            return i;
        }
    }
    events.len()
}

/// Render mixed markdown + HTML events to one HTML string (for coalesced wrapper blocks).
#[must_use]
pub(crate) fn events_to_html(events: &[Event<'static>]) -> String {
    let mut buf = String::new();
    pulldown_cmark::html::push_html(&mut buf, events.iter().cloned());
    buf
}

/// Coalesced wrapper block: HTML container chunk plus markdown until `</details>` / `</center>` / `</div>`.
#[must_use]
pub(crate) fn is_coalesced_wrapper_block(slice: &[Event<'static>]) -> bool {
    if !slice
        .iter()
        .any(|e| matches!(e, Event::Html(_) | Event::InlineHtml(_)))
    {
        return false;
    }
    slice_has_markdown_blocks(slice)
}

fn slice_has_markdown_blocks(slice: &[Event<'static>]) -> bool {
    slice.iter().any(|event| {
        matches!(
            event,
            Event::Start(Tag::Heading { .. })
                | Event::Start(Tag::List(_))
                | Event::Start(Tag::Paragraph)
                | Event::Start(Tag::BlockQuote(_))
                | Event::Start(Tag::Table(_))
                | Event::Start(Tag::CodeBlock(_))
        )
    })
}
