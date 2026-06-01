use super::code_fence::{normalize_code_language, wrap_fenced_code};
use super::{MarkState, UpdateMsg};

/// A fenced or indented code block extracted from markdown source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    language: Option<String>,
    code: String,
    fence_char: char,
    open_fence_len: usize,
}

impl CodeBlock {
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Reconstructs a fenced markdown snippet suitable for `iced::widget::markdown::Content::parse`.
    #[must_use]
    pub fn fence_markdown(&self) -> String {
        let lang = self.language.as_deref().unwrap_or("");
        wrap_fenced_code(self.fence_char, self.open_fence_len, lang, &self.code)
    }
}

/// One renderable part of a split markdown document.
pub enum MarkSegment {
    Rich(MarkState),
    Code(CodeBlock),
}

impl std::fmt::Debug for MarkSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rich(_) => f.write_str("MarkSegment::Rich"),
            Self::Code(block) => f.debug_tuple("MarkSegment::Code").field(block).finish(),
        }
    }
}

/// Markdown source split into rich HTML segments and code blocks.
#[derive(Default)]
pub struct MarkDocument {
    source: String,
    segments: Vec<MarkSegment>,
}

impl Clone for MarkDocument {
    fn clone(&self) -> Self {
        Self::parse(&self.source)
    }
}

impl std::fmt::Debug for MarkDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkDocument")
            .field("source", &self.source)
            .field("segments", &self.segments.len())
            .finish()
    }
}

impl MarkDocument {
    /// Parses markdown, extracting fenced and indented code blocks.
    #[must_use]
    pub fn parse(source: &str) -> Self {
        Self::parse_with_rich_preprocessor(source, |markdown| markdown.to_string())
    }

    /// Parses markdown, running `preprocess_rich` on prose segments before pulldown conversion.
    #[must_use]
    pub fn parse_with_rich_preprocessor(
        source: &str,
        preprocess_rich: impl Fn(&str) -> String,
    ) -> Self {
        Self {
            source: source.to_string(),
            segments: split_markdown_segments(source, &preprocess_rich),
        }
    }

    #[must_use]
    pub fn into_segments(self) -> Vec<MarkSegment> {
        self.segments
    }

    #[must_use]
    pub fn segments(&self) -> &[MarkSegment] {
        &self.segments
    }

    pub fn update_rich_segment(&mut self, segment: usize, update: UpdateMsg) {
        if let Some(MarkSegment::Rich(state)) = self.segments.get_mut(segment) {
            state.update(update);
        }
    }
}

#[derive(Clone)]
struct FencedCodeBlock {
    open_line_start: usize,
    body_start: usize,
    body_end: usize,
    after_close: usize,
    fence_char: char,
    fence_len: usize,
    language: Option<String>,
}

#[derive(Clone)]
struct IndentedCodeBlock {
    start: usize,
    body_start: usize,
    body_end: usize,
    after_end: usize,
}

enum NextCodeBlock {
    Fenced(FencedCodeBlock),
    Indented(IndentedCodeBlock),
}

fn split_markdown_segments(
    source: &str,
    preprocess_rich: &impl Fn(&str) -> String,
) -> Vec<MarkSegment> {
    let mut segments = Vec::new();
    let mut pos = 0;

    while pos < source.len() {
        let Some(next) = find_next_code_block(source, pos) else {
            let tail = &source[pos..];
            if !tail.trim().is_empty() {
                segments.push(rich_segment(tail, preprocess_rich));
            }
            break;
        };

        let block_start = match &next {
            NextCodeBlock::Fenced(block) => block.open_line_start,
            NextCodeBlock::Indented(block) => block.start,
        };

        if block_start > pos {
            let markdown = &source[pos..block_start];
            if !markdown.trim().is_empty() {
                segments.push(rich_segment(markdown, preprocess_rich));
            }
        }

        match next {
            NextCodeBlock::Fenced(block) => {
                let body = &source[block.body_start..block.body_end];
                segments.push(code_segment(
                    block.fence_char,
                    block.fence_len,
                    block.language.as_deref(),
                    body,
                ));
                pos = block.after_close;
            }
            NextCodeBlock::Indented(block) => {
                let body = dedent_indented_code(&source[block.body_start..block.body_end]);
                segments.push(code_segment('`', 3, None, &body));
                pos = block.after_end;
            }
        }
    }

    segments
}

