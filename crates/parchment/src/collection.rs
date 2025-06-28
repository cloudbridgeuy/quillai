//! Collection data structures for efficient blot management.
//!
//! This module provides specialized collection types optimized for Parchment's
//! document model operations. The primary collection is a doubly-linked list
//! that enables efficient insertion, deletion, and traversal of blots within
//! the document tree.
//!
//! # Architecture
//!
//! The collection system is designed to support:
//! - **Efficient insertion/deletion**: O(1) operations at any position
//! - **Memory safety**: Safe abstractions over raw pointer operations
//! - **Iterator support**: Standard Rust iteration patterns
//! - **WASM compatibility**: Optimized for WebAssembly environments
//!
//! # Usage
//!
//! ```rust
//! use parchment::collection::LinkedList;
//!
//! let mut list = LinkedList::new();
//! list.push("first");
//! list.push("second");
//! list.insert(1, "middle");
//!
//! assert_eq!(list.length, 3);
//! assert_eq!(list.get(1), Some(&"middle"));
//! ```
//!
//! # Performance Characteristics
//!
//! | Operation | Time Complexity | Notes |
//! |-----------|----------------|-------|
//! | Insert at head/tail | O(1) | Direct pointer manipulation |
//! | Insert at index | O(n) | Requires traversal to position |
//! | Delete at head/tail | O(1) | Direct pointer manipulation |
//! | Delete at index | O(n) | Requires traversal to position |
//! | Search | O(n) | Linear traversal |
//! | Access by index | O(n) | Linear traversal |
//!
//! # Memory Management
//!
//! The linked list uses `Box<Node<T>>` for heap allocation and `NonNull<Node<T>>`
//! for efficient pointer operations while maintaining memory safety through
//! Rust's ownership system.

pub mod linked_list;

pub use linked_list::*;
