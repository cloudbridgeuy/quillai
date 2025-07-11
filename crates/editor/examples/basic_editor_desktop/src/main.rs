use dioxus::prelude::*;
use dioxus_desktop::{Config, WindowBuilder};
use quillai_editor::component as quillai_editor_component;
use quillai_log::{LogLevel, LogConfig, init_logger, info};

fn main() {
    // Initialize logger for desktop console
    let log_config = LogConfig {
        format: quillai_log::LogFormat::Pretty,
        with_timestamp: true,
        with_target: true,
        with_thread_names: false,
        with_line_number: true,
        env_filter: None,
    };
    
    if let Err(e) = init_logger(LogLevel::Info, &log_config) {
        eprintln!("Failed to initialize logger: {}", e);
    }
    
    info!("Starting QuillAI Editor Desktop Example");

    // Configure the desktop window
    let config = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("QuillAI Editor - Desktop Example")
                .with_inner_size(dioxus_desktop::LogicalSize::new(1024.0, 768.0))
                .with_min_inner_size(dioxus_desktop::LogicalSize::new(800.0, 600.0))
        );

    // Launch the Dioxus desktop app
    LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}

fn App() -> Element {
    rsx! {
        div {
            class: "app-container",
            style: "height: 100vh; display: flex; flex-direction: column; background-color: #fafafa;",
            
            // Header/Toolbar area
            div {
                class: "app-toolbar",
                style: "background-color: #f5f5f5; padding: 0.75rem 1rem; border-bottom: 1px solid #ddd; flex-shrink: 0;",
                h1 {
                    style: "margin: 0; font-size: 1.25rem; color: #333; font-weight: 500;",
                    "QuillAI Editor - Desktop"
                }
            }
            
            // Main content area
            div {
                class: "app-content",
                style: "flex: 1; padding: 1.5rem; overflow-y: auto; display: flex; justify-content: center;",
                
                div {
                    class: "editor-container",
                    style: "width: 100%; max-width: 800px;",
                    
                    div {
                        class: "editor-wrapper",
                        style: "background-color: white; border: 1px solid #ddd; border-radius: 4px; padding: 1.5rem; min-height: 500px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                        
                        // The QuillAI editor component
                        quillai_editor_component {
                            initial_content: Some("Welcome to QuillAI Editor Desktop!\n\nThis is a desktop application demonstrating the QuillAI editor component. The editor works seamlessly across web and desktop platforms.\n\nTry these actions:\n• Edit this text\n• Check the console for change events\n• Notice the native desktop feel".to_string()),
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
                        class: "readonly-section",
                        style: "margin-top: 2rem;",
                        
                        h2 {
                            style: "margin-bottom: 0.75rem; font-size: 1.125rem; color: #333; font-weight: 500;",
                            "Readonly Mode Example"
                        }
                        
                        div {
                            class: "editor-wrapper",
                            style: "background-color: white; border: 1px solid #ddd; border-radius: 4px; padding: 1.5rem; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                            
                            quillai_editor_component {
                                initial_content: Some("This is a readonly editor instance.\n\nThe content cannot be modified, making it perfect for:\n• Displaying documentation\n• Showing code examples\n• Presenting formatted text\n• Review-only scenarios".to_string()),
                                readonly: Some(true),
                                class: Some("readonly-demo".to_string()),
                            }
                        }
                    }
                }
            }
            
            // Status bar
            div {
                class: "app-statusbar",
                style: "background-color: #f5f5f5; padding: 0.5rem 1rem; border-top: 1px solid #ddd; flex-shrink: 0;",
                p {
                    style: "margin: 0; color: #666; font-size: 0.875rem;",
                    "Ready"
                }
            }
        }
    }
}