fn find_next_code_block(source: &str, from: usize) -> Option<NextCodeBlock> {
    let mut line_start = next_line_start(source, from);

    while line_start < source.len() {
        let current_line_end = line_end(source, line_start);
        let line = &source[line_start..current_line_end];
        let trimmed = line.trim_start();

        if line.len() - trimmed.len() <= 3
            && let Some((fence_char, fence_len, language)) = parse_fence_open(trimmed)
        {
            let body_start = advance_past_line(source, current_line_end);
            if let Some(body_end) = find_fence_close(source, body_start, fence_char, fence_len) {
                let close_line_end = line_end(source, body_end);
                let after_close = advance_past_line(source, close_line_end);

                return Some(NextCodeBlock::Fenced(FencedCodeBlock {
                    open_line_start: line_start,
                    body_start,
                    body_end,
                    after_close,
                    fence_char,
                    fence_len,
                    language,
                }));
            }
        }

        if is_indented_code_line(line) && is_indented_block_start(source, line_start) {
            return Some(NextCodeBlock::Indented(collect_indented_code_block(
                source,
                line_start,
                current_line_end,
            )));
        }

        line_start = advance_past_line(source, current_line_end);
    }

    None
}

fn rich_segment(markdown: &str, preprocess_rich: &impl Fn(&str) -> String) -> MarkSegment {
    let processed = preprocess_rich(markdown);
    MarkSegment::Rich(MarkState::with_html_and_markdown(&processed))
}

fn code_segment(
    fence_char: char,
    open_fence_len: usize,
    language: Option<&str>,
    body: &str,
) -> MarkSegment {
    let language = normalize_code_language(language);
    let body = body.strip_suffix('\n').unwrap_or(body);
    MarkSegment::Code(CodeBlock {
        language,
        code: body.to_string(),
        fence_char,
        open_fence_len,
    })
}

fn parse_fence_open(line: &str) -> Option<(char, usize, Option<String>)> {
    let fence_char = line.chars().next()?;
    if fence_char != '`' && fence_char != '~' {
        return None;
    }

    let mut fence_len = 0;
    for ch in line.chars() {
        if ch == fence_char {
            fence_len += 1;
        } else {
            break;
        }
    }

    if fence_len < 3 {
        return None;
    }

    let language = line[fence_len..].trim();
    let language = (!language.is_empty()).then(|| language.to_string());
    Some((fence_char, fence_len, language))
}

fn find_fence_close(
    source: &str,
    from: usize,
    fence_char: char,
    fence_len: usize,
) -> Option<usize> {
    let mut search_from = from;

    while search_from < source.len() {
        let line_start = search_from;
        let line_end = source[line_start..]
            .find('\n')
            .map(|offset| line_start + offset)
            .unwrap_or(source.len());
        let line = &source[line_start..line_end];
        let trimmed = line.trim_start();

        if line.len() - trimmed.len() <= 3 && is_fence_close(trimmed, fence_char, fence_len) {
            return Some(line_start);
        }

        search_from = if line_end < source.len() {
            line_end + 1
        } else {
            source.len()
        };
    }

    None
}

fn is_fence_close(line: &str, fence_char: char, fence_len: usize) -> bool {
    let mut count = 0;
    for ch in line.chars() {
        if ch == fence_char {
            count += 1;
        } else {
            break;
        }
    }

    count >= fence_len && line[count..].trim().is_empty()
}

