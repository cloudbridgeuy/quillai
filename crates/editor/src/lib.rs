#![doc(test(attr(ignore)))]

//! # QuillAI Editor
//!
//! A modern rich text editor component for Dioxus applications.
//!
//! QuillAI Editor is a reusable Dioxus component that provides rich text editing
//! capabilities with keyboard-driven formatting, built on top of the Delta format
//! for document representation and Parchment for DOM state management.
//!
//! ## Quick Start
//!
//! ```rust
//! use dioxus::prelude::*;
//! use quillai_editor::QuillAIEditor;
//!
//! fn App() -> Element {
//!     rsx! {
//!         QuillAIEditor {
//!             placeholder: "Start typing...",
//!         }
//!     }
//! }
//! ```
//!
//! ## Features
//!
//! - **Keyboard-driven formatting**: Bold, italic, strikethrough, code, and highlight
//! - **Delta integration**: JSON-based document representation
//! - **Parchment integration**: Advanced DOM state management
//! - **Dioxus native**: Built specifically for Dioxus applications
//! - **Reusable component**: Easy integration into existing projects

pub mod editor;
pub mod modules;
pub mod utils;

// Re-export the main component and types for easy access
pub use editor::component::{QuillAIEditor, QuillAIEditorProps};
pub use editor::delta_integration;
pub use editor::parchment_integration;