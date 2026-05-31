use std::sync::Arc;

use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use crate::core::block::{
    BlockContent, BlockKind, BlockStatus, RenderBlock,
};
use crate::core::error::ParseError;
use crate::core::ids::BlockId;
use crate::options::ParseOptions;
use crate::parse::content::block_content_from_events;
use crate::profile::ParseProfile;

/// Collect pulldown events into backend-agnostic [`RenderBlock`] values.
pub fn parse_blocks(
    source: &str,
    profile: ParseProfile,
    options: &ParseOptions,
) -> Result<Vec<RenderBlock>, ParseError> {
    let _ = options;
    let parser = Parser::new_ext(source, profile.pulldown_options());
    let events: Vec<Event<'_>> = parser.collect();
    let events = events
        .into_iter()
        .map(|event| event.into_static())
        .collect::<Vec<_>>();

    Ok(group_events_into_blocks(source, events))
}

fn group_events_into_blocks(
    source: &str,
    events: Vec<Event<'static>>,
) -> Vec<RenderBlock> {
    let source_arc = Arc::<str>::from(source);
    let mut blocks = Vec::new();
    let mut next_id = 1u64;
    let mut index = 0usize;

    while index < events.len() {
        let (kind, end) = classify_block_start(&events[index..]);
        let end = index + end.max(1);
        let slice = &events[index..end];
        let block_source = Arc::<str>::from(event_slice_source(source, slice));
        let content = block_content_from_events(slice, block_source.clone());

        blocks.push(RenderBlock {
            id: BlockId::new(next_id),
            status: BlockStatus::Committed,
            kind,
            source: block_source,
            content,
        });
        next_id += 1;
        index = end;
    }

    if blocks.is_empty() && !source.is_empty() {
        blocks.push(RenderBlock {
            id: BlockId::new(next_id),
            status: BlockStatus::Committed,
            kind: BlockKind::Paragraph,
            source: source_arc.clone(),
            content: BlockContent::Markdown(crate::core::block::CompiledMarkdown::new(
                source_arc,
                events,
            )),
        });
    }

    blocks
}

fn classify_block_start(events: &[Event<'static>]) -> (BlockKind, usize) {
    let Some(first) = events.first() else {
        return (BlockKind::Unknown, 0);
    };

    match first {
        Event::Start(tag) => {
            let len = block_extent_for_tag(events, tag);
            (tag_to_kind(tag), len)
        }
        Event::Html(_) => (BlockKind::HtmlBlock, 1),
        Event::InlineHtml(_) => (BlockKind::HtmlBlock, 1),
        Event::Rule => (BlockKind::ThematicBreak, 1),
        Event::FootnoteReference(_) => (BlockKind::FootnoteDefinition, 1),
        Event::DisplayMath(_) => (BlockKind::MathBlock, 1),
        _ => (BlockKind::Paragraph, paragraph_extent(events)),
    }
}

fn tag_to_kind(tag: &Tag<'_>) -> BlockKind {
    match tag {
        Tag::Paragraph => BlockKind::Paragraph,
        Tag::Heading { .. } => BlockKind::Heading,
        Tag::CodeBlock(_) => BlockKind::CodeFence,
        Tag::List(_) => BlockKind::List,
        Tag::BlockQuote(_) => BlockKind::BlockQuote,
        Tag::Table(_) => BlockKind::Table,
        Tag::HtmlBlock => BlockKind::HtmlBlock,
        Tag::FootnoteDefinition(_) => BlockKind::FootnoteDefinition,
        _ => BlockKind::Unknown,
    }
}

fn block_extent_for_tag(events: &[Event<'static>], tag: &Tag<'_>) -> usize {
    let end_tag: TagEnd = tag.clone().into();
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

fn event_slice_source(source: &str, _slice: &[Event<'static>]) -> String {
    // Until byte-range mapping exists, use full source for block metadata.
    source.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::ParseOptions;

    #[test]
    fn github_preview_enables_expected_options() {
        let opts = ParseOptions::for_profile(ParseProfile::GitHubPreview);
        let expected = ParseProfile::GitHubPreview.pulldown_options();
        assert_eq!(opts.pulldown, expected);
        use pulldown_cmark::Options;
        assert!(opts.pulldown.contains(Options::ENABLE_TABLES));
        assert!(opts.pulldown.contains(Options::ENABLE_TASKLISTS));
        assert!(opts.pulldown.contains(Options::ENABLE_STRIKETHROUGH));
        assert!(opts.pulldown.contains(Options::ENABLE_FOOTNOTES));
        assert!(opts.pulldown.contains(Options::ENABLE_GFM));
        assert!(opts.pulldown.contains(Options::ENABLE_MATH));
        assert!(opts.pulldown.contains(Options::ENABLE_WIKILINKS));
    }

    #[test]
    fn parses_paragraph_and_heading_blocks() {
        let source = "# Title\n\nHello **world**.";
        let blocks = parse_blocks(source, ParseProfile::GitHubPreview, &ParseOptions::default())
            .expect("parse");
        assert!(blocks.len() >= 2);
        assert!(blocks.iter().any(|b| b.kind == BlockKind::Heading));
        assert!(blocks.iter().any(|b| b.kind == BlockKind::Paragraph));
    }

    #[test]
    fn routes_raw_html_to_fragment() {
        let source = "<details><summary>x</summary></details>";
        let blocks = parse_blocks(source, ParseProfile::GitHubPreview, &ParseOptions::default())
            .expect("parse");
        assert!(blocks.iter().any(|b| matches!(
            b.content,
            BlockContent::Html(_)
        )));
    }
}
