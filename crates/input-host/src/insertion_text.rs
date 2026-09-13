#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedInsertionText {
    pub text: String,
    pub multiline: bool,
    pub format_normalized: bool,
    pub line_count: usize,
}

pub(crate) fn prepare_plain_text_for_insertion(raw: &str) -> PreparedInsertionText {
    let canonical = canonicalize_newlines(raw);
    let text = canonical
        .split('\n')
        .map(normalize_presentation_line)
        .collect::<Vec<_>>()
        .join("\n");

    PreparedInsertionText {
        multiline: text.contains('\n'),
        format_normalized: text != raw,
        line_count: text.split('\n').count(),
        text,
    }
}

fn canonicalize_newlines(text: &str) -> String {
    let mut canonical = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                canonical.push('\n');
            }
            _ => canonical.push(ch),
        }
    }
    canonical
}

fn normalize_presentation_line(line: &str) -> String {
    let indentation_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indentation, content) = line.split_at(indentation_len);
    let content = match content.as_bytes() {
        [marker @ (b'*' | b'-' | b'+'), whitespace, ..]
            if *whitespace == b' ' || *whitespace == b'\t' =>
        {
            let _ = marker;
            format!("• {}", content[1..].trim_start_matches([' ', '\t']))
        }
        _ => content.to_string(),
    };

    format!("{indentation}{}", strip_clear_bold_markers(&content))
}

fn strip_clear_bold_markers(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;

    while cursor < text.len() {
        let Some((open, delimiter)) = next_bold_open(text, cursor) else {
            output.push_str(&text[cursor..]);
            break;
        };
        output.push_str(&text[cursor..open]);

        let content_start = open + delimiter.len();
        let Some(relative_close) = text[content_start..].find(delimiter) else {
            output.push_str(&text[open..]);
            break;
        };
        let close = content_start + relative_close;
        let after_close = close + delimiter.len();
        let content = &text[content_start..close];

        if is_clear_bold_span(text, open, after_close, content) {
            output.push_str(content);
            cursor = after_close;
        } else {
            output.push_str(delimiter);
            cursor = content_start;
        }
    }

    output
}

fn next_bold_open(text: &str, from: usize) -> Option<(usize, &'static str)> {
    let stars = text[from..].find("**").map(|index| (from + index, "**"));
    let underscores = text[from..].find("__").map(|index| (from + index, "__"));
    match (stars, underscores) {
        (Some(stars), Some(underscores)) => Some(if stars.0 <= underscores.0 {
            stars
        } else {
            underscores
        }),
        (some @ Some(_), None) | (None, some @ Some(_)) => some,
        (None, None) => None,
    }
}

fn is_clear_bold_span(text: &str, open: usize, after_close: usize, content: &str) -> bool {
    if content.is_empty()
        || content.starts_with(char::is_whitespace)
        || content.ends_with(char::is_whitespace)
    {
        return false;
    }

    if text[..open].bytes().filter(|byte| *byte == b'`').count() % 2 == 1 {
        return false;
    }

    let token_start = text[..open]
        .rfind(char::is_whitespace)
        .map_or(0, |index| index + 1);
    let token = &text[token_start..];
    if token.starts_with("http://") || token.starts_with("https://") {
        return false;
    }

    let before_is_word = text[..open]
        .chars()
        .next_back()
        .is_some_and(|ch| ch.is_alphanumeric() || ch == '_');
    let after_is_word = text[after_close..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_alphanumeric() || ch == '_');
    !before_is_word && !after_is_word
}

#[cfg(test)]
mod tests {
    use super::prepare_plain_text_for_insertion;

    #[test]
    fn single_line_plain_text_is_unchanged() {
        let prepared = prepare_plain_text_for_insertion("ordinary dictation");
        assert_eq!(prepared.text, "ordinary dictation");
        assert!(!prepared.multiline);
        assert!(!prepared.format_normalized);
        assert_eq!(prepared.line_count, 1);
    }

    #[test]
    fn canonicalizes_lf_crlf_and_lone_cr_without_losing_blank_lines() {
        for raw in ["one\n\nthree", "one\r\n\r\nthree", "one\r\rthree"] {
            let prepared = prepare_plain_text_for_insertion(raw);
            assert_eq!(prepared.text, "one\n\nthree");
            assert!(prepared.multiline);
            assert_eq!(prepared.line_count, 3);
        }
    }

    #[test]
    fn normalizes_supported_markdown_presentation_syntax() {
        for (raw, expected) in [
            ("* **Team:** text", "• Team: text"),
            ("- item", "• item"),
            ("+ item", "• item"),
            ("  *   __Team:__ text", "  • Team: text"),
        ] {
            assert_eq!(prepare_plain_text_for_insertion(raw).text, expected);
        }
    }

    #[test]
    fn preserves_non_presentation_symbols_and_identifiers() {
        for text in [
            "2 * 3",
            "file_name",
            "name@example.com",
            "https://example.com/a__b?q=some_value",
            "code_like__value__suffix",
            "`**not presentation Markdown**`",
        ] {
            assert_eq!(prepare_plain_text_for_insertion(text).text, text);
        }
    }

    #[test]
    fn reported_email_regression_is_normalized_without_collapsing_paragraphs() {
        let raw = "Subject: Round 2 Chatbot Testing & Doc Updates\r\n\r\nHi team,\r\n\r\nJust a heads-up that we’ll be kicking off the second round of chatbot testing next week.\r\n\r\nA couple of quick requests:\r\n*   **AP and Tax teams:** Please submit your updated documents by next Thursday (not Wednesday).\r\n*   **Boat version:** No further changes are needed here.\r\n\r\nThanks!";
        let expected = "Subject: Round 2 Chatbot Testing & Doc Updates\n\nHi team,\n\nJust a heads-up that we’ll be kicking off the second round of chatbot testing next week.\n\nA couple of quick requests:\n• AP and Tax teams: Please submit your updated documents by next Thursday (not Wednesday).\n• Boat version: No further changes are needed here.\n\nThanks!";
        let prepared = prepare_plain_text_for_insertion(raw);

        assert_eq!(prepared.text, expected);
        assert!(prepared.multiline);
        assert!(prepared.format_normalized);
        assert_eq!(prepared.line_count, 11);
    }
}
