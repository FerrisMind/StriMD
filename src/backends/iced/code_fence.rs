//! Fenced code helpers: fence markdown reconstruction and iced syntax highlighting.

/// Normalizes a language tag for iced/syntect (shared by block cache and segment split).
pub(crate) fn normalize_code_language(language: Option<&str>) -> Option<String> {
    let language = language.unwrap_or("").trim();
    if language.is_empty() {
        return None;
    }

    Some(match language.to_ascii_lowercase().as_str() {
        "powershell" | "ps1" | "pwsh" => "powershell".to_string(),
        "shell" | "sh" | "bash" | "zsh" => "bash".to_string(),
        "yml" | "yaml" => "yaml".to_string(),
        other => other.to_string(),
    })
}

/// Builds a fenced markdown snippet for [`iced::widget::markdown::Content::parse`].
///
/// Uses a standard triple-backtick fence (pulldown block-cache path).
#[must_use]
pub fn fence_markdown_for_codeblock(language: Option<&str>, body: &str) -> String {
    let language = normalize_code_language(language);
    let lang = language.as_deref().unwrap_or("");
    let body = body.strip_suffix('\n').unwrap_or(body);
    wrap_fenced_code('`', 3, lang, body)
}

/// Reconstructs a fence around `body`, widening the closing fence if the body contains runs of `` ` ``.
#[must_use]
pub fn wrap_fenced_code(
    fence_char: char,
    open_fence_len: usize,
    language: &str,
    body: &str,
) -> String {
    let fence_len = wrapping_fence_len(open_fence_len, body);
    let fence = fence_char.to_string().repeat(fence_len);
    if language.is_empty() {
        format!("{fence}\n{body}\n{fence}")
    } else {
        format!("{fence}{language}\n{body}\n{fence}")
    }
}

/// Parses a code fence into iced markdown items (syntax highlighting when `highlighter` is enabled).
#[must_use]
pub fn iced_markdown_items_for_codeblock(
    language: Option<&str>,
    code: &str,
) -> Vec<iced::widget::markdown::Item> {
    use iced::widget::markdown;
    let fence = fence_markdown_for_codeblock(language, code);
    markdown::Content::parse(&fence).items().to_vec()
}

fn wrapping_fence_len(open_fence_len: usize, body: &str) -> usize {
    let mut fence_len = open_fence_len.max(3);
    for line in body.lines() {
        let trimmed = line.trim_start();
        let run = trimmed.chars().take_while(|ch| *ch == '`').count();
        if run >= 3 {
            fence_len = fence_len.max(run + 1);
        }
    }
    fence_len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_markdown_uses_normalized_language() {
        let fence = fence_markdown_for_codeblock(Some("rs"), "fn main() {}");
        assert!(fence.starts_with("```rs\n"));
        assert!(fence.contains("fn main() {}"));
    }

    #[test]
    fn wrap_fenced_code_widens_fence_for_inner_backticks() {
        let body = "```rust\nfn main() {}\n```";
        let wrapped = wrap_fenced_code('`', 4, "markdown", body);
        assert!(wrapped.starts_with("````markdown"));
    }

    #[test]
    fn iced_markdown_items_include_code_block() {
        let items = iced_markdown_items_for_codeblock(Some("rust"), "fn main() {}\n");
        assert!(items
            .iter()
            .any(|item| matches!(item, iced::widget::markdown::Item::CodeBlock { .. })));
    }
}
