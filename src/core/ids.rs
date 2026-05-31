use std::fmt;

/// Stable identifier for a render block within a document or stream.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub u64);

impl BlockId {
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

impl fmt::Debug for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockId({})", self.0)
    }
}
