use wasm_bindgen::prelude::*;

pub mod attributor;
pub mod blot;
pub mod collection;
pub mod dom;
pub mod registry;
pub mod scope;
pub mod text_operations;
pub mod utils;

// Re-exports for public API
pub use blot::block::BlockBlot;
pub use blot::embed::EmbedBlot;
pub use blot::inline::InlineBlot;
pub use blot::scroll::ScrollBlot;
pub use blot::text::TextBlot;
pub use blot::traits_simple::*;
pub use registry::Registry;
pub use scope::Scope;
pub use text_operations::*;
pub use utils::*;

// This is like the `extern` block in C.
#[wasm_bindgen]
extern "C" {
    // Bind the `console.log` function from the browser.
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Get version information
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
