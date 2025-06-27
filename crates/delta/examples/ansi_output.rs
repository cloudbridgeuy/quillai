use quillai_delta::{AttributeValue, Delta};
use std::collections::BTreeMap;

/// ANSI escape codes for text formatting
struct AnsiCodes;

impl AnsiCodes {
    // Reset
    const RESET: &'static str = "\x1b[0m";

    // Text styles
    const BOLD: &'static str = "\x1b[1m";
    const DIM: &'static str = "\x1b[2m";
    const ITALIC: &'static str = "\x1b[3m";
    const UNDERLINE: &'static str = "\x1b[4m";
    const STRIKETHROUGH: &'static str = "\x1b[9m";

    // Foreground colors
    const FG_BLACK: &'static str = "\x1b[30m";
    const FG_RED: &'static str = "\x1b[31m";
    const FG_GREEN: &'static str = "\x1b[32m";
    const FG_YELLOW: &'static str = "\x1b[33m";
    const FG_BLUE: &'static str = "\x1b[34m";
    const FG_MAGENTA: &'static str = "\x1b[35m";
    const FG_CYAN: &'static str = "\x1b[36m";
    const FG_WHITE: &'static str = "\x1b[37m";

    // Background colors
    const BG_BLACK: &'static str = "\x1b[40m";
    const BG_RED: &'static str = "\x1b[41m";
    const BG_GREEN: &'static str = "\x1b[42m";
    const BG_YELLOW: &'static str = "\x1b[43m";
    const BG_BLUE: &'static str = "\x1b[44m";
    const BG_MAGENTA: &'static str = "\x1b[45m";
    const BG_CYAN: &'static str = "\x1b[46m";
    const BG_WHITE: &'static str = "\x1b[47m";

    // Bright foreground colors
    const FG_BRIGHT_BLACK: &'static str = "\x1b[90m";
    const FG_BRIGHT_RED: &'static str = "\x1b[91m";
    const FG_BRIGHT_GREEN: &'static str = "\x1b[92m";
    const FG_BRIGHT_YELLOW: &'static str = "\x1b[93m";
    const FG_BRIGHT_BLUE: &'static str = "\x1b[94m";
    const FG_BRIGHT_MAGENTA: &'static str = "\x1b[95m";
    const FG_BRIGHT_CYAN: &'static str = "\x1b[96m";
    const FG_BRIGHT_WHITE: &'static str = "\x1b[97m";
}

