// src/validation/mask.rs
//! Pure display mask system - user-defined patterns only

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum MaskDisplayMode {
    /// Only show separators as user types
    /// Example: "" → "", "123" → "123", "12345" → "(123) 45"
    #[default]
    Dynamic,
    
    /// Show full template with placeholders from start
    /// Example: "" → "(___) ___-____", "123" → "(123) ___-____"
    Template { 
        /// Character to use as placeholder for empty input positions
        placeholder: char 
    },
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayMask {
    /// Mask pattern like "##-##-####" where # = input position, others are visual separators
    pattern: String,
    /// Character used to represent input positions (usually '#')
    input_char: char,
    /// How to display the mask (dynamic vs template)
    display_mode: MaskDisplayMode,
}

impl DisplayMask {
    /// Create a new display mask with dynamic mode (current behavior)
    /// 
    /// # Arguments
    /// * `pattern` - The mask pattern (e.g., "##-##-####", "(###) ###-####")  
    /// * `input_char` - Character representing input positions (usually '#')
    /// 
    /// # Examples
    /// ```
    /// // Phone number format
    /// let phone_mask = DisplayMask::new("(###) ###-####", '#');
    /// 
    /// // Date format  
    /// let date_mask = DisplayMask::new("##/##/####", '#');
    /// 
    /// // Custom business format
    /// let employee_id = DisplayMask::new("EMP-####-##", '#');
    /// ```
    pub fn new(pattern: impl Into<String>, input_char: char) -> Self {
        Self {
            pattern: pattern.into(),
            input_char,
            display_mode: MaskDisplayMode::Dynamic,
        }
    }

    /// Set the display mode for this mask
    /// 
    /// # Examples
    /// ```
    /// let dynamic_mask = DisplayMask::new("##-##", '#')
    ///     .with_mode(MaskDisplayMode::Dynamic);
    ///     
    /// let template_mask = DisplayMask::new("##-##", '#')
    ///     .with_mode(MaskDisplayMode::Template { placeholder: '_' });
    /// ```
    pub fn with_mode(mut self, mode: MaskDisplayMode) -> Self {
        self.display_mode = mode;
        self
    }

    /// Set template mode with custom placeholder
    /// 
    /// # Examples
    /// ```
    /// let phone_template = DisplayMask::new("(###) ###-####", '#')
    ///     .with_template('_');  // Shows "(___) ___-____" when empty
    ///     
    /// let date_dots = DisplayMask::new("##/##/####", '#')
    ///     .with_template('•');  // Shows "••/••/••••" when empty
    /// ```
    pub fn with_template(self, placeholder: char) -> Self {
        self.with_mode(MaskDisplayMode::Template { placeholder })
    }

    /// Apply mask to raw input, showing visual separators and handling display mode
    pub fn apply_to_display(&self, raw_input: &str) -> String {
        match &self.display_mode {
            MaskDisplayMode::Dynamic => self.apply_dynamic(raw_input),
            MaskDisplayMode::Template { placeholder } => self.apply_template(raw_input, *placeholder),
        }
    }

    /// Dynamic mode - only show separators as user types
    fn apply_dynamic(&self, raw_input: &str) -> String {
        let mut result = String::new();
        let mut raw_chars = raw_input.chars();

        for pattern_char in self.pattern.chars() {
            if pattern_char == self.input_char {
                // Input position - take from raw input
                if let Some(input_char) = raw_chars.next() {
                    result.push(input_char);
                } else {
                    // No more input - stop here in dynamic mode
                    break;
                }
            } else {
                // Visual separator - always show
                result.push(pattern_char);
            }
        }

        // Append any remaining raw characters that don't fit the pattern
        for remaining_char in raw_chars {
            result.push(remaining_char);
        }

        result
    }

    /// Template mode - show full pattern with placeholders
    fn apply_template(&self, raw_input: &str, placeholder: char) -> String {
        let mut result = String::new();
        let mut raw_chars = raw_input.chars().peekable();

        for pattern_char in self.pattern.chars() {
            if pattern_char == self.input_char {
                // Input position - take from raw input or use placeholder
                if let Some(input_char) = raw_chars.next() {
                    result.push(input_char);
                } else {
                    // No more input - use placeholder to show template
                    result.push(placeholder);
                }
            } else {
                // Visual separator - always show in template mode
                result.push(pattern_char);
            }
        }

        // In template mode, we don't append extra characters beyond the pattern
        // This keeps the template consistent
        result
    }

    /// Check if a display position should accept cursor/input
    pub fn is_input_position(&self, display_position: usize) -> bool {
        self.pattern.chars()
            .nth(display_position)
            .map(|c| c == self.input_char)
            .unwrap_or(true) // Beyond pattern = accept input
    }

    /// Map display position to raw position
    pub fn display_pos_to_raw_pos(&self, display_pos: usize) -> usize {
        let mut raw_pos = 0;

        for (i, pattern_char) in self.pattern.chars().enumerate() {
            if i >= display_pos {
                break;
            }
            if pattern_char == self.input_char {
                raw_pos += 1;
            }
        }

        raw_pos
    }

    /// Map raw position to display position
    pub fn raw_pos_to_display_pos(&self, raw_pos: usize) -> usize {
        let mut input_positions_seen = 0;

        for (display_pos, pattern_char) in self.pattern.chars().enumerate() {
            if pattern_char == self.input_char {
                if input_positions_seen == raw_pos {
                    return display_pos;
                }
                input_positions_seen += 1;
            }
        }

        // Beyond pattern, return position after pattern
        self.pattern.len() + (raw_pos - input_positions_seen)
    }

    /// Find next input position at or after the given display position
    pub fn next_input_position(&self, display_pos: usize) -> usize {
        for (i, pattern_char) in self.pattern.chars().enumerate().skip(display_pos) {
            if pattern_char == self.input_char {
                return i;
            }
        }
        // Beyond pattern = all positions are input positions
        display_pos.max(self.pattern.len())
    }

    /// Find previous input position at or before the given display position
    pub fn prev_input_position(&self, display_pos: usize) -> Option<usize> {
        // Collect pattern chars with indices first, then search backwards
        let pattern_chars: Vec<(usize, char)> = self.pattern.chars().enumerate().collect();
        
        // Search backwards from display_pos
        for &(i, pattern_char) in pattern_chars.iter().rev() {
            if i <= display_pos && pattern_char == self.input_char {
                return Some(i);
            }
        }
        None
    }

    /// Get the display mode
    pub fn display_mode(&self) -> &MaskDisplayMode {
        &self.display_mode
    }

    /// Check if this mask uses template mode
    pub fn is_template_mode(&self) -> bool {
        matches!(self.display_mode, MaskDisplayMode::Template { .. })
    }

    /// Get the pattern string
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Get the position of the first input character in the pattern
    pub fn first_input_position(&self) -> usize {
        for (pos, ch) in self.pattern.chars().enumerate() {
            if ch == self.input_char {
                return pos;
            }
        }
        0
    }
}

impl Default for DisplayMask {
    fn default() -> Self {
        Self::new("", '#')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_defined_phone_mask() {
        // User creates their own phone mask
        let dynamic = DisplayMask::new("(###) ###-####", '#');
        let template = DisplayMask::new("(###) ###-####", '#').with_template('_');
        
        // Dynamic mode
        assert_eq!(dynamic.apply_to_display(""), "");
        assert_eq!(dynamic.apply_to_display("1234567890"), "(123) 456-7890");
        
        // Template mode
        assert_eq!(template.apply_to_display(""), "(___) ___-____");
        assert_eq!(template.apply_to_display("123"), "(123) ___-____");
    }

    #[test]
    fn test_user_defined_date_mask() {
        // User creates their own date formats
        let us_date = DisplayMask::new("##/##/####", '#');
        let eu_date = DisplayMask::new("##.##.####", '#');
        let iso_date = DisplayMask::new("####-##-##", '#');
        
        assert_eq!(us_date.apply_to_display("12252024"), "12/25/2024");
        assert_eq!(eu_date.apply_to_display("25122024"), "25.12.2024");
        assert_eq!(iso_date.apply_to_display("20241225"), "2024-12-25");
    }

    #[test]
    fn test_user_defined_business_formats() {
        // User creates custom business formats
        let employee_id = DisplayMask::new("EMP-####-##", '#');
        let product_code = DisplayMask::new("###-###-###", '#');
        let invoice = DisplayMask::new("INV####/##", '#');
        
        assert_eq!(employee_id.apply_to_display("123456"), "EMP-1234-56");
        assert_eq!(product_code.apply_to_display("123456789"), "123-456-789");
        assert_eq!(invoice.apply_to_display("123456"), "INV1234/56");
    }

    #[test]
    fn test_custom_input_characters() {
        // User can define their own input character
        let mask_with_x = DisplayMask::new("XXX-XX-XXXX", 'X');
        let mask_with_hash = DisplayMask::new("###-##-####", '#');
        let mask_with_n = DisplayMask::new("NNN-NN-NNNN", 'N');
        
        assert_eq!(mask_with_x.apply_to_display("123456789"), "123-45-6789");
        assert_eq!(mask_with_hash.apply_to_display("123456789"), "123-45-6789");
        assert_eq!(mask_with_n.apply_to_display("123456789"), "123-45-6789");
    }

    #[test]
    fn test_custom_placeholders() {
        // User can define custom placeholder characters
        let underscores = DisplayMask::new("##-##", '#').with_template('_');
        let dots = DisplayMask::new("##-##", '#').with_template('•');
        let dashes = DisplayMask::new("##-##", '#').with_template('-');
        
        assert_eq!(underscores.apply_to_display(""), "__-__");
        assert_eq!(dots.apply_to_display(""), "••-••");
        assert_eq!(dashes.apply_to_display(""), "---");  // Note: dashes blend with separator
    }

    #[test]
    fn test_position_mapping_user_patterns() {
        let custom = DisplayMask::new("ABC-###-XYZ", '#');
        
        // Position mapping should work correctly with any pattern
        assert_eq!(custom.raw_pos_to_display_pos(0), 4); // First # at position 4
        assert_eq!(custom.raw_pos_to_display_pos(1), 5); // Second # at position 5
        assert_eq!(custom.raw_pos_to_display_pos(2), 6); // Third # at position 6
        
        assert_eq!(custom.display_pos_to_raw_pos(4), 0); // Position 4 -> first input
        assert_eq!(custom.display_pos_to_raw_pos(5), 1); // Position 5 -> second input
        assert_eq!(custom.display_pos_to_raw_pos(6), 2); // Position 6 -> third input
        
        assert!(!custom.is_input_position(0)); // A
        assert!(!custom.is_input_position(3)); // -
        assert!(custom.is_input_position(4));  // #
        assert!(!custom.is_input_position(8)); // Y
    }
}
