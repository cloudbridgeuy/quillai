//! Attributor system for managing text and block formatting
//!
//! The attributor module provides a flexible system for applying, managing, and
//! removing formatting attributes on DOM elements. Attributors handle different
//! types of formatting approaches including direct DOM attributes, CSS classes,
//! and inline styles.
//!
//! ## Attributor Types
//!
//! The system provides three main attributor implementations:
//!
//! - **Attributor**: Direct DOM attribute manipulation (e.g., `href`, `src`)
//! - **ClassAttributor**: CSS class-based formatting with prefix patterns
//! - **StyleAttributor**: Inline style-based formatting (e.g., `color`, `font-size`)
//!
//! ## Architecture
//!
//! ```text
//! AttributorTrait (base interface)
//! ├── Attributor (DOM attributes)
//! ├── ClassAttributor (CSS classes)
//! └── StyleAttributor (inline styles)
//! ```
//!
//! ## Common Use Cases
//!
//! ### Direct Attributes
//! ```rust,no_run
//! use quillai_parchment::attributor::{Attributor, AttributorOptions};
//! use quillai_parchment::registry::AttributorTrait;
//! use wasm_bindgen::JsValue;
//!
//! // Link href attribute
//! let options = AttributorOptions::default();
//! let link_attributor = Attributor::new("href", "href", options);
//! // link_attributor.add(&element, &JsValue::from_str("https://example.com"));
//! ```
//!
//! ### CSS Classes
//! ```rust,no_run
//! // Text alignment with CSS classes
//! // let align_attributor = ClassAttributor::new("align", "text-align", options);
//! // align_attributor.add(&element, &JsValue::from_str("center"));
//! // Results in: class="text-align-center"
//! ```
//!
//! ### Inline Styles
//! ```rust,no_run
//! // Text color with inline styles
//! // let color_attributor = StyleAttributor::new("color", "color", options);
//! // color_attributor.add(&element, &JsValue::from_str("#ff0000"));
//! // Results in: style="color: #ff0000"
//! ```
//!
//! ## Integration with Blots
//!
//! Attributors work seamlessly with the blot system to provide formatting
//! capabilities. They can be applied to any blot that supports attributes,
//! enabling rich text formatting and styling.
//!
//! ## Performance Considerations
//!
//! - **Efficient Updates**: Only modified attributes are updated
//! - **Batch Operations**: Multiple attributes can be applied together
//! - **DOM Optimization**: Minimal DOM manipulation for better performance
//! - **Memory Efficient**: Lightweight attribute management

/// Base attributor implementation for direct DOM attribute manipulation
pub mod base;
/// CSS class-based attributor for class-driven formatting
pub mod class;
/// Inline style attributor for direct style property management
pub mod style;

// Re-export attributors for convenient access
pub use base::*;
pub use class::*;
pub use style::*;