/// Convert Delta attributes to ANSI escape codes
fn attributes_to_ansi(attributes: Option<&BTreeMap<String, AttributeValue>>) -> String {
    let mut ansi_codes = Vec::new();
    let mut rgb_codes = Vec::new(); // Store owned RGB strings

    if let Some(attrs) = attributes {
        for (key, value) in attrs {
            match (key.as_str(), value) {
                ("bold", AttributeValue::Boolean(true)) => ansi_codes.push(AnsiCodes::BOLD),
                ("italic", AttributeValue::Boolean(true)) => ansi_codes.push(AnsiCodes::ITALIC),
                ("underline", AttributeValue::Boolean(true)) => {
                    ansi_codes.push(AnsiCodes::UNDERLINE)
                }
                ("strikethrough", AttributeValue::Boolean(true)) => {
                    ansi_codes.push(AnsiCodes::STRIKETHROUGH)
                }
                ("dim", AttributeValue::Boolean(true)) => ansi_codes.push(AnsiCodes::DIM),

                ("color", AttributeValue::String(color)) => {
                    match color.as_str() {
                        "black" => ansi_codes.push(AnsiCodes::FG_BLACK),
                        "red" => ansi_codes.push(AnsiCodes::FG_RED),
                        "green" => ansi_codes.push(AnsiCodes::FG_GREEN),
                        "yellow" => ansi_codes.push(AnsiCodes::FG_YELLOW),
                        "blue" => ansi_codes.push(AnsiCodes::FG_BLUE),
                        "magenta" => ansi_codes.push(AnsiCodes::FG_MAGENTA),
                        "cyan" => ansi_codes.push(AnsiCodes::FG_CYAN),
                        "white" => ansi_codes.push(AnsiCodes::FG_WHITE),
                        "bright_black" => ansi_codes.push(AnsiCodes::FG_BRIGHT_BLACK),
                        "bright_red" => ansi_codes.push(AnsiCodes::FG_BRIGHT_RED),
                        "bright_green" => ansi_codes.push(AnsiCodes::FG_BRIGHT_GREEN),
                        "bright_yellow" => ansi_codes.push(AnsiCodes::FG_BRIGHT_YELLOW),
                        "bright_blue" => ansi_codes.push(AnsiCodes::FG_BRIGHT_BLUE),
                        "bright_magenta" => ansi_codes.push(AnsiCodes::FG_BRIGHT_MAGENTA),
                        "bright_cyan" => ansi_codes.push(AnsiCodes::FG_BRIGHT_CYAN),
                        "bright_white" => ansi_codes.push(AnsiCodes::FG_BRIGHT_WHITE),
                        _ => {
                            // Try to parse RGB hex color (#RRGGBB)
                            if color.starts_with('#') && color.len() == 7 {
                                if let (Ok(r), Ok(g), Ok(b)) = (
                                    u8::from_str_radix(&color[1..3], 16),
                                    u8::from_str_radix(&color[3..5], 16),
                                    u8::from_str_radix(&color[5..7], 16),
                                ) {
                                    let rgb_code = format!("\x1b[38;2;{};{};{}m", r, g, b);
                                    rgb_codes.push(rgb_code);
                                }
                            }
                        }
                    }
                }

                ("background", AttributeValue::String(color)) => {
                    match color.as_str() {
                        "black" => ansi_codes.push(AnsiCodes::BG_BLACK),
                        "red" => ansi_codes.push(AnsiCodes::BG_RED),
                        "green" => ansi_codes.push(AnsiCodes::BG_GREEN),
                        "yellow" => ansi_codes.push(AnsiCodes::BG_YELLOW),
                        "blue" => ansi_codes.push(AnsiCodes::BG_BLUE),
                        "magenta" => ansi_codes.push(AnsiCodes::BG_MAGENTA),
                        "cyan" => ansi_codes.push(AnsiCodes::BG_CYAN),
                        "white" => ansi_codes.push(AnsiCodes::BG_WHITE),
                        _ => {
                            // Try to parse RGB hex color for background
                            if color.starts_with('#') && color.len() == 7 {
                                if let (Ok(r), Ok(g), Ok(b)) = (
                                    u8::from_str_radix(&color[1..3], 16),
                                    u8::from_str_radix(&color[3..5], 16),
                                    u8::from_str_radix(&color[5..7], 16),
                                ) {
                                    let rgb_code = format!("\x1b[48;2;{};{};{}m", r, g, b);
                                    rgb_codes.push(rgb_code);
                                }
                            }
                        }
                    }
                }
                _ => {} // Ignore unknown attributes
            }
        }
    }

    // Combine static codes and RGB codes
    let mut all_codes = ansi_codes.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    all_codes.extend(rgb_codes);
    all_codes.join("")
}