fn collect_indented_code_block(
    source: &str,
    body_start: usize,
    first_line_end: usize,
) -> IndentedCodeBlock {
    let mut body_end = first_line_end;
    let mut scan = advance_past_line(source, first_line_end);

    while scan < source.len() {
        let next_line_end = line_end(source, scan);
        let next_line = &source[scan..next_line_end];

        if next_line.trim().is_empty() || is_indented_code_line(next_line) {
            body_end = next_line_end;
            scan = advance_past_line(source, next_line_end);
            continue;
        }
        break;
    }

    IndentedCodeBlock {
        start: body_start,
        body_start,
        body_end,
        after_end: advance_past_line(source, body_end),
    }
}

fn is_indented_code_line(line: &str) -> bool {
    if line.trim().is_empty() {
        return false;
    }

    line.starts_with('\t') || line.chars().take_while(|ch| *ch == ' ').count() >= 4
}

fn is_indented_block_start(source: &str, line_start: usize) -> bool {
    if line_start == 0 {
        return true;
    }

    source[..line_start].ends_with("\n\n")
}

fn next_line_start(source: &str, from: usize) -> usize {
    if from == 0
        || source
            .as_bytes()
            .get(from.saturating_sub(1))
            .is_some_and(|b| *b == b'\n')
    {
        return from;
    }
    source[from..]
        .find('\n')
        .map(|offset| from + offset + 1)
        .unwrap_or(source.len())
}

fn line_end(source: &str, start: usize) -> usize {
    source[start..]
        .find('\n')
        .map(|offset| start + offset)
        .unwrap_or(source.len())
}

fn advance_past_line(source: &str, end: usize) -> usize {
    end + usize::from(end < source.len())
}

fn dedent_indented_code(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else if let Some(rest) = line.strip_prefix('\t') {
                rest.to_string()
            } else {
                line.strip_prefix("    ").unwrap_or(line).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_multiline_indented_code_as_single_block() {
        let source = "    fn main() {\n        println!(\"indented code block\");\n    }";
        let document = MarkDocument::parse(source);
        let segments = document.segments();
        assert_eq!(segments.len(), 1);
        let MarkSegment::Code(block) = &segments[0] else {
            panic!("expected a single code block segment");
        };
        assert!(block.code().contains("fn main()"));
        assert!(block.code().contains("println!"));
        assert!(block.code().contains('}'));
    }

    #[test]
    fn splits_indented_code_from_surrounding_markdown() {
        let source = "Intro\n\n    fn main() {}\n\nOutro";
        let document = MarkDocument::parse(source);
        let segments = document.segments();
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], MarkSegment::Rich(_)));
        assert!(matches!(&segments[1], MarkSegment::Code(_)));
        assert!(matches!(&segments[2], MarkSegment::Rich(_)));
    }

    #[test]
    fn four_tick_fence_preserves_inner_three_tick_fence() {
        let source = "````markdown\n```rust\nfn main() {}\n```\n````";
        let document = MarkDocument::parse(source);
        let segments = document.segments();
        assert_eq!(segments.len(), 1);
        let MarkSegment::Code(block) = &segments[0] else {
            panic!("expected a single code block segment");
        };
        assert_eq!(block.language(), Some("markdown"));
        assert!(block.code().contains("```rust"));
        assert!(block.code().contains("fn main() {}"));
        assert!(block.fence_markdown().starts_with("````markdown"));
    }

    #[test]
    fn splits_fenced_code_from_surrounding_markdown() {
        let source = "Intro\n\n```rust\nfn main() {}\n```\n\nOutro";
        let document = MarkDocument::parse(source);
        let segments = document.segments();
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], MarkSegment::Rich(_)));
        assert!(matches!(&segments[1], MarkSegment::Code(_)));
        assert!(matches!(&segments[2], MarkSegment::Rich(_)));
    }

    #[test]
    fn splits_consecutive_fenced_blocks() {
        let source = "```powershell\na\n```\n\n```powershell\nb\n```";
        let document = MarkDocument::parse(source);
        let segments = document.segments();
        assert_eq!(segments.len(), 2);
        assert!(matches!(&segments[0], MarkSegment::Code(_)));
        assert!(matches!(&segments[1], MarkSegment::Code(_)));
    }
}
