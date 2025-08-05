// src/validation/patterns.rs
//! Position-based pattern filtering for validation

use serde::{Deserialize, Serialize};

/// A filter that applies to specific character positions in a field
#[derive(Debug, Clone)]
pub struct PositionFilter {
    /// Which positions this filter applies to
    pub positions: PositionRange,
    /// What type of character filter to apply
    pub filter: CharacterFilter,
}

/// Defines which character positions a filter applies to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionRange {
    /// Single position (e.g., position 3 only)
    Single(usize),
    /// Range of positions (e.g., positions 0-2, inclusive)
    Range(usize, usize),
    /// From position onwards (e.g., position 4 and beyond)
    From(usize),
    /// Multiple specific positions (e.g., positions 0, 2, 5)
    Multiple(Vec<usize>),
}

/// Types of character filters that can be applied
#[derive(Debug, Clone)]
pub enum CharacterFilter {
    /// Allow only alphabetic characters (a-z, A-Z)
    Alphabetic,
    /// Allow only numeric characters (0-9)
    Numeric,
    /// Allow alphanumeric characters (a-z, A-Z, 0-9)
    Alphanumeric,
    /// Allow only exact character match
    Exact(char),
    /// Allow any character from the provided set
    OneOf(Vec<char>),
    /// Custom user-defined filter function
    Custom(Box<dyn Fn(char) -> bool + Send + Sync>),
}

impl PositionRange {
    /// Check if a position is included in this range
    pub fn contains(&self, position: usize) -> bool {
        match self {
            PositionRange::Single(pos) => position == *pos,
            PositionRange::Range(start, end) => position >= *start && position <= *end,
            PositionRange::From(start) => position >= *start,
            PositionRange::Multiple(positions) => positions.contains(&position),
        }
    }
    
    /// Get all positions up to a given length that this range covers
    pub fn positions_up_to(&self, max_length: usize) -> Vec<usize> {
        match self {
            PositionRange::Single(pos) => {
                if *pos < max_length { vec![*pos] } else { vec![] }
            },
            PositionRange::Range(start, end) => {
                let actual_end = (*end).min(max_length.saturating_sub(1));
                if *start <= actual_end {
                    (*start..=actual_end).collect()
                } else {
                    vec![]
                }
            },
            PositionRange::From(start) => {
                if *start < max_length {
                    (*start..max_length).collect()
                } else {
                    vec![]
                }
            },
            PositionRange::Multiple(positions) => {
                positions.iter()
                    .filter(|&&pos| pos < max_length)
                    .copied()
                    .collect()
            },
        }
    }
}

impl CharacterFilter {
    /// Test if a character passes this filter
    pub fn accepts(&self, ch: char) -> bool {
        match self {
            CharacterFilter::Alphabetic => ch.is_alphabetic(),
            CharacterFilter::Numeric => ch.is_numeric(),
            CharacterFilter::Alphanumeric => ch.is_alphanumeric(),
            CharacterFilter::Exact(expected) => ch == *expected,
            CharacterFilter::OneOf(chars) => chars.contains(&ch),
            CharacterFilter::Custom(func) => func(ch),
        }
    }
    
    /// Get a human-readable description of this filter
    pub fn description(&self) -> String {
        match self {
            CharacterFilter::Alphabetic => "alphabetic characters (a-z, A-Z)".to_string(),
            CharacterFilter::Numeric => "numeric characters (0-9)".to_string(),
            CharacterFilter::Alphanumeric => "alphanumeric characters (a-z, A-Z, 0-9)".to_string(),
            CharacterFilter::Exact(ch) => format!("exactly '{}'", ch),
            CharacterFilter::OneOf(chars) => {
                let char_list: String = chars.iter().collect();
                format!("one of: {}", char_list)
            },
            CharacterFilter::Custom(_) => "custom filter".to_string(),
        }
    }
}

impl PositionFilter {
    /// Create a new position filter
    pub fn new(positions: PositionRange, filter: CharacterFilter) -> Self {
        Self { positions, filter }
    }
    
    /// Validate a character at a specific position
    pub fn validate_position(&self, position: usize, character: char) -> bool {
        if self.positions.contains(position) {
            self.filter.accepts(character)
        } else {
            true // Position not covered by this filter, allow any character
        }
    }
    
    /// Get error message for invalid character at position
    pub fn error_message(&self, position: usize, character: char) -> Option<String> {
        if self.positions.contains(position) && !self.filter.accepts(character) {
            Some(format!(
                "Position {} requires {} but got '{}'",
                position,
                self.filter.description(),
                character
            ))
        } else {
            None
        }
    }
}

/// A collection of position filters for a field
#[derive(Debug, Clone, Default)]
pub struct PatternFilters {
    filters: Vec<PositionFilter>,
}

impl PatternFilters {
    /// Create empty pattern filters
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a position filter
    pub fn add_filter(mut self, filter: PositionFilter) -> Self {
        self.filters.push(filter);
        self
    }
    
