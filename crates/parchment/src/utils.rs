//! Utility functions and helper modules for Parchment operations.
//!
//! This module provides a collection of utility functions and helper modules
//! that support various operations throughout the Parchment library. The utilities
//! are organized into specialized sub-modules for different domains.
//!
//! # Module Organization
//!
//! - **DOM utilities**: Browser DOM manipulation helpers for WebAssembly
//! - **General utilities**: Common helper functions used across the codebase
//!
//! # Usage
//!
//! The utilities are designed to be lightweight, focused helpers that provide
//! common functionality without adding significant complexity or dependencies.
//! They follow Parchment's philosophy of minimal dependencies and efficient
//! WebAssembly integration.
//!
//! # Examples
//!
//! ```rust
//! use parchment::utils::*;
//!
//! // DOM utilities are re-exported for convenience
//! let window = window()?;
//! let document = document()?;
//! let element = create_element("div")?;
//! ```
//!
//! # Design Principles
//!
//! - **Minimal overhead**: Thin wrappers over existing APIs
//! - **Error handling**: Proper error propagation and handling
//! - **WebAssembly optimized**: Designed for efficient WASM execution
//! - **Reusability**: Common patterns extracted into reusable functions

pub mod dom;

pub use dom::*;