/// Convert a Delta to an ANSI-formatted string
fn delta_to_ansi_string(delta: &Delta) -> String {
    let mut result = String::new();
    let mut current_formatting = String::new();

    for op in delta.ops() {
        match op {
            quillai_delta::Op::Insert { text, attributes } => {
                // Reset and apply new formatting
                if !current_formatting.is_empty() {
                    result.push_str(AnsiCodes::RESET);
                }

                current_formatting = attributes_to_ansi(attributes.as_ref());
                result.push_str(&current_formatting);
                result.push_str(text);
            }

            quillai_delta::Op::InsertEmbed { embed, attributes } => {
                // Handle embeds as special formatted text
                if !current_formatting.is_empty() {
                    result.push_str(AnsiCodes::RESET);
                }

                current_formatting = attributes_to_ansi(attributes.as_ref());
                result.push_str(&current_formatting);
                result.push_str(&format!("[{}:{}]", embed.embed_type, embed.data));
            }

            quillai_delta::Op::Retain {
                length: _,
                attributes,
            } => {
                // For retain operations, we would need the base document to apply to
                // In this simplified example, we just update the current formatting
                if attributes.is_some() {
                    if !current_formatting.is_empty() {
                        result.push_str(AnsiCodes::RESET);
                    }
                    current_formatting = attributes_to_ansi(attributes.as_ref());
                    result.push_str(&current_formatting);
                }
            }

            quillai_delta::Op::Delete(_) | quillai_delta::Op::RetainEmbed { .. } => {
                // Delete and RetainEmbed operations don't produce output in this context
                // They would be used when applying changes to an existing document
            }
        }
    }

    // Reset formatting at the end
    if !current_formatting.is_empty() {
        result.push_str(AnsiCodes::RESET);
    }

    result
}

