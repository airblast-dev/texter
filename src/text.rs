use std::{
    fmt::{Debug, Display},
    sync::LazyLock,
};

use memchr::memmem::Finder;

use crate::change::{Change, GridIndex, ToByteIndex};

#[derive(Clone, Default)]
pub struct Text {
    br_indexes: BrIndexes,
    text: String,
}

impl Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Breaklines: {:?}\n{:?}", self.br_indexes, self.text)
    }
}

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

static BR_FINDER: LazyLock<Finder> = LazyLock::new(|| Finder::new("\n"));

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BrIndexes(Vec<usize>);

impl<S: AsRef<[usize]>> PartialEq<S> for BrIndexes {
    fn eq(&self, other: &S) -> bool {
        self.0 == other.as_ref()
    }
}

impl BrIndexes {
    fn new(s: &str) -> Self {
        let iter = BR_FINDER.find_iter(s.as_bytes());
        let mut byte_indexes = vec![0];
        byte_indexes.extend(iter);
        Self(byte_indexes)
    }

    // The index to the first byte in the row.
    fn row_start(&self, row: usize) -> usize {
        // we increment by one if it is not zero since the index points to a break line,
        // and the first row should start at zero.
        self.0[row] + (row != 0) as usize
    }

    fn remove_indexes(&mut self, start: usize, end: usize) {
        let start = if start != end { start + 1 } else { return };
        self.0.drain(start..end);
    }

    /// Add an offset to all rows after the provided row number including itself.
    fn add_offsets(&mut self, row: usize, by: usize) {
        self.0[row.max(1)..].iter_mut().for_each(|bi| *bi += by);
    }

    /// Sub an offset to all rows after the provided row number including itself.
    fn sub_offsets(&mut self, row: usize, by: usize) {
        self.0[row.max(1)..].iter_mut().for_each(|bi| *bi -= by);
    }
}

impl Text {
    pub fn new(text: String) -> Self {
        let br_indexes = BrIndexes::new(&text);
        Self { text, br_indexes }
    }

    pub fn update<B: ToByteIndex + Copy>(&mut self, change: Change<GridIndex<B>>) {
        match change {
            Change::Delete { start, end } => {
                let (start_index, end_index) = {
                    let rs = self.br_indexes.row_start(start.row);
                    let re = self.br_indexes.row_start(end.row);
                    let start_s = &self.text[rs..];
                    let end_s = &self.text[re..];
                    (
                        rs + start.col.to_byte_index(start_s),
                        re + end.col.to_byte_index_exclusive(end_s),
                    )
                };

                // if end.col.as_raw_index() == 0 it means we are removing the trailing break line so we should also remove
                // it from the break line indexes.
                self.br_indexes.remove_indexes(
                    start.row,
                    end.row + ((end.col.as_raw_index() == 0) as usize),
                );
                self.br_indexes
                    .sub_offsets(start.row + 1, end_index - start_index);

                self.text.drain(start_index..end_index);
            }
            Change::Insert { at, text } => {
                let br_indexes = BR_FINDER.find_iter(text.as_bytes());
                let start = self.br_indexes.row_start(at.row);
                let start = &self.text[start..];
                let insertion_index = at.col.to_byte_index_exclusive(start);
                self.br_indexes.add_offsets(at.row, text.len());
                self.br_indexes.0.splice(
                    at.row + 1..at.row + 1,
                    br_indexes.map(|i| i + insertion_index),
                );
                self.text.insert_str(insertion_index, &text);
            }
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::change::{Change, GridIndex, NthChar};

    use super::Text;

    #[test]
    fn delete_multiline() {
        let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
        t.update(crate::change::Change::Delete {
            start: GridIndex {
                row: 1,
                col: NthChar(3),
            },
            end: GridIndex {
                row: 3,
                col: NthChar(2),
            },
        });

        assert_eq!(t.text, "Hello, World!\nAppars");
    }

    #[test]
    fn delete_in_line() {
        let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
        assert_eq!(t.br_indexes, [0, 13, 20, 29]);
        t.update(crate::change::Change::Delete {
            start: GridIndex {
                row: 0,
                col: NthChar(3),
            },
            end: GridIndex {
                row: 0,
                col: NthChar(5),
            },
        });

        assert_eq!(t.br_indexes, [0, 11, 18, 27]);
        assert_eq!(t.text, "Hel, World!\nApples\n Oranges\nPears");
    }

    #[test]
    fn delete_from_start() {
        let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
        assert_eq!(t.br_indexes, [0, 13, 20, 29]);
        t.update(crate::change::Change::Delete {
            start: GridIndex {
                row: 0,
                col: NthChar(0),
            },
            end: GridIndex {
                row: 0,
                col: NthChar(5),
            },
        });

        assert_eq!(t.br_indexes, [0, 8, 15, 24]);
        assert_eq!(t.text, ", World!\nApples\n Oranges\nPears");
    }

    #[test]
    fn delete_from_end() {
        let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
        t.update(crate::change::Change::Delete {
            start: GridIndex {
                row: 3,
                col: NthChar(0),
            },
            end: GridIndex {
                row: 3,
                col: NthChar(5),
            },
        });

        assert_eq!(t.br_indexes, [0, 13, 20, 29]);
        assert_eq!(t.text, "Hello, World!\nApples\n Oranges\n");
    }

    #[test]
    fn delete_br() {
        let mut t = Text::new("Hello, World!\nBadApple\n".to_string());
        assert_eq!(t.br_indexes, [0, 13, 22]);
        t.update(Change::Delete {
            start: GridIndex {
                row: 1,
                col: NthChar(8),
            },
            end: GridIndex {
                row: 2,
                col: NthChar(0),
            },
        });

        assert_eq!(t.br_indexes, [0, 13]);
        assert_eq!(t.text, "Hello, World!\nBadApple");
    }

    #[test]
    fn delete_br_chain() {
        let mut t = Text::new("Hello, World!\n\n\nBadApple\n".to_string());
        assert_eq!(t.br_indexes, [0, 13, 14, 15, 24]);
        t.update(Change::Delete {
            start: GridIndex {
                row: 1,
                col: NthChar(0),
            },
            end: GridIndex {
                row: 2,
                col: NthChar(0),
            },
        });

        assert_eq!(t.br_indexes, [0, 13, 14, 23]);
        assert_eq!(t.text, "Hello, World!\n\nBadApple\n");
    }

    #[test]
    fn insert() {
        let mut t = Text::new(String::new());
        assert_eq!(t.br_indexes.0, [0]);
        t.update(Change::Insert {
            at: GridIndex {
                row: 0,
                col: NthChar(0),
            },
            text: "Hello, World!".to_string(),
        });

        assert_eq!(t.text, "Hello, World!");
        assert_eq!(t.br_indexes, [0]);
    }

    #[test]
    fn insert_single_line_in_middle() {
        let mut t = Text::new(String::from("ABC\nDEF"));
        assert_eq!(t.br_indexes.0, [0, 3]);
        t.update(Change::Insert {
            at: GridIndex {
                row: 0,
                col: NthChar(1),
            },
            text: "Hello,\n World!\n".to_string(),
        });

        assert_eq!(t.text, "AHello,\n World!\nBC\nDEF");
        assert_eq!(t.br_indexes.0, [0, 7, 15, 18]);
    }
}
