// examples/validation_patterns.rs
//! Example demonstrating position-based pattern filtering
//! 
//! Run with: cargo run --example validation_patterns --features validation

use canvas::{
    prelude::*,
    validation::{ValidationConfigBuilder, PatternFilters, PositionFilter, PositionRange, CharacterFilter},
};

#[derive(Debug)]
struct DocumentForm {
    license_plate: String,
    phone_number: String,
    credit_card: String,
    custom_id: String,
}

impl DocumentForm {
    fn new() -> Self {
        Self {
            license_plate: String::new(),
            phone_number: String::new(),
            credit_card: String::new(),
            custom_id: String::new(),
        }
    }
}

impl DataProvider for DocumentForm {
    fn field_count(&self) -> usize {
        4
    }

    fn field_name(&self, index: usize) -> &str {
        match index {
            0 => "License Plate",
            1 => "Phone Number", 
            2 => "Credit Card",
            3 => "Custom ID",
            _ => "",
        }
    }

    fn field_value(&self, index: usize) -> &str {
        match index {
            0 => &self.license_plate,
            1 => &self.phone_number,
            2 => &self.credit_card,
            3 => &self.custom_id,
            _ => "",
        }
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        match index {
            0 => self.license_plate = value,
            1 => self.phone_number = value,
            2 => self.credit_card = value,
            3 => self.custom_id = value,
            _ => {}
        }
    }

