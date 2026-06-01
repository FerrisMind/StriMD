//! GFM tagfilter (spec §6.11): escape disallowed raw HTML tag openers.

const GFM_FILTERED_TAGS: &[&str] = &[
    "title",
    "textarea",
    "style",
    "xmp",
    "iframe",
    "noembed",
    "noframes",
    "script",
    "plaintext",
];

/// Replace the leading `<` of disallowed raw HTML tags with `&lt;` (GFM tagfilter).
#[must_use]
pub fn apply_gfm_tagfilter(html: &str) -> String {
    let bytes = html.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() + 16);
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<'
            && let Some(tag_len) = matched_filtered_tag(bytes, i)
        {
            out.extend_from_slice(b"&lt;");
            out.extend_from_slice(&bytes[i + 1..i + tag_len]);
            i += tag_len;
            continue;
        }
        let len = utf8_char_len(bytes[i]);
        out.extend_from_slice(&bytes[i..i + len]);
        i += len;
    }
    String::from_utf8(out).expect("tagfilter preserves utf-8")
}

fn matched_filtered_tag(bytes: &[u8], open: usize) -> Option<usize> {
    debug_assert_eq!(bytes.get(open), Some(&b'<'));
    let rest = &bytes[open + 1..];
    for tag in GFM_FILTERED_TAGS {
        let tag_b = tag.as_bytes();
        if rest.len() < tag_b.len() {
            continue;
        }
        if !rest[..tag_b.len()].eq_ignore_ascii_case(tag_b) {
            continue;
        }
        if tag_name_terminator(rest.get(tag_b.len()).copied()) {
            return Some(1 + tag_b.len());
        }
    }
    None
}

fn tag_name_terminator(next: Option<u8>) -> bool {
    matches!(
        next,
        None | Some(b'>') | Some(b'/') | Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')
    )
}

fn utf8_char_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_example_657_tagfilter() {
        let input = "<strong> <title> <style> <em>\n\n<blockquote>\n  <xmp> is disallowed.  <XMP> is also disallowed.\n</blockquote>\n";
        let out = apply_gfm_tagfilter(input);
        assert!(out.contains("&lt;title>"));
        assert!(out.contains("&lt;style>"));
        assert!(out.contains("&lt;xmp>"));
        assert!(out.contains("&lt;XMP>"));
        assert!(out.contains("<strong>"));
        assert!(out.contains("<em>"));
    }

    #[test]
    fn script_opening_is_filtered() {
        let out = apply_gfm_tagfilter("<script>alert(1)</script>");
        assert!(out.starts_with("&lt;script>"));
    }

    #[test]
    fn preserves_utf8_ruby_markup() {
        let input = "<ruby>東<rt>とう</rt>京<rt>きょう</rt></ruby>";
        let out = apply_gfm_tagfilter(input);
        assert_eq!(out, input);
    }
}
