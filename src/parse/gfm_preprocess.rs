//! GFM extensions not provided by pulldown-cmark (extended autolinks, spec §6.9).

/// Rewrite `www.` / bare-email autolinks into bracketed `http` autolinks pulldown understands.
#[must_use]
pub fn apply_gfm_extended_autolinks(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + 32);
    for line in source.split_inclusive('\n') {
        out.push_str(&rewrite_line(line));
    }
    out
}

fn rewrite_line(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len() + 16);
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some((len, replacement)) = try_www_autolink(line, i) {
            out.push_str(&replacement);
            i += len;
            continue;
        }
        if let Some(scan) = scan_bare_email_candidate(line, i) {
            match scan {
                EmailScan::Rewritten {
                    consumed,
                    replacement,
                } => {
                    out.push_str(&replacement);
                    i += consumed;
                    continue;
                }
                EmailScan::Original { consumed } if consumed > 0 => {
                    out.push_str(&line[i..i + consumed]);
                    i += consumed;
                    continue;
                }
                EmailScan::Original { .. } => {}
            }
        }
        let ch = line[i..].chars().next().expect("utf8");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn autolink_may_start(line: &str, start: usize) -> bool {
    if start == 0 {
        return true;
    }
    let prev = line.as_bytes()[start - 1];
    prev.is_ascii_whitespace() || matches!(prev, b'*' | b'_' | b'~' | b'(')
}

enum EmailScan {
    Rewritten {
        consumed: usize,
        replacement: String,
    },
    Original {
        consumed: usize,
    },
}

fn try_www_autolink(line: &str, start: usize) -> Option<(usize, String)> {
    if !autolink_may_start(line, start) {
        return None;
    }
    let rest = &line[start..];
    if !rest.starts_with("www.") {
        return None;
    }
    let domain_end = www_domain_end(rest)?;
    let path_end = www_path_end(&rest[..domain_end], &rest[domain_end..]);
    let full_end = domain_end + path_end;
    let link_text = &rest[..full_end];
    let trimmed = trim_trailing_punctuation(link_text);
    if trimmed.is_empty() {
        return None;
    }
    let suffix = &link_text[trimmed.len()..];
    let href = format!("http://{trimmed}");
    let replacement = format!("<{href}>{suffix}");
    Some((start + full_end, replacement))
}

fn www_domain_end(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if bytes.len() < 5 || !rest.starts_with("www.") {
        return None;
    }
    let mut i = 4usize;
    let mut period_count = 0usize;
    let mut since_period = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
            since_period += 1;
            i += 1;
            continue;
        }
        if c == b'.' {
            if i == 4 || since_period == 0 {
                return None;
            }
            period_count += 1;
            since_period = 0;
            i += 1;
            continue;
        }
        break;
    }
    if period_count == 0 || since_period == 0 {
        return None;
    }
    let last_two = rest[4..i].split('.').rev().take(2).collect::<Vec<_>>();
    if last_two.iter().any(|seg| seg.contains('_')) {
        return None;
    }
    Some(i)
}

fn www_path_end(domain: &str, tail: &str) -> usize {
    let mut len = 0usize;
    for ch in tail.chars() {
        if ch.is_whitespace() || ch == '<' {
            break;
        }
        len += ch.len_utf8();
    }
    let _ = domain;
    len
}

fn trim_trailing_punctuation(link: &str) -> &str {
    let mut end = link.len();
    while end > 0 {
        let ch = link[..end].chars().last().unwrap();
        if "?!.:,*_~".contains(ch) {
            end -= ch.len_utf8();
        } else {
            break;
        }
    }
    if link.as_bytes().get(end - 1) == Some(&b')') {
        let open = link[..end].chars().filter(|&c| c == '(').count();
        let close = link[..end].chars().filter(|&c| c == ')').count();
        if close > open {
            while end > 0 && link.as_bytes()[end - 1] == b')' {
                end -= 1;
            }
        }
    }
    &link[..end]
}

fn scan_bare_email_candidate(line: &str, start: usize) -> Option<EmailScan> {
    if !autolink_may_start(line, start) {
        return None;
    }
    let rest = &line[start..];
    if !rest.chars().next().is_some_and(is_email_local_char) {
        return None;
    }

    let mut at = None;
    for (offset, ch) in rest.char_indices() {
        if ch == '@' {
            at = Some(offset);
            break;
        }
        if ch.is_whitespace() || ch == '<' {
            return Some(EmailScan::Original { consumed: offset });
        }
        if !is_email_local_char(ch) {
            return Some(EmailScan::Original { consumed: offset });
        }
    }

    let Some(at) = at else {
        return Some(EmailScan::Original {
            consumed: rest.len(),
        });
    };

    let domain_len = email_domain_end(&rest[at + 1..])?;
    let email = &rest[..at + 1 + domain_len];
    let replacement = format!("<mailto:{email}>");
    Some(EmailScan::Rewritten {
        consumed: email.len(),
        replacement,
    })
}

fn is_email_local_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(ch)
}

fn email_domain_end(domain: &str) -> Option<usize> {
    let mut i = 0usize;
    let mut label_start = 0usize;
    let mut labels = 0usize;
    while i < domain.len() {
        let b = domain.as_bytes()[i];
        if b.is_ascii_alphanumeric() || b == b'-' {
            i += 1;
            continue;
        }
        if b == b'.' {
            if i == label_start {
                return None;
            }
            labels += 1;
            label_start = i + 1;
            i += 1;
            continue;
        }
        break;
    }
    if labels == 0 || i == label_start {
        return None;
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn www_line_becomes_bracketed_http() {
        let out = apply_gfm_extended_autolinks("www.commonmark.org\n");
        assert!(out.contains("<http://www.commonmark.org>"));
    }

    #[test]
    fn www_in_sentence() {
        let out = apply_gfm_extended_autolinks("Visit www.commonmark.org/help for more.\n");
        assert!(out.contains("<http://www.commonmark.org/help>"));
    }

    #[test]
    fn bare_email_becomes_mailto_autolink() {
        let out = apply_gfm_extended_autolinks("mail me at user.name+tag@example.com\n");
        assert!(out.contains("<mailto:user.name+tag@example.com>"));
    }

    #[test]
    fn invalid_local_prefix_does_not_block_later_email_start() {
        let out = apply_gfm_extended_autolinks("a(user@example.com)\n");
        assert!(out.contains("a(<mailto:user@example.com>)"));
    }
}
