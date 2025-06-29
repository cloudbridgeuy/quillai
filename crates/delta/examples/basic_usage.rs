use quillai_delta::{AttributeValue, Delta};
use std::collections::BTreeMap;

fn main() {
    // Create a document Delta
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

    println!("Document: {:?}", document);
    println!("Document length: {}", document.length());

    // Create a change Delta
    let mut white_attrs = BTreeMap::new();
    white_attrs.insert(
        "color".to_string(),
        AttributeValue::String("#fff".to_string()),
    );

    let change = Delta::new()
        .retain(12, None) // Keep the first 12 characters
        .insert("White", Some(white_attrs)) // Insert "White"
        .delete(4); // Delete "Grey"

    println!("Change: {:?}", change);
    println!("Change length: {}", change.change_length());

    // Apply the change
    let result = document.compose(&change);
    println!("Result: {:?}", result);

    // Create inverted change
    let inverted = change.invert(&document);
    println!("Inverted: {:?}", inverted);

    // Test round-trip: document -> change -> invert should equal original
    let roundtrip = result.compose(&inverted);
    println!("Roundtrip: {:?}", roundtrip);

    // Transform example
    let change_a = Delta::new().insert("A", None).retain(5, None);
    let change_b = Delta::new().insert("B", None).retain(5, None);

    let transformed_b = change_a.transform(&change_b, true);
    println!("Original change B: {:?}", change_b);
    println!("Transformed B: {:?}", transformed_b);

    // Position transformation
    let index = 5;
    let change_delta = Delta::new().insert("Hello ", None);
    let new_index = change_delta.transform_position(index, false);
    println!("Position {} becomes {}", index, new_index);
}

