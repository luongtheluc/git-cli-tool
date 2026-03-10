//! Unicode-aware text utilities for terminal rendering
//! 
//! Provides grapheme-safe width calculations and truncation for cross-platform
//! terminal display. Handles CJK (2-column), combining marks (0-column), and emoji.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Returns the display width of text in terminal columns.
/// 
/// - CJK characters count as 2 columns
/// - Combining marks count as 0 columns
/// - Standard ASCII counts as 1 column each
/// - Emoji width is platform-dependent (generally 1-2 columns)
///
/// # Example
/// ```
/// assert_eq!(display_width("Hello"), 5);        // ASCII
/// assert_eq!(display_width("你好"), 4);         // CJK (2 + 2)
/// ```
pub fn display_width(text: &str) -> usize {
    text.width()
}

/// Truncates text to fit within a maximum display width, respecting grapheme boundaries.
///
/// Returns a byte slice that ends at or before `max_width` columns, ensuring no
/// grapheme clusters are broken and no multi-byte characters are split.
///
/// # Example
/// ```
/// let result = truncate_to_width("Hello世界", 7);
/// assert_eq!(result, "Hello世");  // Fits in 7 columns (5 + 2)
/// ```
pub fn truncate_to_width(text: &str, max_width: usize) -> &str {
    if max_width == 0 {
        return "";
    }

    let mut current_width = 0;
    let mut last_valid_end = 0;

    for grapheme in text.graphemes(true) {
        let grapheme_width = grapheme.width();
        let next_width = current_width + grapheme_width;

        if next_width > max_width {
            // Grapheme doesn't fit; return text up to last valid position
            return &text[..last_valid_end];
        }

        last_valid_end += grapheme.len();
        current_width = next_width;
    }

    text
}

/// Pads text with spaces to reach the target display width.
///
/// If text is already at or exceeds the target width, returns the original text unchanged.
///
/// # Example
/// ```
/// let result = pad_to_width("你好", 10);
/// assert_eq!(display_width(&result), 10);  // Padded to 10 columns
/// ```
pub fn pad_to_width(text: &str, target_width: usize) -> String {
    let current_width = display_width(text);
    if current_width >= target_width {
        return text.to_string();
    }
    let padding = target_width - current_width;
    format!("{}{}", text, " ".repeat(padding))
}

/// Counts the number of grapheme clusters in the text.
///
/// Each grapheme cluster counts as 1, regardless of width:
/// - "é" (single codepoint) = 1 grapheme
/// - "e\u{0308}" (combining) = 1 grapheme
/// - "👨‍👩‍👧‍👦" (ZWJ sequence) = 1 grapheme unit
///
/// # Example
/// ```
/// assert_eq!(grapheme_count("hello"), 5);
/// assert_eq!(grapheme_count("你好"), 2);      // 2 CJK characters
/// assert_eq!(grapheme_count("e\u{0308}"), 1); // e with combining diaeresis
/// ```
#[allow(dead_code)]
pub fn grapheme_count(text: &str) -> usize {
    text.graphemes(true).count()
}

