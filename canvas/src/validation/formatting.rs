/* canvas/src/validation/formatting.rs
   Add new formatting module with CustomFormatter, PositionMapper, DefaultPositionMapper, and FormattingResult
*/
use std::sync::Arc;

/// Bidirectional mapping between raw input positions and formatted display positions.
///
/// The library uses this to keep cursor/selection behavior intuitive when the UI
/// shows a formatted transformation (e.g., "01001" -> "010 01") while the editor
/// still stores raw text.
pub trait PositionMapper: Send + Sync {
    /// Map a raw cursor position to a formatted cursor position.
    ///
    /// raw_pos is an index into the raw text (0..=raw.len() in char positions).
    /// Implementations should return a position within 0..=formatted.len() (in char positions).
    fn raw_to_formatted(&self, raw: &str, formatted: &str, raw_pos: usize) -> usize;

    /// Map a formatted cursor position to a raw cursor position.
    ///
    /// formatted_pos is an index into the formatted text (0..=formatted.len()).
    /// Implementations should return a position within 0..=raw.len() (in char positions).
    fn formatted_to_raw(&self, raw: &str, formatted: &str, formatted_pos: usize) -> usize;
}

/// A reasonable default mapper that works for "insert separators" style formatting,
/// such as grouping digits or adding dashes/spaces.
///
/// Heuristic:
/// - Treat letters and digits (is_alphanumeric) in the formatted string as user-entered characters
///   corresponding to raw characters, in order.
/// - Treat any non-alphanumeric characters as purely visual separators.
/// - Raw positions are mapped by counting alphanumeric characters in the formatted string.
/// - If the formatted contains fewer alphanumeric characters than the raw (shouldn't happen
///   for plain grouping), we cap at the end of the formatted string.
#[derive(Clone, Default)]
pub struct DefaultPositionMapper;

impl PositionMapper for DefaultPositionMapper {
    fn raw_to_formatted(&self, raw: &str, formatted: &str, raw_pos: usize) -> usize {
        // Convert to char indices for correctness in presence of UTF-8
        let raw_len = raw.chars().count();
        let clamped_raw_pos = raw_pos.min(raw_len);

        // Count alphanumerics in formatted, find the index where we've seen `clamped_raw_pos` of them.
        let mut seen_user_chars = 0usize;
        for (idx, ch) in formatted.char_indices() {
            if ch.is_alphanumeric() {
                if seen_user_chars == clamped_raw_pos {
                    // Cursor is positioned before this user character in the formatted view
                    return idx;
                }
                seen_user_chars += 1;
            }
        }

        // If we consumed all alphanumeric chars and still haven't reached clamped_raw_pos,
        // place cursor at the end of the formatted string.
        formatted.len()
    }

    fn formatted_to_raw(&self, raw: &str, formatted: &str, formatted_pos: usize) -> usize {
        let clamped_fmt_pos = formatted_pos.min(formatted.len());

        // Count alphanumerics in formatted up to formatted_pos.
        let mut seen_user_chars = 0usize;
        for (idx, ch) in formatted.char_indices() {
            if idx >= clamped_fmt_pos {
                break;
            }
            if ch.is_alphanumeric() {
                seen_user_chars += 1;
            }
        }

        // Map to raw position by clamping to raw char count
        let raw_len = raw.chars().count();
        seen_user_chars.min(raw_len)
    }
}

/// Result of invoking a custom formatter on the raw input.
///
/// Success variants carry the formatted string and a position mapper to translate
/// between raw and formatted cursor positions. If you don't provide a custom mapper,
/// the library will fall back to DefaultPositionMapper.
pub enum FormattingResult {
    /// Successfully produced a formatted display value and a position mapper.
    Success {
        formatted: String,
        /// Mapper to convert cursor positions between raw and formatted representations.
        mapper: Arc<dyn PositionMapper>,
    },
    /// Successfully produced a formatted value, but with a non-fatal warning message
    /// that can be shown in the UI (e.g., "incomplete value").
    Warning {
        formatted: String,
        message: String,
        mapper: Arc<dyn PositionMapper>,
    },
    /// Failed to produce a formatted display. The library will typically fall back to raw.
    Error {
        message: String,
    },
}

impl FormattingResult {
    /// Convenience to create a success result using the default mapper.
    pub fn success(formatted: impl Into<String>) -> Self {
        FormattingResult::Success {
            formatted: formatted.into(),
            mapper: Arc::new(DefaultPositionMapper::default()),
        }
    }

    /// Convenience to create a warning result using the default mapper.
    pub fn warning(formatted: impl Into<String>, message: impl Into<String>) -> Self {
        FormattingResult::Warning {
            formatted: formatted.into(),
            message: message.into(),
            mapper: Arc::new(DefaultPositionMapper::default()),
        }
    }

    /// Convenience to create a success result with a custom mapper.
    pub fn success_with_mapper(
        formatted: impl Into<String>,
        mapper: Arc<dyn PositionMapper>,
    ) -> Self {
        FormattingResult::Success {
            formatted: formatted.into(),
            mapper,
        }
    }

    /// Convenience to create a warning result with a custom mapper.
    pub fn warning_with_mapper(
        formatted: impl Into<String>,
        message: impl Into<String>,
        mapper: Arc<dyn PositionMapper>,
    ) -> Self {
        FormattingResult::Warning {
            formatted: formatted.into(),
            message: message.into(),
            mapper,
        }
    }

    /// Convenience to create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        FormattingResult::Error {
            message: message.into(),
        }
    }
}

/// A user-implemented formatter that turns raw input into a formatted display string,
/// optionally providing a custom cursor position mapper.
///
/// Notes:
/// - The library will keep raw input authoritative for editing and validation.
/// - The formatted value is only used for display.
/// - If formatting fails, return Error; the library will show the raw value.
/// - For common grouping (spaces/dashes), you can return Success/Warning and rely
///   on DefaultPositionMapper, or provide your own mapper for advanced cases
///   (reordering, compression, locale-specific rules, etc.).
pub trait CustomFormatter: Send + Sync {
    fn format(&self, raw: &str) -> FormattingResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct GroupEvery3;
    impl CustomFormatter for GroupEvery3 {
        fn format(&self, raw: &str) -> FormattingResult {
            let mut out = String::new();
            for (i, ch) in raw.chars().enumerate() {
                if i > 0 && i % 3 == 0 {
                    out.push(' ');
                }
                out.push(ch);
            }
            FormattingResult::success(out)
        }
    }

    #[test]
    fn default_mapper_roundtrip_basic() {
        let mapper = DefaultPositionMapper::default();
        let raw = "01001";
        let formatted = "010 01";

        // raw_to_formatted monotonicity and bounds
        for rp in 0..=raw.chars().count() {
            let fp = mapper.raw_to_formatted(raw, formatted, rp);
            assert!(fp <= formatted.len());
        }

        // formatted_to_raw bounds
        for fp in 0..=formatted.len() {
            let rp = mapper.formatted_to_raw(raw, formatted, fp);
            assert!(rp <= raw.chars().count());
        }
    }

    #[test]
    fn formatter_groups_every_3() {
        let f = GroupEvery3;
        match f.format("1234567") {
            FormattingResult::Success { formatted, .. } => {
                assert_eq!(formatted, "123 456 7");
            }
            _ => panic!("expected success"),
        }
    }
}