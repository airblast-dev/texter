use std::fmt::Debug;

#[derive(Clone, Debug)]
pub enum Change<P: Position> {
    Delete { start: P, end: P },
    Insert { at: P, text: String },
    Replace { start: P, end: P, text: String },
}

pub trait Position: Clone + Copy + Debug {}
pub trait PositionItem: Clone + Copy + Debug {}
pub trait AsRawIndex {
    /// The raw internal index.
    ///
    /// The exact requirments of this can vary depending where the value will be used in.
    fn as_raw_index(&self) -> usize;
}
/// Transformer from type to ByteIndex.
pub trait ToByteIndex: AsRawIndex {
    /// Transforms a type of PositionItem to a different one.
    fn to_byte_index(self, s: &str) -> usize;
    /// Transforms a type of PositionItem to a different one, but allowing off by one indexes to be
    /// used in contexts of exclusive range ends.
    fn to_byte_index_exclusive(self, s: &str) -> usize;
}

#[derive(Clone, Copy, Debug, Hash)]
pub struct ByteIndex(pub usize);

impl Position for ByteIndex {}
impl PositionItem for ByteIndex {}
impl AsRawIndex for ByteIndex {
    fn as_raw_index(&self) -> usize {
        self.0
    }
}
impl ToByteIndex for ByteIndex {
    fn to_byte_index(self, _: &str) -> usize {
        self.0
    }

    fn to_byte_index_exclusive(self, _: &str) -> usize {
        self.0 + 1
    }
}

#[derive(Clone, Copy, Debug, Hash)]
pub struct NthChar(pub usize);

impl Position for NthChar {}
impl PositionItem for NthChar {}
impl AsRawIndex for NthChar {
    fn as_raw_index(&self) -> usize {
        self.0
    }
}
impl ToByteIndex for NthChar {
    fn to_byte_index(self, s: &str) -> usize {
        s.as_bytes()
            .iter()
            .enumerate()
            .filter(|(_, ci)| (**ci as i8) >= -0x40)
            .map(|(bi, _)| bi)
            .nth(self.0)
            .expect("out of bounds char index")
    }

    fn to_byte_index_exclusive(self, s: &str) -> usize {
        if self.0 == s.len() {
            self.0
        } else {
            self.to_byte_index(s)
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
pub struct GridIndex<P> {
    pub row: usize,
    pub col: P,
}

impl<P: ToByteIndex + PositionItem> Position for GridIndex<P> {}
#[cfg(feature = "lsp-types")]
mod lsp_types {
    use lsp_types::Position;

    use super::{GridIndex, NthChar};

    impl From<Position> for GridIndex<NthChar> {
        fn from(value: Position) -> Self {
            Self {
                row: value.line as usize,
                col: NthChar(value.character as usize),
            }
        }
    }
}
