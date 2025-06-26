use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Scope {
    // TYPE = (1 << 2) - 1 = 0011 (Lower two bits)
    Type = 0b0011,
    // LEVEL = ((1 << 2) - 1) << 2 = 1100 (Higher two bits)
    Level = 0b1100,

    // ATTRIBUTE = (1 << 0) | LEVEL = 1101
    Attribute = 0b1101,
    // BLOT = (1 << 1) | LEVEL = 1110
    Blot = 0b1110,
    // INLINE = (1 << 2) | TYPE = 0111
    Inline = 0b0111,
    // BLOCK = (1 << 3) | TYPE = 1011
    Block = 0b1011,

    // BLOCK_BLOT = BLOCK & BLOT = 1010
    BlockBlot = 0b1010,
    // INLINE_BLOT = INLINE & BLOT = 0110
    InlineBlot = 0b0110,
    // EMBED_BLOT (leaf node, similar to inline but self-contained) = 0100
    EmbedBlot = 0b0100,
    // BLOCK_ATTRIBUTE = BLOCK & ATTRIBUTE = 1001
    BlockAttribute = 0b1001,
    // INLINE_ATTRIBUTE = INLINE & ATTRIBUTE = 0101
    InlineAttribute = 0b0101,

    // ANY = TYPE | LEVEL = 1111
    #[default]
    Any = 0b1111,
}

impl Scope {
    pub fn has_type(&self, other: Scope) -> bool {
        (*self as u8) & (Scope::Type as u8) & (other as u8) != 0
    }

    pub fn has_level(&self, other: Scope) -> bool {
        (*self as u8) & (Scope::Level as u8) & (other as u8) != 0
    }

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
