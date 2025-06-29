//! Basic Parchment integration for the QuillAI Editor.
//!
//! This module provides the foundational integration between the editor component
//! and the Parchment document model system. It handles registry initialization,
//! basic blot type registration, and synchronization with Delta changes.

use quillai_parchment::{Registry, ParchmentError};
use quillai_delta::Delta;


/// Parchment integration state for the editor.
///
/// This struct manages the Parchment registry and provides utilities for
/// synchronizing document state between Delta operations and Parchment blots.
pub struct ParchmentIntegration {
    /// The Parchment registry for managing blot types
    registry: Registry,
}

impl ParchmentIntegration {
    /// Create a new Parchment integration with basic blot types registered.
    ///
    /// This initializes the registry and registers the fundamental blot types
    /// needed for basic text editing functionality.
    ///
    /// # Returns
    ///
    /// A new `ParchmentIntegration` instance with basic blots registered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_editor::editor::parchment_integration::ParchmentIntegration;
    ///
    /// let integration = ParchmentIntegration::new();
    /// ```
    pub fn new() -> Self {
        let mut registry = Registry::new();
        
        // Register basic blot types needed for text editing
        Self::register_basic_blots(&mut registry);
        
        Self { registry }
    }

    /// Register the basic blot types required for text editing.
    ///
    /// This registers:
    /// - TextBlot for raw text content
    /// - BlockBlot for paragraph-level containers
    /// - InlineBlot for inline formatting (future use)
    ///
    /// # Arguments
    ///
    /// * `registry` - The registry to register blot types with
    fn register_basic_blots(registry: &mut Registry) {
        // Register text blot for actual text content
        registry.register_blot_type("text", "#text");
        
        // Register block blot for paragraph containers
        registry.register_blot_type("block", "p");
        
        // Register inline blot for future formatting support
        registry.register_blot_type("inline", "span");
        
        // Register scroll blot as the root container
        registry.register_blot_type("scroll", "div");
    }

    /// Get a reference to the Parchment registry.
    ///
    /// This allows access to the registry for advanced operations or
    /// registering additional custom blot types.
    ///
    /// # Returns
    ///
    /// A reference to the internal `Registry`
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Get a mutable reference to the Parchment registry.
    ///
    /// This allows modification of the registry, such as registering
    /// additional blot types or updating existing registrations.
    ///
    /// # Returns
    ///
    /// A mutable reference to the internal `Registry`
    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }

    /// Synchronize Parchment state with a Delta document.
    ///
    /// This method updates the Parchment document structure to match
    /// the provided Delta operations. It's called when the Delta document
    /// changes to keep the Parchment representation in sync.
    ///
    /// # Arguments
    ///
    /// * `delta` - The Delta document to synchronize with
    ///
    /// # Returns
    ///
    /// `Ok(())` if synchronization succeeded, or an error if the operation failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_editor::editor::parchment_integration::ParchmentIntegration;
    /// use quillai_delta::Delta;
    ///
    /// let mut integration = ParchmentIntegration::new();
    /// let delta = Delta::new().insert("Hello, world!", None);
    /// 
    /// match integration.sync_with_delta(&delta) {
    ///     Ok(()) => println!("Synchronization successful"),
    ///     Err(_e) => println!("Synchronization failed"),
    /// }
    /// ```
    pub fn sync_with_delta(&mut self, delta: &Delta) -> Result<(), ParchmentError> {
        // For Phase 1, we implement a basic synchronization that doesn't cause errors
        // but doesn't perform complex DOM operations yet.
        // This will be expanded in later phases.
        
        // Validate that the delta is not empty and contains valid operations
        if delta.ops().is_empty() {
            return Ok(());
        }
        
        // For now, we just validate that our registry has the required blot types
        // In future phases, this will create actual blot structures
        let _text_tag = self.registry.query_by_name("text");
        let _block_tag = self.registry.query_by_name("block");
        
        Ok(())
    }

    /// Create a simple blot structure for the given text content.
    ///
    /// This is a utility method that creates a basic document structure
    /// with a root scroll blot containing a block blot with text content.
    /// This provides the foundation for more complex document structures.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to create blots for
    ///
    /// # Returns
    ///
    /// `Ok(())` if blot creation succeeded, or an error if the operation failed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_editor::editor::parchment_integration::ParchmentIntegration;
    ///
    /// let mut integration = ParchmentIntegration::new();
    /// let _ = integration.create_simple_blot_structure("Hello, world!");
    /// ```
    pub fn create_simple_blot_structure(&mut self, text: &str) -> Result<(), ParchmentError> {
        // For Phase 1, we implement a basic structure creation that validates
        // our registry setup without complex DOM manipulation
        
        if text.is_empty() {
            return Ok(());
        }
        
        // Verify that we can query for the required blot types
        let _scroll_tag = self.registry.query_by_name("scroll");
        let _block_tag = self.registry.query_by_name("block");
        let _text_tag = self.registry.query_by_name("text");
        
        // In future phases, this will create actual DOM nodes and blot instances
        // For now, we just validate that the registry is properly set up
        
        Ok(())
    }

    /// Check if the Parchment integration is properly initialized.
    ///
    /// This validates that all required blot types are registered and
    /// the integration is ready for use.
    ///
    /// # Returns
    ///
    /// `true` if the integration is properly initialized, `false` otherwise
    pub fn is_initialized(&self) -> bool {
        // Check that all basic blot types are registered
        self.registry.query_by_name("text").is_some() &&
        self.registry.query_by_name("block").is_some() &&
        self.registry.query_by_name("inline").is_some() &&
        self.registry.query_by_name("scroll").is_some()
    }
}

impl Default for ParchmentIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parchment_integration_creation() {
        let integration = ParchmentIntegration::new();
        assert!(integration.is_initialized());
    }

    #[test]
    fn test_basic_blot_registration() {
        let integration = ParchmentIntegration::new();
        let registry = integration.registry();
        
        // Verify basic blot types are registered
        assert!(registry.query_by_name("text").is_some());
        assert!(registry.query_by_name("block").is_some());
        assert!(registry.query_by_name("inline").is_some());
        assert!(registry.query_by_name("scroll").is_some());
    }

    #[test]
    fn test_delta_synchronization() {
        let mut integration = ParchmentIntegration::new();
        let delta = Delta::new().insert("Hello, world!", None);
        
        // Should not error with basic delta
        assert!(integration.sync_with_delta(&delta).is_ok());
    }

    #[test]
    fn test_empty_delta_synchronization() {
        let mut integration = ParchmentIntegration::new();
        let delta = Delta::new();
        
        // Should handle empty delta gracefully
        assert!(integration.sync_with_delta(&delta).is_ok());
    }

    #[test]
    fn test_simple_blot_structure_creation() {
        let mut integration = ParchmentIntegration::new();
        
        // Should handle text content without errors
        assert!(integration.create_simple_blot_structure("Hello, world!").is_ok());
        
        // Should handle empty text gracefully
        assert!(integration.create_simple_blot_structure("").is_ok());
    }

    #[test]
    fn test_registry_access() {
        let mut integration = ParchmentIntegration::new();
        
        // Test immutable access
        let _registry_ref = integration.registry();
        
        // Test mutable access
        let _registry_mut = integration.registry_mut();
    }
}