/// Checks if text fits within a given display width.
///
/// Returns `true` if the text's display width is <= `available_width`.
///
/// # Example
/// ```
/// assert!(line_fits_width("Hello", 10));
/// assert!(!line_fits_width("你好世界", 5));  // Needs 8 columns
/// ```
pub fn line_fits_width(text: &str, available_width: usize) -> bool {
    display_width(text) <= available_width
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_width_ascii() {
        assert_eq!(display_width("Hello"), 5);
        assert_eq!(display_width(""), 0);
        assert_eq!(display_width("12345"), 5);
    }

    #[test]
    fn test_display_width_cjk() {
        // Chinese characters (east asian width: wide)
        assert_eq!(display_width("你"), 2);
        assert_eq!(display_width("你好"), 4);
        // Japanese hiragana (east asian width: wide)
        assert_eq!(display_width("プ"), 2);
        assert_eq!(display_width("プロジェクト"), 12);
    }

    #[test]
    fn test_display_width_combining() {
        // Combining diaeresis (e + combining mark)
        let combining_e = "e\u{0308}";
        assert_eq!(display_width(combining_e), 1); // e=1, combining=0
    }

    #[test]
    fn test_display_width_emoji() {
        // Basic emoji (usually 1-2 columns depending on terminal)
        // Thumbs up emoji
        let emoji_width = "👍".width();
        assert_eq!(display_width("👍"), emoji_width);
    }

    #[test]
    fn test_truncate_to_width_ascii() {
        assert_eq!(truncate_to_width("Hello", 5), "Hello");
        assert_eq!(truncate_to_width("Hello", 3), "Hel");
        assert_eq!(truncate_to_width("Hello", 10), "Hello");
        assert_eq!(truncate_to_width("Hello", 0), "");
    }

    #[test]
    fn test_truncate_to_width_cjk() {
        // 你 = 2 columns, 好 = 2 columns
        let text = "你好世";
        assert_eq!(truncate_to_width(text, 6), "你好世"); // 6 columns, all fit
        assert_eq!(truncate_to_width(text, 4), "你好"); // 4 columns
        assert_eq!(truncate_to_width(text, 3), "你"); // 3 columns (3 < 4, so only 你)
        assert_eq!(truncate_to_width(text, 2), "你"); // Exactly 2
    }

    #[test]
    fn test_truncate_to_width_mixed() {
        let text = "Hello世界"; // H-e-l-l-o (5) + 世 (2) + 界 (2) = 9 columns
        assert_eq!(truncate_to_width(text, 9), "Hello世界");
        assert_eq!(truncate_to_width(text, 7), "Hello世"); // 5 + 2 = 7
        assert_eq!(truncate_to_width(text, 5), "Hello"); // Just ASCII
    }

    #[test]
    fn test_truncate_to_width_combining() {
        let text = "café"; // c + a + f + é (combining or precomposed)
        let result = truncate_to_width(text, 4);
        assert_eq!(display_width(result), 4);
        assert!(result.len() > 0);
    }

    #[test]
    fn test_pad_to_width_ascii() {
        let result = pad_to_width("Hi", 5);
        assert_eq!(result, "Hi   ");
        assert_eq!(display_width(&result), 5);
    }

    #[test]
    fn test_pad_to_width_cjk() {
        // 你好 = 4 columns, pad to 8
        let result = pad_to_width("你好", 8);
        assert_eq!(display_width(&result), 8);
        assert!(result.starts_with("你好"));
        assert_eq!(result.len(), "你好".len() + 4); // 4 spaces added
    }

    #[test]
    fn test_pad_to_width_already_wide() {
        let result = pad_to_width("Hello", 3);
        assert_eq!(result, "Hello"); // No padding when already too wide
    }

    #[test]
    fn test_grapheme_count_ascii() {
        assert_eq!(grapheme_count(""), 0);
        assert_eq!(grapheme_count("Hello"), 5);
        assert_eq!(grapheme_count("a"), 1);
    }

    #[test]
    fn test_grapheme_count_cjk() {
        assert_eq!(grapheme_count("你"), 1);
        assert_eq!(grapheme_count("你好"), 2);
        assert_eq!(grapheme_count("プロジェクト"), 6);
    }

    #[test]
    fn test_grapheme_count_combining() {
        // e + combining diaeresis = 1 grapheme
        let combining_e = "e\u{0308}";
        assert_eq!(grapheme_count(combining_e), 1);
        // But two separate: e and e\u{0308} = 2 graphemes
        let two_graphemes = format!("e{}", combining_e);
        assert_eq!(grapheme_count(&two_graphemes), 2);
    }

    #[test]
    fn test_grapheme_count_emoji() {
        assert_eq!(grapheme_count("👍"), 1);
        assert_eq!(grapheme_count("👍👎"), 2);
    }

    #[test]
    fn test_line_fits_width() {
        assert!(line_fits_width("Hello", 10));
        assert!(line_fits_width("Hello", 5));
        assert!(!line_fits_width("Hello", 4));
        assert!(line_fits_width("", 1));
    }

    #[test]
    fn test_line_fits_width_cjk() {
        // 你好 = 4 columns
        assert!(line_fits_width("你好", 4));
        assert!(line_fits_width("你好", 5));
        assert!(!line_fits_width("你好", 3));
    }

    #[test]
    fn test_integration_truncate_and_pad() {
        let text = "Hello世界テスト";
        let truncated = truncate_to_width(text, 10);
        let padded = pad_to_width(truncated, 10);
        assert_eq!(display_width(&padded), 10);
    }
}
