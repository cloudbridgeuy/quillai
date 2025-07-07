//! Content rendering system for converting Delta operations to Dioxus RSX elements.
//!
//! This module provides utilities for rendering Delta document content as Dioxus RSX,
//! enabling the editor to display formatted text content properly.

use dioxus::prelude::*;
use quillai_delta::{Delta, Op, AttributeMap};
use std::collections::HashMap;

/// Render a Delta document as Dioxus RSX elements
///
/// Converts Delta operations into displayable RSX content that can be
/// rendered by the Dioxus component system.
///
/// # Arguments
///
/// * `delta` - The Delta document to render
///
/// # Returns
///
/// RSX element containing the rendered content
pub fn render_delta_to_rsx(delta: &Delta) -> Element {
    if delta.ops().is_empty() {
        return rsx! { div { class: "editor-content empty" } };
    }

    let content_elements = delta.ops().iter().map(|op| {
        render_operation_to_rsx(op)
    }).collect::<Vec<_>>();

    rsx! {
        div {
            class: "editor-content",
            {content_elements.into_iter()}
        }
    }
}

/// Render a single Delta operation as RSX
fn render_operation_to_rsx(operation: &Op) -> Element {
    match operation {
        Op::Insert { insert, attributes } => {
            render_insert_operation(insert, attributes.as_ref())
        }
        Op::Retain { .. } => {
            // Retain operations don't produce visible content in rendering
            rsx! { span { hidden: true } }
        }
        Op::Delete { .. } => {
            // Delete operations don't produce visible content in rendering
            rsx! { span { hidden: true } }
        }
        Op::InsertEmbed { .. } => {
            // Embed operations not yet implemented in Phase 1
            rsx! { span { class: "embed-placeholder", "[Embed]" } }
        }
        Op::RetainEmbed { .. } => {
            // Embed operations not yet implemented in Phase 1
            rsx! { span { hidden: true } }
        }
    }
}

/// Render an insert operation with optional formatting
fn render_insert_operation(content: &str, attributes: Option<&AttributeMap>) -> Element {
    if content.is_empty() {
        return rsx! { span {} };
    }

    // Check for line breaks and split content accordingly
    if content.contains('\n') {
        let lines: Vec<&str> = content.split('\n').collect();
        let line_elements = lines.into_iter().enumerate().map(|(index, line)| {
            if index > 0 {
                rsx! {
                    br {}
                    {render_text_with_formatting(line, attributes)}
                }
            } else {
                render_text_with_formatting(line, attributes)
            }
        }).collect::<Vec<_>>();

        return rsx! {
            span {
                {line_elements.into_iter()}
            }
        };
    }

    render_text_with_formatting(content, attributes)
}

/// Render text content with formatting attributes applied
fn render_text_with_formatting(text: &str, attributes: Option<&AttributeMap>) -> Element {
    if text.is_empty() {
        return rsx! { span {} };
    }

    match attributes {
        Some(attrs) => render_formatted_text(text, attrs),
        None => rsx! { span { "{text}" } },
    }
}

/// Apply formatting attributes to text content
fn render_formatted_text(text: &str, attributes: &AttributeMap) -> Element {
    // Handle block-level formatting first
    if let Some(header) = attributes.get("header") {
        if let Some(level) = header.as_f64() {
            let header_level = level as i32;
            return match header_level {
                1 => rsx! { h1 { {apply_inline_styles(text, attributes)} } },
                2 => rsx! { h2 { {apply_inline_styles(text, attributes)} } },
                3 => rsx! { h3 { {apply_inline_styles(text, attributes)} } },
                4 => rsx! { h4 { {apply_inline_styles(text, attributes)} } },
                5 => rsx! { h5 { {apply_inline_styles(text, attributes)} } },
                6 => rsx! { h6 { {apply_inline_styles(text, attributes)} } },
                _ => rsx! { h3 { {apply_inline_styles(text, attributes)} } },
            };
        }
    }

    if let Some(blockquote) = attributes.get("blockquote") {
        if blockquote.as_bool().unwrap_or(false) {
            return rsx! {
                blockquote {
                    class: "ql-blockquote",
                    {apply_inline_styles(text, attributes)}
                }
            };
        }
    }

    if let Some(code_block) = attributes.get("code-block") {
        if code_block.as_bool().unwrap_or(false) {
            return rsx! {
                pre {
                    class: "ql-code-block",
                    code { "{text}" }
                }
            };
        }
    }

    if let Some(list) = attributes.get("list") {
        if let Some(list_type) = list.as_str() {
            return match list_type {
                "ordered" => rsx! {
                    ol {
                        li { {apply_inline_styles(text, attributes)} }
                    }
                },
                "bullet" => rsx! {
                    ul {
                        li { {apply_inline_styles(text, attributes)} }
                    }
                },
                _ => rsx! {
                    ul {
                        li { {apply_inline_styles(text, attributes)} }
                    }
                },
            };
        }
    }

    // Apply inline formatting
    apply_inline_styles(text, attributes)
}

