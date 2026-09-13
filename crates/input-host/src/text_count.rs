pub(crate) fn count_words_entered(text: &str) -> u64 {
    shared_protocol::count_text_units(text)
}

#[cfg(test)]
mod tests {
    use super::count_words_entered;

    #[test]
    fn counts_chinese_han_characters() {
        assert_eq!(count_words_entered("你好世界"), 4);
    }

    #[test]
    fn counts_english_words() {
        assert_eq!(count_words_entered("hello world"), 2);
    }

    #[test]
    fn counts_mixed_chinese_and_english() {
        assert_eq!(count_words_entered("你好 hello world"), 4);
        assert_eq!(count_words_entered("你好，world!"), 3);
    }

    #[test]
    fn ignores_punctuation_and_whitespace() {
        assert_eq!(count_words_entered("  ...，！？\n\t"), 0);
    }

    #[test]
    fn counts_each_contiguous_number_run() {
        assert_eq!(count_words_entered("123 45,678"), 3);
    }

    #[test]
    fn empty_text_has_no_count() {
        assert_eq!(count_words_entered(""), 0);
    }
}
