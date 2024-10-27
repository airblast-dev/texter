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
                let (br_offset, drain_range) = 't: {
                    let (row_start, col_start, row_end, col_end) = {
                        let row_start_index = self.br_indexes.row_start(start.row);
                        let row_end_index = self.br_indexes.row_start(end.row);
                        let row_start = &self.text[row_start_index..];
                        let col_start_index = start.col.to_byte_index(row_start);
                        let row_end = &self.text[row_end_index..];
                        let col_end_index = end.col.to_byte_index_exclusive(row_end);
                        assert!(
                            self.br_indexes
                                .0
                                .get(end.row + 1)
                                .is_none_or(|next_index| col_end_index < *next_index),
                            "provided column end exceedes the current line"
                        );
                        (
                            row_start_index,
                            col_start_index,
                            row_end_index,
                            col_end_index,
                        )
                    };
                    let drain_range = row_start + col_start..row_end + col_end;

                    // this isnt just handling a common case, it also avoids an overflow below
                    //
                    // when subtracting with the end column, and adding the start column, things
                    // are fine since end column > start column which means it cannot overflow.
                    //
                    // However when the change is inside a line and the also in the last line, we
                    // end up with a possible overflow.
                    if start.row == end.row {
                        break 't (col_end - col_start, drain_range);
                    }

                    let mut br_offset = row_end - row_start;

                    // if the deleted characters are on the last row, they should not be included
                    // when updating the break line indexes
                    if !self.br_indexes.is_last_row(end.row) {
                        br_offset += col_end;
                    }

                    br_offset -= col_start;

                    (br_offset, drain_range)
                };

                self.br_indexes.remove_indexes(start.row, end.row);
                self.br_indexes.sub_offsets(start.row, br_offset);

                self.text.drain(drain_range);
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

    // All index modifying tests must check the resulting string, end breakline indexes.

    mod delete {
        use super::*;

        #[test]
        fn single_line() {
            let mut t = Text::new("Hello, World!".to_string());
            assert_eq!(t.br_indexes, [0]);
            t.update(crate::change::Change::Delete {
                start: GridIndex {
                    row: 0,
                    col: NthChar(1),
                },
                end: GridIndex {
                    row: 0,
                    col: NthChar(6),
                },
            });

            assert_eq!(t.br_indexes, [0]);
            assert_eq!(t.text, "H World!");
        }

        #[test]
        fn multiline() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
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

            assert_eq!(t.br_indexes, [0, 13]);
            assert_eq!(t.text, "Hello, World!\nAppars");
        }

        #[test]
        fn in_line_into_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(Change::Delete {
                start: GridIndex {
                    row: 0,
                    col: NthChar(1),
                },
                end: GridIndex {
                    row: 0,
                    col: NthChar(4),
                },
            });

            assert_eq!(t.br_indexes, [0, 10, 17, 26]);
            assert_eq!(t.text, "Ho, World!\nApples\n Oranges\nPears");
        }

        #[test]
        fn in_line_at_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(Change::Delete {
                start: GridIndex {
                    row: 0,
                    col: NthChar(0),
                },
                end: GridIndex {
                    row: 0,
                    col: NthChar(4),
                },
            });

            assert_eq!(t.br_indexes, [0, 9, 16, 25]);
            assert_eq!(t.text, "o, World!\nApples\n Oranges\nPears");
        }

        #[test]
        fn across_first_line() {
            let mut t = Text::new("Hello, World!\nApplbs\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(Change::Delete {
                start: GridIndex {
                    row: 0,
                    col: NthChar(3),
                },
                end: GridIndex {
                    row: 1,
                    col: NthChar(4),
                },
            });

            assert_eq!(t.br_indexes, [0, 5, 14]);
            assert_eq!(t.text, "Helbs\n Oranges\nPears");
        }

        #[test]
        fn across_last_line() {
            let mut t = Text::new("Hello, World!\nApplbs\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(Change::Delete {
                start: GridIndex {
                    row: 2,
                    col: NthChar(3),
                },
                end: GridIndex {
                    row: 3,
                    col: NthChar(2),
                },
            });

            assert_eq!(t.br_indexes, [0, 13, 20]);
            assert_eq!(t.text, "Hello, World!\nApplbs\n Orars");
        }

        #[test]
        fn in_line_at_middle() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(Change::Delete {
                start: GridIndex {
                    row: 2,
                    col: NthChar(1),
                },
                end: GridIndex {
                    row: 2,
                    col: NthChar(4),
                },
            });

            assert_eq!(t.br_indexes, [0, 13, 20, 26]);
            assert_eq!(t.text, "Hello, World!\nApples\n nges\nPears");
        }

        #[test]
        fn in_line_at_end() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(Change::Delete {
                start: GridIndex {
                    row: 3,
                    col: NthChar(1),
                },
                end: GridIndex {
                    row: 3,
                    col: NthChar(4),
                },
            });

            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            assert_eq!(t.text, "Hello, World!\nApples\n Oranges\nPs");
        }

        #[test]
        fn from_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(Change::Delete {
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
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(Change::Delete {
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

        #[test]
        fn long_text_single_byte() {
            let mut t = Text::new(
                "Hello, World!\nBanana\nHuman\nInteresting\nSuper\nMohawk\nShrek is a great movie."
                    .to_string(),
            );
            assert_eq!(t.br_indexes, [0, 13, 20, 26, 38, 44, 51]);
            t.update(Change::Delete {
                start: GridIndex {
                    row: 1,
                    col: NthChar(3),
                },
                end: GridIndex {
                    row: 5,
                    col: NthChar(2),
                },
            });
            assert_eq!(t.br_indexes, [0, 13, 21]);
            assert_eq!(t.text, "Hello, World!\nBanhawk\nShrek is a great movie.");
        }

        // TODO: add multibyte checks
    }

    mod insert {
        use super::*;

        // TODO: add more break line index checks

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

        #[test]
        fn long_text_single_byte() {
            let mut t = Text::new(
                "1234567\nABCD\nHELLO\nWORLD\nSOMELONGLINEFORTESTINGVARIOUSCASES\nAHAHHAHAH"
                    .to_string(),
            );

            assert_eq!(t.br_indexes.0, [0, 7, 12, 18, 24, 59]);

            t.update(Change::Insert {
                at: GridIndex {
                    row: 4,
                    col: NthChar(5),
                },
                text: "Apple Juice\nBananaMilkshake\nWobbly".to_string(),
            });

            assert_eq!(
                t.text,
                "1234567\nABCD\nHELLO\nWORLD\nSOMELApple Juice\nBananaMilkshake\nWobblyONGLINEFORTESTINGVARIOUSCASES\nAHAHHAHAH"
            );
            assert_eq!(t.br_indexes, [0, 7, 12, 18, 24, 41, 57, 93]);

            assert_eq!(
                &t.text[t.br_indexes.row_start(0)..t.br_indexes.0[1]],
                "1234567"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(1)..t.br_indexes.0[2]],
                "ABCD"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(2)..t.br_indexes.0[3]],
                "HELLO"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(3)..t.br_indexes.0[4]],
                "WORLD"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(4)..t.br_indexes.0[5]],
                "SOMELApple Juice"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(5)..t.br_indexes.0[6]],
                "BananaMilkshake"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(6)..t.br_indexes.0[7]],
                "WobblyONGLINEFORTESTINGVARIOUSCASES"
            );
            assert_eq!(&t.text[t.br_indexes.row_start(7)..], "AHAHHAHAH");
        }

        #[test]
        fn long_text_multi_byte() {
            let mut t = Text::new(
                "シュタ\nHello, ゲートWorld!\nインズ・ゲートは素晴らしいです。\nこんにちは世界！"
                    .to_string(),
            );

            assert_eq!(t.br_indexes, [0, 9, 32, 81]);

            t.update(Change::Insert {
                at: GridIndex {
                    row: 2,
                    col: NthChar(5),
                },
                text: "Olá, mundo!\nWaltuh Put the fork away Waltuh.".to_string(),
            });

            assert_eq!(
                t.text,
                "シュタ\nHello, ゲートWorld!\nインズ・ゲOlá, mundo!\nWaltuh Put the fork away Waltuh.ートは素晴らしいです。\nこんにちは世界！"
            );

            assert_eq!(t.br_indexes, [0, 9, 32, 60, 126]);

            assert_eq!(
                &t.text[t.br_indexes.row_start(0)..t.br_indexes.0[1]],
                "シュタ"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(1)..t.br_indexes.0[2]],
                "Hello, ゲートWorld!"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(2)..t.br_indexes.0[3]],
                "インズ・ゲOlá, mundo!"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(3)..t.br_indexes.0[4]],
                "Waltuh Put the fork away Waltuh.ートは素晴らしいです。"
            );
            assert_eq!(&t.text[t.br_indexes.row_start(4)..], "こんにちは世界！");
        }
    }
}