/// Apply inline formatting styles to text
fn apply_inline_styles(text: &str, attributes: &AttributeMap) -> Element {
    let mut element = rsx! { "{text}" };

    // Apply formatting in reverse order so they nest properly
    if let Some(link) = attributes.get("link") {
        if let Some(url) = link.as_str() {
            element = rsx! {
                a {
                    href: "{url}",
                    class: "ql-link",
                    target: "_blank",
                    rel: "noopener noreferrer",
                    {element}
                }
            };
        }
    }

    if let Some(code) = attributes.get("code") {
        if code.as_bool().unwrap_or(false) {
            element = rsx! {
                code {
                    class: "ql-code",
                    {element}
                }
            };
        }
    }

    if let Some(strike) = attributes.get("strike") {
        if strike.as_bool().unwrap_or(false) {
            element = rsx! {
                s {
                    class: "ql-strike",
                    {element}
                }
            };
        }
    }

    if let Some(underline) = attributes.get("underline") {
        if underline.as_bool().unwrap_or(false) {
            element = rsx! {
                u {
                    class: "ql-underline",
                    {element}
                }
            };
        }
    }

    if let Some(italic) = attributes.get("italic") {
        if italic.as_bool().unwrap_or(false) {
            element = rsx! {
                em {
                    class: "ql-italic",
                    {element}
                }
            };
        }
    }

    if let Some(bold) = attributes.get("bold") {
        if bold.as_bool().unwrap_or(false) {
            element = rsx! {
                strong {
                    class: "ql-bold",
                    {element}
                }
            };
        }
    }

    // Apply style attributes
    let style = build_inline_style(attributes);
    if !style.is_empty() {
        element = rsx! {
            span {
                style: "{style}",
                {element}
            }
        };
    }

    element
}

/// Build CSS style string from Delta attributes
fn build_inline_style(attributes: &AttributeMap) -> String {
    let mut styles = Vec::new();

    if let Some(color) = attributes.get("color") {
        if let Some(color_str) = color.as_str() {
            styles.push(format!("color: {}", color_str));
        }
    }

    if let Some(background) = attributes.get("background") {
        if let Some(bg_str) = background.as_str() {
            styles.push(format!("background-color: {}", bg_str));
        }
    }

    if let Some(font) = attributes.get("font") {
        if let Some(font_str) = font.as_str() {
            styles.push(format!("font-family: {}", font_str));
        }
    }

    if let Some(size) = attributes.get("size") {
        if let Some(size_str) = size.as_str() {
            styles.push(format!("font-size: {}", size_str));
        }
    }

    styles.join("; ")
}

/// Render Delta content as plain text (useful for accessibility)
pub fn render_delta_to_text(delta: &Delta) -> String {
    let mut text = String::new();

    for operation in delta.ops() {
        match operation {
            Op::Insert { insert, .. } => {
                text.push_str(insert);
            }
            Op::Retain { .. } | Op::Delete { .. } | Op::InsertEmbed { .. } | Op::RetainEmbed { .. } => {
                // These operations don't contribute to plain text representation
            }
        }
    }

    text
}

/// Extract document statistics from a Delta
pub fn get_delta_statistics(delta: &Delta) -> DocumentStatistics {
    let mut stats = DocumentStatistics::default();

    for operation in delta.ops() {
        match operation {
            Op::Insert { insert, .. } => {
                stats.character_count += insert.chars().count();
                stats.word_count += insert.split_whitespace().count();
                stats.line_count += insert.matches('\n').count();
            }
            Op::Retain { .. } | Op::Delete { .. } | Op::InsertEmbed { .. } | Op::RetainEmbed { .. } => {
                // These operations don't affect statistics in basic counting
            }
        }
    }

    // Ensure at least one line if there's any content
    if stats.character_count > 0 && stats.line_count == 0 {
        stats.line_count = 1;
    }

    stats
}

