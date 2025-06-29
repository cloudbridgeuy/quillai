//! Minimal setup example for QuillAI Editor
//!
//! This example demonstrates the simplest possible usage of the QuillAI Editor component.

use dioxus::prelude::*;
use quillai_editor::QuillAIEditor;

fn main() {
    // Launch the Dioxus app
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            style: "padding: 20px; font-family: Arial, sans-serif;",
            h1 { "QuillAI Editor - Minimal Setup" }
            p { "This is a minimal example of the QuillAI Editor component." }
            
            // Basic editor with minimal configuration
            QuillAIEditor {
                placeholder: "Start typing...",
            }
        }
    }
}