//!
//! The Scope system provides a hierarchical categorization mechanism for different types
//! of blots and attributors in the Parchment document model. It uses bitwise operations
//! to efficiently determine compatibility and relationships between different content types.
//!
//! ## Bitwise Design
//!
//! The scope system uses a 4-bit encoding where:
//! - Lower 2 bits represent TYPE (inline vs block content)
//! - Upper 2 bits represent LEVEL (blot vs attribute)
//!
//! This allows for efficient bitwise operations to determine scope compatibility
//! and hierarchical relationships.

use wasm_bindgen::prelude::*;

/// Scope enumeration for categorizing blots and attributors using bitwise operations
///
/// The Scope enum uses a carefully designed bit pattern system to enable efficient
/// categorization and matching of different document elements. Each scope value
/// encodes both the type (inline/block) and level (blot/attribute) information.
///
/// # Bit Pattern Design
///
/// ```text
/// Bit positions: 3 2 1 0
///               L L T T
///
/// Where:
/// - TT (bits 0-1): Type information (inline vs block)
/// - LL (bits 2-3): Level information (blot vs attribute)
/// ```
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::Scope;
///
/// // Check if a scope matches another
/// assert!(Scope::BlockBlot.matches(Scope::Block));
/// assert!(Scope::InlineBlot.matches(Scope::Inline));
///
/// // Check type compatibility
/// assert!(Scope::BlockBlot.has_type(Scope::Block));
/// assert!(!Scope::BlockBlot.has_type(Scope::Inline));
/// ```
#[wasm_bindgen]
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Scope {
    /// Base type mask - covers lower two bits (0011)
    /// Used for type-based operations and matching
    Type = 0b0011,

    /// Base level mask - covers upper two bits (1100)
    /// Used for level-based operations and matching
    Level = 0b1100,

    /// Attribute level scope (1101)
    /// Represents formatting attributes that modify content appearance
    Attribute = 0b1101,

    /// Blot level scope (1110)
    /// Represents content blots that contain actual document data
    Blot = 0b1110,

    /// Inline type scope (0111)
    /// Represents inline content that flows within text
    Inline = 0b0111,

    /// Block type scope (1011)
    /// Represents block-level content that creates new lines/paragraphs
    Block = 0b1011,

    /// Block blot scope (1010) - intersection of Block and Blot
    /// Represents block-level content blots like paragraphs, headers
    BlockBlot = 0b1010,

    /// Inline blot scope (0110) - intersection of Inline and Blot
    /// Represents inline content blots like formatted text spans
    InlineBlot = 0b0110,

    /// Embed blot scope (0100)
    /// Represents self-contained leaf nodes like images, videos
    EmbedBlot = 0b0100,

    /// Block attribute scope (1001) - intersection of Block and Attribute
    /// Represents block-level formatting attributes
    BlockAttribute = 0b1001,

    /// Inline attribute scope (0101) - intersection of Inline and Attribute
    /// Represents inline formatting attributes like bold, italic
    InlineAttribute = 0b0101,

    /// Universal scope (1111) - matches any other scope
    /// Default scope used when no specific categorization is needed
    #[default]
    Any = 0b1111,
}

impl Scope {
    /// Check if this scope has the same type as another scope
    ///
    /// Performs a bitwise AND operation on the type bits (lower 2 bits) to determine
    /// if two scopes share the same type classification (inline vs block).
    ///
    /// # Parameters
    /// * `other` - The scope to compare type compatibility with
    ///
    /// # Returns
    /// `true` if the scopes share type bits, `false` otherwise
    ///
    /// # Examples
    /// ```rust
    /// use quillai_parchment::Scope;
    ///
    /// assert!(Scope::BlockBlot.has_type(Scope::Block));
    /// assert!(Scope::InlineBlot.has_type(Scope::Inline));
    /// assert!(!Scope::BlockBlot.has_type(Scope::Inline));
    /// ```
    pub fn has_type(&self, other: Scope) -> bool {
        (*self as u8) & (Scope::Type as u8) & (other as u8) != 0
    }

    /// Check if this scope has the same level as another scope
    ///
    /// Performs a bitwise AND operation on the level bits (upper 2 bits) to determine
    /// if two scopes share the same level classification (blot vs attribute).
    ///
    /// # Parameters
    /// * `other` - The scope to compare level compatibility with
    ///
    /// # Returns
    /// `true` if the scopes share level bits, `false` otherwise
    ///
    /// # Examples
    /// ```rust
    /// use quillai_parchment::Scope;
    ///
    /// assert!(Scope::BlockBlot.has_level(Scope::Blot));
    /// assert!(Scope::InlineAttribute.has_level(Scope::Attribute));
    /// assert!(!Scope::BlockBlot.has_level(Scope::Attribute));
    /// ```
    pub fn has_level(&self, other: Scope) -> bool {
        (*self as u8) & (Scope::Level as u8) & (other as u8) != 0
    }

    /// Check if this scope fully matches another scope
    ///
    /// A scope matches another if it shares both the same type and level bits.
    /// This is the primary method for determining scope compatibility in the
    /// blot system.
    ///
    /// # Parameters
    /// * `scope` - The scope to check for full compatibility
    ///
    /// # Returns
    /// `true` if both type and level match, `false` otherwise
    ///
    /// # Examples
    /// ```rust
    /// use quillai_parchment::Scope;
    ///
    /// assert!(Scope::BlockBlot.matches(Scope::Block));
    /// assert!(Scope::InlineBlot.matches(Scope::Inline));
    /// assert!(!Scope::BlockBlot.matches(Scope::Inline));
    /// assert!(Scope::Any.matches(Scope::Block)); // Any matches everything
    /// ```
    pub fn matches(&self, scope: Scope) -> bool {
        self.has_level(scope) && self.has_type(scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_bitwise_operations() {
        assert_eq!(Scope::BlockBlot as u8, 0b1010);
        assert_eq!(Scope::InlineBlot as u8, 0b0110);
        assert_eq!(Scope::BlockAttribute as u8, 0b1001);
        assert_eq!(Scope::InlineAttribute as u8, 0b0101);
    }

    #[test]
    fn test_scope_matching() {
        assert!(Scope::BlockBlot.matches(Scope::Block));
        assert!(Scope::InlineBlot.matches(Scope::Inline));
        assert!(!Scope::BlockBlot.matches(Scope::Inline));
        assert!(!Scope::InlineBlot.matches(Scope::Block));
    }
}