    /// Add multiple filters
    pub fn add_filters(mut self, filters: Vec<PositionFilter>) -> Self {
        self.filters.extend(filters);
        self
    }
    
    /// Validate a character at a specific position against all applicable filters
    pub fn validate_char_at_position(&self, position: usize, character: char) -> Result<(), String> {
        for filter in &self.filters {
            if let Some(error) = filter.error_message(position, character) {
                return Err(error);
            }
        }
        Ok(())
    }
    
    /// Validate entire text against all filters
    pub fn validate_text(&self, text: &str) -> Result<(), String> {
        for (position, character) in text.char_indices() {
            if let Err(error) = self.validate_char_at_position(position, character) {
                return Err(error);
            }
        }
        Ok(())
    }
    
    /// Check if any filters are configured
    pub fn has_filters(&self) -> bool {
        !self.filters.is_empty()
    }
    
    /// Get all configured filters
    pub fn filters(&self) -> &[PositionFilter] {
        &self.filters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_range_contains() {
        assert!(PositionRange::Single(3).contains(3));
        assert!(!PositionRange::Single(3).contains(2));
        
        assert!(PositionRange::Range(1, 4).contains(3));
        assert!(!PositionRange::Range(1, 4).contains(5));
        
        assert!(PositionRange::From(2).contains(5));
        assert!(!PositionRange::From(2).contains(1));
        
        assert!(PositionRange::Multiple(vec![0, 2, 5]).contains(2));
        assert!(!PositionRange::Multiple(vec![0, 2, 5]).contains(3));
    }
    
    #[test]
    fn test_position_range_positions_up_to() {
        assert_eq!(PositionRange::Single(3).positions_up_to(5), vec![3]);
        assert_eq!(PositionRange::Single(5).positions_up_to(3), vec![]);
        
        assert_eq!(PositionRange::Range(1, 3).positions_up_to(5), vec![1, 2, 3]);
        assert_eq!(PositionRange::Range(1, 5).positions_up_to(3), vec![1, 2]);
        
        assert_eq!(PositionRange::From(2).positions_up_to(5), vec![2, 3, 4]);
        
        assert_eq!(PositionRange::Multiple(vec![0, 2, 5]).positions_up_to(4), vec![0, 2]);
    }
    
    #[test]
    fn test_character_filter_accepts() {
        assert!(CharacterFilter::Alphabetic.accepts('a'));
        assert!(CharacterFilter::Alphabetic.accepts('Z'));
        assert!(!CharacterFilter::Alphabetic.accepts('1'));
        
        assert!(CharacterFilter::Numeric.accepts('5'));
        assert!(!CharacterFilter::Numeric.accepts('a'));
        
        assert!(CharacterFilter::Alphanumeric.accepts('a'));
        assert!(CharacterFilter::Alphanumeric.accepts('5'));
        assert!(!CharacterFilter::Alphanumeric.accepts('-'));
        
        assert!(CharacterFilter::Exact('x').accepts('x'));
        assert!(!CharacterFilter::Exact('x').accepts('y'));
        
        assert!(CharacterFilter::OneOf(vec!['a', 'b', 'c']).accepts('b'));
        assert!(!CharacterFilter::OneOf(vec!['a', 'b', 'c']).accepts('d'));
    }
    
    #[test]
    fn test_position_filter_validation() {
        let filter = PositionFilter::new(
            PositionRange::Range(0, 1),
            CharacterFilter::Alphabetic,
        );
        
        assert!(filter.validate_position(0, 'A'));
        assert!(filter.validate_position(1, 'b'));
        assert!(!filter.validate_position(0, '1'));
        assert!(filter.validate_position(2, '1')); // Position 2 not covered, allow anything
    }
    
    #[test]
    fn test_pattern_filters_validation() {
        let patterns = PatternFilters::new()
            .add_filter(PositionFilter::new(
                PositionRange::Range(0, 1),
                CharacterFilter::Alphabetic,
            ))
            .add_filter(PositionFilter::new(
                PositionRange::Range(2, 4),
                CharacterFilter::Numeric,
            ));
        
        // Valid pattern: AB123
        assert!(patterns.validate_text("AB123").is_ok());
        
        // Invalid: number in alphabetic position
        assert!(patterns.validate_text("A1123").is_err());
        
        // Invalid: letter in numeric position
        assert!(patterns.validate_text("AB1A3").is_err());
    }
    
    #[test]
    fn test_custom_filter() {
        let pattern = PatternFilters::new()
            .add_filter(PositionFilter::new(
                PositionRange::From(0),
                CharacterFilter::Custom(Box::new(|c| c.is_lowercase())),
            ));
        
        assert!(pattern.validate_text("hello").is_ok());
        assert!(pattern.validate_text("Hello").is_err()); // Uppercase not allowed
    }
}
