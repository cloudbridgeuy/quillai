use dioxus::prelude::*;
use quillai_editor::component as quillai_editor_component;
use quillai_log::{LogLevel, LogConfig, init_logger, info};

fn main() {
    // Initialize logger for web console
    let log_config = LogConfig {
        format: quillai_log::LogFormat::Compact,
        with_timestamp: false, // Timestamps are less useful in browser console
        with_target: true,
        with_thread_names: false,
        with_line_number: true,
        env_filter: None,
    };
    
    if let Err(e) = init_logger(LogLevel::Info, &log_config) {
        eprintln!("Failed to initialize logger: {}", e);
    }
    
    info!("Starting QuillAI Editor Web Example");

    // Launch the Dioxus web app
    launch(App);
}

fn App() -> Element {
    rsx! {
        div {
            class: "app-container",
            style: "min-height: 100vh; display: flex; flex-direction: column;",
            
            // Header
            header {
                class: "app-header",
                style: "background-color: #f5f5f5; padding: 1rem; border-bottom: 1px solid #ddd;",
                h1 {
                    style: "margin: 0; font-size: 1.5rem; color: #333;",
                    "QuillAI Editor - Web Example"
                }
            }
            
            // Main content area
            main {
                class: "app-main",
                style: "flex: 1; padding: 2rem; max-width: 800px; margin: 0 auto; width: 100%;",
                
                div {
                    class: "editor-wrapper",
                    style: "border: 1px solid #ddd; border-radius: 4px; padding: 1rem; min-height: 400px; background-color: white;",
                    
                    // The QuillAI editor component
                    quillai_editor_component {
                        initial_content: Some("Welcome to QuillAI Editor!\n\nThis is a web-based rich text editor built with Dioxus and Rust. Try editing this text to see how it works.\n\nFeatures:\n- Type to edit text\n- Real-time change events\n- Cross-platform compatibility".to_string()),
                        on_change: move |delta_json: String| {
                            info!("Document updated: {}", delta_json);
                        },
                        on_selection_change: move |(start, end): (usize, usize)| {
                            info!("Selection: {} to {}", start, end);
                        }
                    }
                }
                
                // Readonly example
                div {
                    class: "editor-section",
                    style: "margin-top: 2rem;",
                    
                    h2 {
                        style: "margin-bottom: 1rem; font-size: 1.25rem; color: #333;",
                        "Readonly Example"
                    }
                    
                    div {
                        class: "editor-wrapper",
                        style: "border: 1px solid #ddd; border-radius: 4px; padding: 1rem; background-color: white;",
                        
                        quillai_editor_component {
                            initial_content: Some("This editor is in readonly mode.\n\nYou cannot edit this text, but you can select and copy it. This is useful for displaying content that users should not modify.".to_string()),
                            readonly: Some(true),
                            class: Some("readonly-editor".to_string()),
                        }
                    }
                }
            }
            
            // Footer
            footer {
                class: "app-footer",
                style: "background-color: #f5f5f5; padding: 1rem; border-top: 1px solid #ddd; text-align: center;",
                p {
                    style: "margin: 0; color: #666; font-size: 0.875rem;",
                    "QuillAI Editor Example - Built with Dioxus"
                }
            }
        }
    }
}