/// Document statistics extracted from Delta content
#[derive(Debug, Clone, Default)]
pub struct DocumentStatistics {
    pub character_count: usize,
    pub word_count: usize,
    pub line_count: usize,
}

/// Check if a Delta contains any formatted content
pub fn has_formatting(delta: &Delta) -> bool {
    delta.ops().iter().any(|op| {
        match op {
            Op::Insert { attributes: Some(attrs), .. } => !attrs.is_empty(),
            _ => false,
        }
    })
}

/// Extract all unique formatting attributes from a Delta
pub fn extract_formatting_attributes(delta: &Delta) -> HashMap<String, Vec<serde_json::Value>> {
    let mut attributes_map = HashMap::new();

    for operation in delta.ops() {
        if let Op::Insert { attributes: Some(attrs), .. } = operation {
            for (key, value) in attrs {
                attributes_map
                    .entry(key.clone())
                    .or_insert_with(Vec::new)
                    .push(value.clone());
            }
        }
    }

    // Remove duplicates and sort values
    for values in attributes_map.values_mut() {
        values.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        values.dedup();
    }

    attributes_map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty_delta() {
        let delta = Delta::new();
        let _element = render_delta_to_rsx(&delta);
        // Visual verification would be needed in a real test environment
    }

    #[test]
    fn test_render_plain_text() {
        let delta = Delta::new().insert("Hello, world!", None);
        let _element = render_delta_to_rsx(&delta);
        // Visual verification would be needed in a real test environment
    }

    #[test]
    fn test_render_bold_text() {
        let mut attributes = AttributeMap::new();
        attributes.insert("bold".to_string(), serde_json::Value::Bool(true));
        
        let delta = Delta::new().insert("Bold text", Some(attributes));
        let _element = render_delta_to_rsx(&delta);
        // Visual verification would be needed in a real test environment
    }

    #[test]
    fn test_delta_to_text() {
        let delta = Delta::new()
            .insert("Hello ", None)
            .insert("world", Some({
                let mut attrs = AttributeMap::new();
                attrs.insert("bold".to_string(), serde_json::Value::Bool(true));
                attrs
            }));

        let text = render_delta_to_text(&delta);
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn test_delta_statistics() {
        let delta = Delta::new()
            .insert("Hello world\nSecond line", None);

        let stats = get_delta_statistics(&delta);
        assert_eq!(stats.character_count, 18); // "Hello world\nSecond line"
        assert_eq!(stats.word_count, 3); // "Hello", "world", "Second", "line"
        assert_eq!(stats.line_count, 1); // One newline character
    }

    #[test]
    fn test_has_formatting() {
        let plain_delta = Delta::new().insert("Plain text", None);
        assert!(!has_formatting(&plain_delta));

        let mut attributes = AttributeMap::new();
        attributes.insert("bold".to_string(), serde_json::Value::Bool(true));
        let formatted_delta = Delta::new().insert("Bold text", Some(attributes));
        assert!(has_formatting(&formatted_delta));
    }

    #[test]
    fn test_extract_formatting_attributes() {
        let mut bold_attrs = AttributeMap::new();
        bold_attrs.insert("bold".to_string(), serde_json::Value::Bool(true));
        
        let mut color_attrs = AttributeMap::new();
        color_attrs.insert("color".to_string(), serde_json::Value::String("red".to_string()));
        
        let delta = Delta::new()
            .insert("Bold ", Some(bold_attrs))
            .insert("red text", Some(color_attrs));

        let attributes = extract_formatting_attributes(&delta);
        assert!(attributes.contains_key("bold"));
        assert!(attributes.contains_key("color"));
        assert_eq!(attributes["bold"].len(), 1);
        assert_eq!(attributes["color"].len(), 1);
    }

    #[test]
    fn test_build_inline_style() {
        let mut attributes = AttributeMap::new();
        attributes.insert("color".to_string(), serde_json::Value::String("red".to_string()));
        attributes.insert("background".to_string(), serde_json::Value::String("yellow".to_string()));
        
        let style = build_inline_style(&attributes);
        assert!(style.contains("color: red"));
        assert!(style.contains("background-color: yellow"));
    }
}