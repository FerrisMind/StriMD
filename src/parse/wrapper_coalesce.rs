//! Extend pulldown block slices through unclosed HTML containers (`<details>`, `<center>`, etc.).

use pulldown_cmark::{Event, Tag};

use crate::parse::content::extract_html_from_events;

/// True when the initial HTML chunk opens a container that closes in a later block.
#[must_use]
pub(crate) fn starts_unclosed_html_container(events: &[Event<'static>]) -> bool {
    container_state_from_events(events).needs_more()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ContainerState {
    details_depth: usize,
    center_depth: usize,
    div_depth: usize,
}

impl ContainerState {
    fn needs_more(self) -> bool {
        self.details_depth > 0 || self.center_depth > 0 || self.div_depth > 0
    }

    fn scan_events(&mut self, events: &[Event<'static>]) {
        let Some(html) = extract_html_from_events(events) else {
            return;
        };
        self.scan_html(&html);
    }

    fn scan_html(&mut self, html: &str) {
        let bytes = html.as_bytes();
        let mut i = 0usize;

        while i < bytes.len() {
            if bytes[i] != b'<' {
                i += 1;
                continue;
            }

            let Some(tag_end) = html[i..].find('>') else {
                break;
            };
            let tag = &html[i..=i + tag_end];
            if is_open_tag(tag, "details") {
                self.details_depth += 1;
            } else if is_close_tag(tag, "details") {
                self.details_depth = self.details_depth.saturating_sub(1);
            } else if is_open_tag(tag, "center") {
                self.center_depth += 1;
            } else if is_close_tag(tag, "center") {
                self.center_depth = self.center_depth.saturating_sub(1);
            } else if is_open_tag(tag, "div") {
                self.div_depth += 1;
            } else if is_close_tag(tag, "div") {
                self.div_depth = self.div_depth.saturating_sub(1);
            }
            i += tag_end + 1;
        }
    }
}

fn container_state_from_events(events: &[Event<'static>]) -> ContainerState {
    let mut state = ContainerState::default();
    state.scan_events(events);
    state
}

/// Extend `start..initial_end` until the combined HTML closes all opened containers.
#[must_use]
pub(crate) fn extend_through_unclosed_container(
    events: &[Event<'static>],
    start: usize,
    initial_end: usize,
) -> usize {
    let mut end = initial_end.max(start + 1);
    let mut state = container_state_from_events(&events[start..end]);

    while end < events.len() && state.needs_more() {
        let chunk_len = next_event_chunk_len(&events[end..]);
        let next_end = (end + chunk_len).min(events.len());
        state.scan_events(&events[end..next_end]);
        end = next_end;
    }
    end
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

fn is_open_tag(tag: &str, name: &str) -> bool {
    let tag = tag.trim();
    if !tag.starts_with('<') || tag.starts_with("</") {
        return false;
    }
    let Some(rest) = tag.get(1..) else {
        return false;
    };
    rest.len() >= name.len()
        && rest[..name.len()].eq_ignore_ascii_case(name)
        && tag_name_terminator(rest.as_bytes().get(name.len()).copied())
}

fn is_close_tag(tag: &str, name: &str) -> bool {
    let tag = tag.trim();
    let Some(rest) = tag.strip_prefix("</") else {
        return false;
    };
    rest.len() >= name.len()
        && rest[..name.len()].eq_ignore_ascii_case(name)
        && tag_name_terminator(rest.as_bytes().get(name.len()).copied())
}

fn tag_name_terminator(next: Option<u8>) -> bool {
    matches!(
        next,
        None | Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    fn parse_events(source: &str) -> Vec<Event<'static>> {
        Parser::new_ext(source, Options::all())
            .map(|event| event.into_static())
            .collect()
    }

    #[test]
    fn detects_unclosed_details_container() {
        let events = parse_events("<details>\nSummary\n");
        assert!(starts_unclosed_html_container(&events));
    }

    #[test]
    fn extends_until_details_closes() {
        let source = "<details>\n\nParagraph\n\n</details>\n";
        let events = parse_events(source);
        let Event::Start(tag) = &events[0] else {
            panic!("expected html block start");
        };
        let initial_end = block_extent_for_tag(&events, tag);
        let end = extend_through_unclosed_container(&events, 0, initial_end);
        assert_eq!(end, events.len());
    }

    #[test]
    fn plain_div_open_is_tracked_without_rendering_full_slice() {
        let state = {
            let mut state = ContainerState::default();
            state.scan_html("<div>");
            state
        };
        assert!(state.needs_more());
    }
}
