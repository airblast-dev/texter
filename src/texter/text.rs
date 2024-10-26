use std::fmt::{Debug, Display};

use super::{br_indexes::BrIndexes, BR_FINDER};

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
                    .sub_offsets(start.row, end_index - start_index);

                self.text.drain(start_index..end_index);
            }
            Change::Insert { at, text } => {
                let br_indexes = BR_FINDER.find_iter(text.as_bytes());
                let start_br = self.br_indexes.row_start(at.row);
                let start = &self.text[start_br..];
                let insertion_index = at.col.to_byte_index_exclusive(start) + start_br;
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

    mod delete {
        use super::*;

        #[test]
        fn multiline() {
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
        fn in_line() {
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
        fn from_start() {
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
        fn from_end() {
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
        fn br() {
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
        fn br_chain() {
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
    }

    mod insert {
        use super::*;

        #[test]
        fn into_empty() {
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
        fn in_start() {
            let mut t = Text::new(String::from("Apples"));
            assert_eq!(t.br_indexes.0, [0]);
            t.update(Change::Insert {
                at: GridIndex {
                    row: 0,
                    col: NthChar(0),
                },
                text: "Hello, World!".to_string(),
            });

            assert_eq!(t.text, "Hello, World!Apples");
            assert_eq!(t.br_indexes, [0]);
        }

        #[test]
        fn in_end() {
            let mut t = Text::new(String::from("Apples"));
            assert_eq!(t.br_indexes.0, [0]);
            t.update(Change::Insert {
                at: GridIndex {
                    row: 0,
                    col: NthChar(6),
                },
                text: "Hello, \nWorld!\n".to_string(),
            });

            assert_eq!(t.text, "ApplesHello, \nWorld!\n");
            assert_eq!(t.br_indexes, [0, 13, 20]);
        }

        #[test]
        fn multi_line_in_middle() {
            let mut t = Text::new(String::from("ABC\nDEF"));
            assert_eq!(t.br_indexes.0, [0, 3]);
            t.update(Change::Insert {
                at: GridIndex {
                    row: 1,
                    col: NthChar(1),
                },
                text: "Hello,\n World!\n".to_string(),
            });

            assert_eq!(t.text, "ABC\nDHello,\n World!\nEF");
            assert_eq!(t.br_indexes.0, [0, 3, 11, 19]);
        }

        #[test]
        fn single_line_in_middle() {
            let mut t = Text::new(String::from("ABC\nDEF"));
            assert_eq!(t.br_indexes.0, [0, 3]);
            t.update(Change::Insert {
                at: GridIndex {
                    row: 0,
                    col: NthChar(1),
                },
                text: "Hello, World!".to_string(),
            });

            assert_eq!(t.text, "AHello, World!BC\nDEF");
            assert_eq!(t.br_indexes.0, [0, 16]);
        }

        #[test]
        fn multi_byte() {
            let mut t = Text::new("シュタインズ・ゲートは素晴らしいです。".to_string());
            assert_eq!(t.br_indexes.0, [0]);
            t.update(Change::Insert {
                at: GridIndex {
                    row: 0,
                    col: NthChar(3),
                },
                text: "\nHello, ゲートWorld!\n".to_string(),
            });

            assert_eq!(
                t.text,
                "シュタ\nHello, ゲートWorld!\nインズ・ゲートは素晴らしいです。"
            );
            assert_eq!(t.br_indexes, [0, 9, 32]);
            assert_eq!(
                &t.text[t.br_indexes.0[1] + 1..t.br_indexes.0[2]],
                "Hello, ゲートWorld!"
            );
            assert_eq!(
                &t.text[t.br_indexes.0[2] + 1..],
                "インズ・ゲートは素晴らしいです。"
            )
        }
    }
}
