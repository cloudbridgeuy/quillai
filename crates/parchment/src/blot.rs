//! Blot system for representing document content in the Parchment model
//!
//! The blot module contains all implementations of document content nodes in the Parchment
//! system. Blots are the fundamental building blocks that represent different types of
//! content (text, formatting, containers, embeds) in a structured document tree.
//!
//! ## Architecture Overview
//!
//! The blot system follows a hierarchical design based on content types and capabilities:
//!
//! ```text
//! BlotTrait (base)
//! ├── LeafBlotTrait (terminal content)
//! │   ├── TextBlot (text content)
//! │   └── EmbedBlot (images, videos, etc.)
//! └── ParentBlotTrait (containers)
//!     ├── InlineBlot (inline formatting)
//!     ├── BlockBlot (paragraphs, headers)
//!     └── ScrollBlot (root container)
//! ```
//!
//! ## Core Blot Types
//!
//! - **[TextBlot]**: Represents actual text content (leaf nodes)
//! - **[InlineBlot]**: Inline formatting containers (bold, italic, links)
//! - **[BlockBlot]**: Block-level containers (paragraphs, headers, lists)
//! - **[EmbedBlot]**: Self-contained embeds (images, videos, widgets)
//! - **[ScrollBlot]**: Root document container with scroll management
//!
//! ## Supporting Systems
//!
//! - **ParentBlot**: Base implementation for container blots
//! - **ShadowBlot**: Shadow DOM integration for complex blots
//! - **MutationObserverWrapper**: DOM change detection and synchronization
//! - **Traits**: Core interfaces defined in the traits_simple module
//!
//! ## Usage Patterns
//!
//! ```rust
//! use quillai_parchment::{BlotTrait, TextBlot, BlockBlot};
//! 
//! // Create a paragraph containing text
//! let mut paragraph = BlockBlot::new("p", None);
//! let text = TextBlot::new("Hello, world!");
//! paragraph.append_child(Box::new(text))?;
//! 
//! // All blots implement the common interface
//! fn process_blot(blot: &dyn BlotTrait) {
//!     println!("Processing {} with length {}", 
//!              blot.get_blot_name(), blot.length());
//! }
//! ```

/// Block-level blot implementations (paragraphs, headers, lists)
pub mod block;
/// Embed blot implementations (images, videos, widgets)
pub mod embed;
/// Inline blot implementations (formatting, links)
pub mod inline;
/// Mutation detection and DOM synchronization
pub mod mutations;
/// Parent blot base implementation for containers
pub mod parent;
/// Root scroll container implementation
pub mod scroll;
/// Shadow DOM integration for complex blots
pub mod shadow_simple;
/// Text blot implementation for actual content
pub mod text;
/// Core trait definitions for the blot system
pub mod traits_simple;

// Re-export key types for convenient access
pub use mutations::{MutationObserverWrapper, OptimizeContext, UpdateContext};
pub use parent::ParentBlot;
pub use shadow_simple::ShadowBlot;
pub use traits_simple::*;
