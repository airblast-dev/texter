use std::{fmt::Debug, hash::Hash};

use crate::utils::string_ext::fast_char_iter;

#[derive(Clone, Debug)]
pub enum Change<P> {
    Delete { start: P, end: P },
    Insert { at: P, text: String },
    Replace { start: P, end: P, text: String },
}

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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteIndex(pub usize);

impl AsRawIndex for ByteIndex {
    fn as_raw_index(&self) -> usize {
        self.0
    }
}
impl ToByteIndex for ByteIndex {
    #[inline]
    fn to_byte_index(self, s: &str) -> usize {
        assert!(self.0 < s.len());
        self.0
    }

    #[inline]
    fn to_byte_index_exclusive(self, s: &str) -> usize {
        assert!(self.0 <= s.len());
        self.0
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NthChar(pub usize);

impl AsRawIndex for NthChar {
    fn as_raw_index(&self) -> usize {
        self.0
    }
}

impl ToByteIndex for NthChar {
    #[inline]
    fn to_byte_index(self, s: &str) -> usize {
        fast_char_iter(s)
            .nth(self.0)
            .expect("char index out of bounds")
    }

    #[inline]
    fn to_byte_index_exclusive(self, s: &str) -> usize {
        let mut char_count = 0;
        let nth_byte = fast_char_iter(s).inspect(|_| char_count += 1).nth(self.0);
        if let Some(nth_byte) = nth_byte {
            return nth_byte;
        }

        if char_count == self.0 {
            s.len()
        } else {
            panic!(
                "char index {} out of bounds, and {} > string.len(), {} chars found",
                self.0,
                self.0,
                char_count - 1
            )
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GridIndex<P> {
    pub row: usize,
    pub col: P,
}

impl<P> PartialEq for GridIndex<P>
where
    P: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row && self.col == other.col
    }
}

impl<P> Hash for GridIndex<P>
where
    P: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self.row);
        self.col.hash(state);
    }
}

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

#[cfg(test)]
mod tests {
    use super::GridIndex;
    use super::{NthChar, ToByteIndex};

    const SAMPLE: &str = "Hello, World!";
    const SAMPLE_MB: &str = "Secret Message: シュタインズ・ゲートは素晴らしいです。";

    mod nth_char {
        use super::*;

        mod to_byte_index {
            use super::*;

            #[test]
            fn single_byte() {
                let bi = NthChar(12).to_byte_index(SAMPLE);
                assert_eq!(bi, 12);
            }

            #[test]
            fn multi_byte() {
                let bi = NthChar(22).to_byte_index(SAMPLE_MB);
                assert_eq!(bi, 34);
            }

            #[test]
            #[should_panic]
            fn oob_char() {
                NthChar(13).to_byte_index(SAMPLE);
            }

            #[test]
            fn exclusive() {
                let bi = NthChar(13).to_byte_index_exclusive(SAMPLE);
                assert_eq!(bi, 13);
                assert_eq!(&SAMPLE[bi..], "");
            }

            #[test]
            fn to_byte_index_exclusive_multi_byte() {
                let bi = NthChar(35).to_byte_index_exclusive(SAMPLE_MB);
                assert_eq!(bi, 73);
                assert_eq!(&SAMPLE_MB[bi..], "");
            }

            #[test]
            #[should_panic]
            fn exclusive_oob_char() {
                NthChar(14).to_byte_index_exclusive(SAMPLE);
            }
        }
    }

    #[cfg(feature = "lsp-types")]
    mod lsp_types_ {
        use lsp_types::Position;

        use super::*;

        #[test]
        fn pos_to_grid() {
            let pos = Position {
                line: 10,
                character: 3,
            };

            let grid_index = GridIndex::<NthChar>::from(pos);

            assert_eq!(grid_index.row, 10);
            assert_eq!(grid_index.col.0, 3)
        }
    }
}
