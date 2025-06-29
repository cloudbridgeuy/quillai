//! Example demonstrating Delta integration with the QuillAI Editor.
//!
//! This example shows:
//! - Initial content loading
//! - Delta change callbacks
//! - Document state management
//!
//! Run with: cargo run --example delta_integration

use dioxus::prelude::*;
use quillai_editor::QuillAIEditor;

fn main() {
    dioxus::launch(App);
}

fn App() -> Element {
    // State to track document changes
    let mut change_count = use_signal(|| 0);
    let mut last_delta = use_signal(|| String::from("No changes yet"));
    
    rsx! {
        div {
            style: "font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px;",
            
            h1 { "QuillAI Editor - Delta Integration Demo" }
            
            p { 
                "This example demonstrates Delta integration. The editor starts with initial content "
                "and shows how Delta operations are generated as you type."
            }
            
            // Editor with initial content
            div {
                style: "border: 1px solid #ccc; border-radius: 4px; margin: 20px 0;",
                QuillAIEditor {
                    initial_content: Some("Welcome to QuillAI! Start editing this text to see Delta operations in action.".to_string()),
                    placeholder: Some("Type something here...".to_string()),
                    class: Some("demo-editor".to_string()),
                    on_change: move |delta_json: String| {
                        change_count.set(change_count() + 1);
                        last_delta.set(delta_json);
                        println!("Document changed ({}): {}", change_count(), last_delta());
                    }
                }
            }
            
            // Debug information
            div {
                style: "background: #f5f5f5; padding: 15px; border-radius: 4px; margin-top: 20px;",
                
                h3 { "Debug Information" }
                
                p { 
                    strong { "Changes: " }
                    "{change_count()}"
                }
                
                p { 
                    strong { "Last Delta JSON: " }
                }
                
                pre {
                    style: "background: white; padding: 10px; border: 1px solid #ddd; border-radius: 4px; overflow-x: auto; font-size: 12px;",
                    "{last_delta()}"
                }
                
                p {
                    style: "font-size: 12px; color: #666; margin-top: 10px;",
                    "💡 Try typing, deleting, or pasting text to see how Delta operations are generated."
                }
            }
        }
    }
}