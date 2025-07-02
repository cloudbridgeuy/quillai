//! Basic usage examples for the QuillAI Delta library
//!
//! This example demonstrates the fundamental operations of the Delta format:
//! - Creating documents with formatted text
//! - Applying changes to documents
//! - Inverting changes for undo functionality
//! - Transforming concurrent changes
//! - Tracking position changes
//!
//! Run with: `cargo run --example basic_usage`

use quillai_delta::{AttributeValue, Delta};
use std::collections::BTreeMap;

fn main() {
    println!("=== QuillAI Delta Basic Usage Example ===\n");

    // Create a document Delta
    // This represents the text "Gandalf the Grey" with:
    // - "Gandalf" in bold
    // - " the " with no formatting
    // - "Grey" in gray color
    let mut bold_attrs = BTreeMap::new();
    bold_attrs.insert("bold".to_string(), AttributeValue::Boolean(true));

    let mut color_attrs = BTreeMap::new();
    color_attrs.insert(
        "color".to_string(),
        AttributeValue::String("#ccc".to_string()),
    );

    let document = Delta::new()
        .insert("Gandalf", Some(bold_attrs))
        .insert(" the ", None)
        .insert("Grey", Some(color_attrs));

    println!("1. Document Creation:");
    println!("   Document: {:?}", document);
    println!("   Document length: {} characters", document.length());
    println!("   Text: \"Gandalf the Grey\" (with formatting)\n");

    // Create a change Delta
    // This change will:
    // 1. Keep "Gandalf the " (12 characters)
    // 2. Insert "White" with white color
    // 3. Delete "Grey" (4 characters)
    // Result: "Gandalf the White"
    let mut white_attrs = BTreeMap::new();
    white_attrs.insert(
        "color".to_string(),
        AttributeValue::String("#fff".to_string()),
    );

    let change = Delta::new()
        .retain(12, None) // Keep the first 12 characters
        .insert("White", Some(white_attrs)) // Insert "White"
        .delete(4); // Delete "Grey"

    println!("2. Creating a Change:");
    println!("   Change: {:?}", change);
    println!("   Net length change: {} characters", change.change_length());

    // Apply the change using compose
    // Compose combines the document with the change to produce a new document
    let result = document.compose(&change);
    println!("\n3. Applying the Change:");
    println!("   Result: {:?}", result);
    println!("   Result text: \"Gandalf the White\"\n");

    // Create inverted change for undo functionality
    // The inverted change, when applied to the result, will restore the original
    let inverted = change.invert(&document);
    println!("4. Inverting for Undo:");
    println!("   Inverted: {:?}", inverted);
    println!("   This change would restore \"Grey\" and remove \"White\"\n");

    // Test round-trip: document -> change -> invert should equal original
    let roundtrip = result.compose(&inverted);
    println!("5. Round-trip Test:");
    println!("   Roundtrip: {:?}", roundtrip);
    println!("   Should match original document\n");

    // Transform example for concurrent editing
    // Two users are inserting at the beginning of a document
    // User A inserts "A" at position 0
    // User B inserts "B" at position 0
    // We need to transform B's change to account for A's insertion
    println!("6. Operational Transformation:");
    let change_a = Delta::new().insert("A", None).retain(5, None);
    let change_b = Delta::new().insert("B", None).retain(5, None);

    let transformed_b = change_a.transform(&change_b, true);
    println!("   User A's change: {:?}", change_a);
    println!("   User B's change: {:?}", change_b);
    println!("   Transformed B: {:?}", transformed_b);
    println!("   B's insertion is now at position 1 (after A's insertion)\n");

    // Position transformation
    // When text is inserted, positions after the insertion point need to be adjusted
    println!("7. Position Transformation:");
    let index = 5;
    let change_delta = Delta::new().insert("Hello ", None);
    let new_index = change_delta.transform_position(index, false);
    println!("   Original position: {}", index);
    println!("   Change: Insert \"Hello \" at beginning", );
    println!("   New position: {}", new_index);
    println!("   (Position shifted by length of insertion)");
}

