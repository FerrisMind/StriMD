pub fn markdown_to_html(input: &str) -> String {
    comrak::markdown_to_html(input, &options())
}

pub fn options() -> comrak::Options<'static> {
    comrak::Options {
        extension: comrak::options::Extension {
            strikethrough: true,
            cjk_friendly_emphasis: true,
            tasklist: true,
            superscript: true,
            subscript: true,
            underline: true,
            table: true,
            autolink: true,
            ..Default::default()
        },
        parse: comrak::options::Parse::default(),
        render: comrak::options::Render {
            r#unsafe: true,
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_indented_code_as_pre_block() {
        let html = markdown_to_html("    fn main() {}\n");
        assert!(html.contains("<pre>"));
        assert!(html.contains("<code>"));
    }

    #[test]
    fn renders_inline_code_with_dots() {
        let html = markdown_to_html("Requires Rust `1.93` or newer.");
        assert!(html.contains("<code>1.93</code>"));
    }

    #[test]
    fn does_not_treat_dotted_number_as_ordered_list() {
        let html = markdown_to_html("1.93\n");
        assert!(!html.contains("<ol>"));
        assert!(html.contains("1.93"));
    }
}