    fn validation_config(&self, field_index: usize) -> Option<ValidationConfig> {
        match field_index {
            0 => {
                // License plate: AB123 (2 letters, 3 numbers) - USER DEFINED
                let license_plate_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(0, 1),
                        CharacterFilter::Alphabetic,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(2, 4),
                        CharacterFilter::Numeric,
                    ));
                
                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(license_plate_pattern)
                    .build())
            }
            1 => {
                // Phone number: 123-456-7890 - USER DEFINED
                let phone_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![0,1,2,4,5,6,8,9,10,11]),
                        CharacterFilter::Numeric,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![3, 7]),
                        CharacterFilter::Exact('-'),
                    ));
                
                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(phone_pattern)
                    .build())
            }
            2 => {
                // Credit card: 1234-5678-9012-3456 - USER DEFINED
                let credit_card_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![0,1,2,3,5,6,7,8,10,11,12,13,15,16,17,18]),
                        CharacterFilter::Numeric,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![4, 9, 14]),
                        CharacterFilter::Exact('-'),
                    ));
                
                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(credit_card_pattern)
                    .build())
            }
            3 => {
                // Custom ID: First 2 letters, rest alphanumeric - USER DEFINED
                let custom_id_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(0, 1),
                        CharacterFilter::Alphabetic,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::From(2),
                        CharacterFilter::Alphanumeric,
                    ));
                
                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(custom_id_pattern)
                    .build())
            }
            _ => None,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Canvas Pattern Filtering Demo");
    println!("=================================");
    println!();
    
    let form = DocumentForm::new();
    let mut editor = FormEditor::new(form);
    
    println!("📋 Form initialized with USER-DEFINED pattern validation rules:");
    for i in 0..editor.data_provider().field_count() {
        let field_name = editor.data_provider().field_name(i);
        println!("  • {}: Position-based pattern filtering (user-defined)", field_name);
    }
    println!();
    
    // Test License Plate (Field 0)
    println!("1. Testing USER-DEFINED License Plate pattern (AB123 - 2 letters, 3 numbers):");
    
    // Valid license plate
    println!("   Entering valid license plate 'AB123':");
    for ch in "AB123".chars() {
        match editor.insert_char(ch) {
            Ok(_) => println!("     '{}' ✓ accepted", ch),
            Err(e) => println!("     '{}' ✗ rejected: {}", ch, e),
        }
    }
    println!("   Result: '{}'", editor.current_text());
    println!();
    
    // Clear and test invalid pattern
    editor.clear_current_field();
    println!("   Testing invalid pattern 'A1123':");
    for (i, ch) in "A1123".chars().enumerate() {
        match editor.insert_char(ch) {
            Ok(_) => println!("     Position {}: '{}' ✓ accepted", i, ch),
            Err(e) => println!("     Position {}: '{}' ✗ rejected: {}", i, ch, e),
        }
    }
    println!("   Result: '{}'", editor.current_text());
    println!();
    
    // Move to phone number field
    editor.move_to_next_field()?;
    
    // Test Phone Number (Field 1)
    println!("2. Testing USER-DEFINED Phone Number pattern (123-456-7890):");
    
    // Valid phone number
    println!("   Entering valid phone number '123-456-7890':");
    for (i, ch) in "123-456-7890".chars().enumerate() {
        match editor.insert_char(ch) {
            Ok(_) => println!("     Position {}: '{}' ✓ accepted", i, ch),
            Err(e) => println!("     Position {}: '{}' ✗ rejected: {}", i, ch, e),
        }
    }
    println!("   Result: '{}'", editor.current_text());
    println!();
    
    // Move to credit card field
    editor.move_to_next_field()?;
    
    // Test Credit Card (Field 2)
    println!("3. Testing USER-DEFINED Credit Card pattern (1234-5678-9012-3456):");
    
    // Valid credit card (first few characters)
    println!("   Entering valid credit card start '1234-56':");
    for (i, ch) in "1234-56".chars().enumerate() {
        match editor.insert_char(ch) {
            Ok(_) => println!("     Position {}: '{}' ✓ accepted", i, ch),
            Err(e) => println!("     Position {}: '{}' ✗ rejected: {}", i, ch, e),
        }
    }
    println!("   Result: '{}'", editor.current_text());
    println!();
    
    // Test invalid character at dash position
    println!("   Testing invalid character at dash position:");
    editor.clear_current_field();
    for (i, ch) in "1234A56".chars().enumerate() {
        match editor.insert_char(ch) {
            Ok(_) => println!("     Position {}: '{}' ✓ accepted", i, ch),
            Err(e) => println!("     Position {}: '{}' ✗ rejected: {}", i, ch, e),
        }
    }
    println!("   Result: '{}'", editor.current_text());
    println!();
    
    // Move to custom ID field
    editor.move_to_next_field()?;
    
    // Test Custom ID (Field 3)
    println!("4. Testing USER-DEFINED Custom ID pattern (2 letters + alphanumeric):");
    
    // Valid custom ID
    println!("   Entering valid custom ID 'AB123def':");
    for (i, ch) in "AB123def".chars().enumerate() {
        match editor.insert_char(ch) {
            Ok(_) => println!("     Position {}: '{}' ✓ accepted", i, ch),
            Err(e) => println!("     Position {}: '{}' ✗ rejected: {}", i, ch, e),
        }
    }
    println!("   Result: '{}'", editor.current_text());
    println!();
    
    // Test invalid pattern
    editor.clear_current_field();
    println!("   Testing invalid pattern '1B123def' (number in first position):");
    for (i, ch) in "1B123def".chars().enumerate() {
        match editor.insert_char(ch) {
            Ok(_) => println!("     Position {}: '{}' ✓ accepted", i, ch),
            Err(e) => println!("     Position {}: '{}' ✗ rejected: {}", i, ch, e),
        }
    }
    println!("   Result: '{}'", editor.current_text());
    println!();
    
    // Show validation summary
    println!("📊 Final validation summary:");
    let summary = editor.validation_summary();
    println!("   Total fields with validation: {}", summary.total_fields);
    println!("   Validated fields: {}", summary.validated_fields);
    println!("   Valid fields: {}", summary.valid_fields);
    println!("   Fields with warnings: {}", summary.warning_fields);
    println!("   Fields with errors: {}", summary.error_fields);
    println!();
    
    // Show field-by-field status
    println!("📝 Field-by-field validation status:");
    for i in 0..editor.data_provider().field_count() {
        let field_name = editor.data_provider().field_name(i);
        let field_value = editor.data_provider().field_value(i);
        
        if let Some(result) = editor.field_validation(i) {
            println!("   {} [{}]: {} - {:?}", 
                field_name,
                field_value,
                if result.is_acceptable() { "✓" } else { "✗" },
                result
            );
        } else {
            println!("   {} [{}]: (not validated)", field_name, field_value);
        }
    }
    
    println!();
    println!("✨ USER-DEFINED Pattern filtering demo completed!");
    println!("Key Features Demonstrated:");
    println!("  • Position-specific character filtering (USER DEFINES PATTERNS)");
    println!("  • Library provides CharacterFilter: Alphabetic, Numeric, Alphanumeric, Exact, OneOf, Custom");
    println!("  • User defines all patterns using library's building blocks");
    println!("  • Real-time validation during typing");
    println!("  • Flexible position ranges (single, range, from, multiple)");
    
    Ok(())
}