fn main() {
    println!("🎨 Delta to ANSI Terminal Output Demo 🎨\n");

    // Create various styled text examples
    let mut red_bold = BTreeMap::new();
    red_bold.insert(
        "color".to_string(),
        AttributeValue::String("red".to_string()),
    );
    red_bold.insert("bold".to_string(), AttributeValue::Boolean(true));

    let mut green_italic = BTreeMap::new();
    green_italic.insert(
        "color".to_string(),
        AttributeValue::String("green".to_string()),
    );
    green_italic.insert("italic".to_string(), AttributeValue::Boolean(true));

    let mut blue_underline = BTreeMap::new();
    blue_underline.insert(
        "color".to_string(),
        AttributeValue::String("blue".to_string()),
    );
    blue_underline.insert("underline".to_string(), AttributeValue::Boolean(true));

    let mut yellow_bg = BTreeMap::new();
    yellow_bg.insert(
        "color".to_string(),
        AttributeValue::String("black".to_string()),
    );
    yellow_bg.insert(
        "background".to_string(),
        AttributeValue::String("yellow".to_string()),
    );

    let mut rainbow_attrs = BTreeMap::new();
    rainbow_attrs.insert(
        "color".to_string(),
        AttributeValue::String("#FF00FF".to_string()),
    ); // Magenta
    rainbow_attrs.insert("bold".to_string(), AttributeValue::Boolean(true));

    let mut dim_strikethrough = BTreeMap::new();
    dim_strikethrough.insert("dim".to_string(), AttributeValue::Boolean(true));
    dim_strikethrough.insert("strikethrough".to_string(), AttributeValue::Boolean(true));

    // Example 1: Basic styled text
    println!("Example 1: Basic Styled Text");
    let basic_delta = Delta::new()
        .insert("ERROR: ", Some(red_bold.clone()))
        .insert("Something went wrong!\n", None)
        .insert("SUCCESS: ", Some(green_italic.clone()))
        .insert("Operation completed!\n", None)
        .insert("INFO: ", Some(blue_underline.clone()))
        .insert("Please check the logs.\n", None);

    let ansi_output = delta_to_ansi_string(&basic_delta);
    println!("{}", ansi_output);

    // Example 2: Terminal progress bar simulation
    println!("Example 2: Progress Bar");
    let mut progress_bar = BTreeMap::new();
    progress_bar.insert(
        "background".to_string(),
        AttributeValue::String("green".to_string()),
    );
    progress_bar.insert(
        "color".to_string(),
        AttributeValue::String("white".to_string()),
    );

    let mut progress_empty = BTreeMap::new();
    progress_empty.insert(
        "background".to_string(),
        AttributeValue::String("black".to_string()),
    );
    progress_empty.insert(
        "color".to_string(),
        AttributeValue::String("white".to_string()),
    );

    let progress_delta = Delta::new()
        .insert("Progress: [", None)
        .insert("████████████", Some(progress_bar))
        .insert("░░░░░░░░", Some(progress_empty))
        .insert("] 60%\n", None);

    let progress_output = delta_to_ansi_string(&progress_delta);
    println!("{}", progress_output);

    // Example 3: Code syntax highlighting simulation
    println!("Example 3: Code Syntax Highlighting");
    let mut keyword = BTreeMap::new();
    keyword.insert(
        "color".to_string(),
        AttributeValue::String("blue".to_string()),
    );
    keyword.insert("bold".to_string(), AttributeValue::Boolean(true));

    let mut string_literal = BTreeMap::new();
    string_literal.insert(
        "color".to_string(),
        AttributeValue::String("green".to_string()),
    );

    let mut comment = BTreeMap::new();
    comment.insert(
        "color".to_string(),
        AttributeValue::String("bright_black".to_string()),
    );
    comment.insert("italic".to_string(), AttributeValue::Boolean(true));

    let code_delta = Delta::new()
        .insert("fn ", Some(keyword.clone()))
        .insert("main", None)
        .insert("() {\n", None)
        .insert("    println!", Some(keyword.clone()))
        .insert("(", None)
        .insert("\"Hello, World!\"", Some(string_literal))
        .insert(");\n", None)
        .insert("    // This is a comment\n", Some(comment))
        .insert("}", None);

    let code_output = delta_to_ansi_string(&code_delta);
    println!("{}", code_output);

    // Example 4: Warning/Error messages with highlighting
    println!("\nExample 4: System Messages");
    let warning_delta = Delta::new()
        .insert("⚠️  WARNING: ", Some(yellow_bg.clone()))
        .insert("Deprecated function used!\n", None)
        .insert("🎉 SUCCESS: ", Some(green_italic.clone()))
        .insert("All tests passed!\n", None)
        .insert("❌ ERROR: ", Some(red_bold.clone()))
        .insert("Connection failed!\n", None)
        .insert("📝 NOTE: ", Some(blue_underline.clone()))
        .insert("Check configuration file.\n", None);

    let warning_output = delta_to_ansi_string(&warning_delta);
    println!("{}", warning_output);

    // Example 5: Demonstrate Delta composition with ANSI
    println!("Example 5: Delta Composition");
    let base_text = Delta::new()
        .insert("Base text ", None)
        .insert("in normal formatting.\n", None);

    let formatting_change = Delta::new()
        .retain(10, None) // Keep "Base text "
        .insert("with BOLD ", Some(red_bold.clone()))
        .retain(21, Some(rainbow_attrs.clone())); // Make the rest colorful

    let composed = base_text.compose(&formatting_change);
    let composed_output = delta_to_ansi_string(&composed);
    println!("Composed result: {}", composed_output);

    // Example 6: Show strikethrough and dim effects
    println!("Example 6: Special Effects");
    let effects_delta = Delta::new()
        .insert("This text is ", None)
        .insert("crossed out", Some(dim_strikethrough))
        .insert(" and this is normal.\n", None);

    let effects_output = delta_to_ansi_string(&effects_delta);
    println!("{}", effects_output);

    // Example 7: RGB color demonstration
    println!("Example 7: RGB Colors");
    let mut rgb_red = BTreeMap::new();
    rgb_red.insert(
        "color".to_string(),
        AttributeValue::String("#FF0000".to_string()),
    );

    let mut rgb_green = BTreeMap::new();
    rgb_green.insert(
        "color".to_string(),
        AttributeValue::String("#00FF00".to_string()),
    );

    let mut rgb_blue = BTreeMap::new();
    rgb_blue.insert(
        "color".to_string(),
        AttributeValue::String("#0000FF".to_string()),
    );

    let rgb_delta = Delta::new()
        .insert("R", Some(rgb_red))
        .insert("G", Some(rgb_green))
        .insert("B", Some(rgb_blue))
        .insert(" colors using hex values!\n", None);

    let rgb_output = delta_to_ansi_string(&rgb_delta);
    println!("{}", rgb_output);

    println!("✨ Demo complete! ✨");
}
