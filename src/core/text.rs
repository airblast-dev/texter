use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
};

use super::{
    br_indexes::BrIndexes,
    encodings::{UTF16, UTF32, UTF8},
    lines::FastEOL,
};

use crate::{
    change::Change,
    updateables::{ChangeContext, UpdateContext, Updateable},
};

#[derive(Clone)]
pub struct Text {
    /// The EOL byte positions of the text.
    ///
    /// In case of multibyte EOL patterns (such as `\r\n`) the values point to the last byte.
    ///
    /// If modifying `Text.text`, the changes should also be reflected in [`BrIndexes`].
    pub br_indexes: BrIndexes,
    /// The EOL positions of the text, from the previous update.
    ///
    /// The same rules and restrictions that apply to the current [`BrIndexes`] also apply
    /// here.
    ///
    /// This is provided to the [`Updateable`] passed to [`Self::update`] to avoid recalculating
    /// positions.
    pub old_br_indexes: BrIndexes,
    /// The text that is stored.
    ///
    /// When an insertion is performed on line count + 1, a line break is inserted.
    /// This means the string stored is not always an exact one to one copy of its source.
    ///
    /// When manually modifying the string outside of the provided methods, it is up to the user to
    /// assure that the `Text.br_indexes` are alligned with what is present in the string.
    pub text: String,
    pub(crate) encoding: fn(&str, usize) -> usize,
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
    /// Creates a new [`Text`] for UTF8 encoded positions.
    pub fn new(text: String) -> Self {
        let br_indexes = BrIndexes::new(&text);
        Text {
            text,
            br_indexes,
            old_br_indexes: BrIndexes(vec![]),
            encoding: UTF8,
        }
    }

    /// Creates a new [`Text`] for UTF16 encoded positions.
    pub fn new_utf16(text: String) -> Self {
        let br_indexes = BrIndexes::new(&text);
        Text {
            text,
            br_indexes,
            old_br_indexes: BrIndexes(vec![]),
            encoding: UTF16,
        }
    }

    /// Creates a new [`Text`] for UTF32 encoded positions.
    pub fn new_utf32(text: String) -> Self {
        let br_indexes = BrIndexes::new(&text);
        Text {
            text,
            br_indexes,
            old_br_indexes: BrIndexes(vec![]),
            encoding: UTF32,
        }
    }

    pub fn update<U: Updateable, C: Into<Change>>(&mut self, change: C, updateable: &mut U) {
        let mut change = change.into();
        change.normalize(self);
        self.old_br_indexes.clone_from(&self.br_indexes);
        match change {
            Change::Delete { start, end } => {
                let (br_offset, drain_range) = 't: {
                    let (row_start, col_start, row_end, col_end) = {
                        let row_start_index = self.nth_row(start.row);
                        let row_end_index = self.nth_row(end.row);
                        (row_start_index, start.col, row_end_index, end.col)
                    };
                    let drain_range = row_start + col_start..row_end + col_end;

                    // this isnt just handling a common case, it also avoids an overflow below
                    //
                    // when subtracting with the end column, and adding the start column, things
                    // are fine since end column > start column which means it cannot overflow.
                    //
                    // However when the change is inside a line (start.row == end.row) and the row is
                    // also in the last line, we end up with a possible overflow since the column
                    // end should not be included in the offsets.
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

                updateable.update(UpdateContext {
                    change: ChangeContext::Delete { start, end },
                    breaklines: &self.br_indexes,
                    old_breaklines: &self.old_br_indexes,
                    old_str: self.text.as_str(),
                });

                self.text.drain(drain_range);
            }
            Change::Insert { at, text } => {
                let start_br = self.nth_row(at.row);
                let insertion_index = at.col + start_br;

                let br_indexes = FastEOL::new(&text).map(|i| i + insertion_index);
                self.br_indexes.add_offsets(at.row, text.len());
                let inserted_br_indexes = {
                    let r = self.br_indexes.insert_indexes(at.row + 1, br_indexes);
                    &self.br_indexes[r]
                };

                updateable.update(UpdateContext {
                    change: ChangeContext::Insert {
                        inserted_br_indexes,
                        position: at,
                        text: text.as_str(),
                    },
                    breaklines: &self.br_indexes,
                    old_breaklines: &self.old_br_indexes,
                    old_str: self.text.as_str(),
                });

                self.text.insert_str(insertion_index, &text);
            }
            Change::Replace { start, end, text } => {
                let start_br = self.nth_row(start.row);
                let replace_start_col = start.col;
                let end_br = self.nth_row(end.row);
                let replace_end_col = end.col;
                let old_len = end_br + replace_end_col - (start_br + replace_start_col);
                let new_len = text.len();

                let start_index = start_br + replace_start_col;
                let end_index = end_br + replace_end_col;

                match old_len.cmp(&new_len) {
                    Ordering::Less => self.br_indexes.add_offsets(end.row, new_len - old_len),
                    Ordering::Greater => self.br_indexes.sub_offsets(end.row, old_len - new_len),
                    Ordering::Equal => {}
                }

                let inserted = {
                    let r = self.br_indexes.replace_indexes(
                        start.row,
                        end.row,
                        FastEOL::new(&text).map(|bri| bri + start_index),
                    );
                    &self.br_indexes[r]
                };

                updateable.update(UpdateContext {
                    change: ChangeContext::Replace {
                        start,
                        end,
                        text: text.as_str(),
                        inserted_br_indexes: inserted,
                    },
                    breaklines: &self.br_indexes,
                    old_breaklines: &self.old_br_indexes,
                    old_str: self.text.as_str(),
                });

                self.text.replace_range(start_index..end_index, &text);
            }
            Change::ReplaceFull(s) => {
                self.br_indexes = BrIndexes::new(&s);
                updateable.update(UpdateContext {
                    change: ChangeContext::ReplaceFull { text: s.as_str() },
                    breaklines: &self.br_indexes,
                    old_breaklines: &self.old_br_indexes,
                    old_str: self.text.as_str(),
                });
                self.text = s;
            }
        }
    }

    /// returns the nth row including the trailing line break if one if present
    #[inline]
    fn nth_row(&self, r: usize) -> usize {
        self.br_indexes.row_start(r)
    }
}

#[cfg(test)]
mod tests {
    use crate::change::{Change, GridIndex};

    use super::Text;

    // All index modifying tests must check the resulting string, and breakline indexes.

    #[test]
    fn nth_row() {
        let t = Text::new("Apple\nOrange\nBanana\nCoconut\nFruity".to_string());
        assert_eq!(t.br_indexes, [0, 5, 12, 19, 27]);
        assert_eq!(t.nth_row(0), 0);
        assert_eq!(t.nth_row(1), 6);
        assert_eq!(t.nth_row(2), 13);
        assert_eq!(t.nth_row(3), 20);
        assert_eq!(t.nth_row(4), 28);
    }

    mod delete {
        use super::*;

        #[test]
        fn single_line() {
            let mut t = Text::new("Hello, World!".to_string());
            assert_eq!(t.br_indexes, [0]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 0, col: 1 },
                    end: GridIndex { row: 0, col: 6 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0]);
            assert_eq!(t.text, "H World!");
        }

        #[test]
        fn multiline() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 1, col: 3 },
                    end: GridIndex { row: 3, col: 2 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 13]);
            assert_eq!(t.text, "Hello, World!\nAppars");
        }

        #[test]
        fn in_line_into_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 0, col: 1 },
                    end: GridIndex { row: 0, col: 4 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 10, 17, 26]);
            assert_eq!(t.text, "Ho, World!\nApples\n Oranges\nPears");
        }

        #[test]
        fn in_line_at_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 0, col: 0 },
                    end: GridIndex { row: 0, col: 4 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 9, 16, 25]);
            assert_eq!(t.text, "o, World!\nApples\n Oranges\nPears");
        }

        #[test]
        fn across_first_line() {
            let mut t = Text::new("Hello, World!\nApplbs\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 0, col: 3 },
                    end: GridIndex { row: 1, col: 4 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 5, 14]);
            assert_eq!(t.text, "Helbs\n Oranges\nPears");
        }

        #[test]
        fn across_last_line() {
            let mut t = Text::new("Hello, World!\nApplbs\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 2, col: 3 },
                    end: GridIndex { row: 3, col: 2 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 13, 20]);
            assert_eq!(t.text, "Hello, World!\nApplbs\n Orars");
        }

        #[test]
        fn in_line_at_middle() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 2, col: 1 },
                    end: GridIndex { row: 2, col: 4 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 13, 20, 26]);
            assert_eq!(t.text, "Hello, World!\nApples\n nges\nPears");
        }

        #[test]
        fn in_line_at_end() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 3, col: 1 },
                    end: GridIndex { row: 3, col: 4 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            assert_eq!(t.text, "Hello, World!\nApples\n Oranges\nPs");
        }

        #[test]
        fn from_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 0, col: 0 },
                    end: GridIndex { row: 0, col: 5 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 8, 15, 24]);
            assert_eq!(t.text, ", World!\nApples\n Oranges\nPears");
        }

        #[test]
        fn from_end() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".to_string());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 3, col: 0 },
                    end: GridIndex { row: 3, col: 5 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            assert_eq!(t.text, "Hello, World!\nApples\n Oranges\n");
        }

        #[test]
        fn br() {
            let mut t = Text::new("Hello, World!\nBadApple\n".to_string());
            assert_eq!(t.br_indexes, [0, 13, 22]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 1, col: 8 },
                    end: GridIndex { row: 2, col: 0 },
                },
                &mut (),
            );

            assert_eq!(t.br_indexes, [0, 13]);
            assert_eq!(t.text, "Hello, World!\nBadApple");
        }

        #[test]
        fn br_chain() {
            let mut t = Text::new("Hello, World!\n\n\nBadApple\n".to_string());
            assert_eq!(t.br_indexes, [0, 13, 14, 15, 24]);
            t.update(
                Change::Delete {
                    start: GridIndex { row: 1, col: 0 },
                    end: GridIndex { row: 2, col: 0 },
                },
                &mut (),
            );

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
            t.update(
                Change::Delete {
                    start: GridIndex { row: 1, col: 3 },
                    end: GridIndex { row: 5, col: 2 },
                },
                &mut (),
            );
            assert_eq!(t.br_indexes, [0, 13, 21]);
            assert_eq!(t.text, "Hello, World!\nBanhawk\nShrek is a great movie.");
        }

        #[test]
        fn long_text_multi_byte() {
            let mut t = Text::new(
                "\
誰かがかつて世界が私をロールつもりである私に言いました
私は小屋で最もシャープなツールではありません
彼女は彼女の指と親指でダムのようなものを探していました
彼女の額の「L」の形をしました

さて、年が来て起動し、彼らが来て停止しません
ルールに供給され、私は地上走行をヒット
楽しみのために生きることはない意味がありませんでした
あなたの脳は、スマート取得しますが、あなたの頭はダム取得します

見るために、あまりを行うことがそんなに
だから、裏通りを取ると間違って何ですか？
あなたが行っていない場合は、あなたが知っていることは決してないだろう
あなたが輝くない場合は輝くことは決してないだろう"
                    .to_string(),
            );
            assert_eq!(
                t.br_indexes,
                [0, 81, 148, 230, 274, 275, 342, 400, 479, 573, 574, 632, 693, 796]
            );
            t.update(
                Change::Delete {
                    start: GridIndex { row: 1, col: 3 },
                    end: GridIndex { row: 5, col: 0 },
                },
                &mut (),
            );
            assert_eq!(
                t.br_indexes,
                [0, 81, 151, 209, 288, 382, 383, 441, 502, 605]
            );
            assert_eq!(
                t.text,
                "\
誰かがかつて世界が私をロールつもりである私に言いました
私さて、年が来て起動し、彼らが来て停止しません
ルールに供給され、私は地上走行をヒット
楽しみのために生きることはない意味がありませんでした
あなたの脳は、スマート取得しますが、あなたの頭はダム取得します

見るために、あまりを行うことがそんなに
だから、裏通りを取ると間違って何ですか？
あなたが行っていない場合は、あなたが知っていることは決してないだろう
あなたが輝くない場合は輝くことは決してないだろう"
            );
        }
    }

    mod insert {
        use super::*;

        #[test]
        fn into_empty() {
            let mut t = Text::new(String::new());
            assert_eq!(t.br_indexes.0, [0]);
            t.update(
                Change::Insert {
                    at: GridIndex { row: 0, col: 0 },
                    text: "Hello, World!".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "Hello, World!");
            assert_eq!(t.br_indexes, [0]);
        }

        #[test]
        fn in_start() {
            let mut t = Text::new(String::from("Apples"));
            assert_eq!(t.br_indexes.0, [0]);
            t.update(
                Change::Insert {
                    at: GridIndex { row: 0, col: 0 },
                    text: "Hello, World!".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "Hello, World!Apples");
            assert_eq!(t.br_indexes, [0]);
        }

        #[test]
        fn in_end() {
            let mut t = Text::new(String::from("Apples"));
            assert_eq!(t.br_indexes.0, [0]);
            t.update(
                Change::Insert {
                    at: GridIndex { row: 0, col: 6 },
                    text: "Hello, \nWorld!\n".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "ApplesHello, \nWorld!\n");
            assert_eq!(t.br_indexes, [0, 13, 20]);
        }

        #[test]
        fn end_of_multiline() {
            let mut t = Text::new(String::from("Apples\nBashdjad\nashdkasdh\nasdsad"));
            assert_eq!(t.br_indexes.0, [0, 6, 15, 25]);
            t.update(
                Change::Insert {
                    at: GridIndex { row: 3, col: 2 },
                    text: "Hello, \nWorld!\n".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "Apples\nBashdjad\nashdkasdh\nasHello, \nWorld!\ndsad"
            );
            assert_eq!(t.br_indexes, [0, 6, 15, 25, 35, 42]);
        }

        #[test]
        fn multi_line_in_middle() {
            let mut t = Text::new(String::from("ABC\nDEF"));
            assert_eq!(t.br_indexes.0, [0, 3]);
            t.update(
                Change::Insert {
                    at: GridIndex { row: 1, col: 1 },
                    text: "Hello,\n World!\n".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "ABC\nDHello,\n World!\nEF");
            assert_eq!(t.br_indexes.0, [0, 3, 11, 19]);
        }

        #[test]
        fn single_line_in_middle() {
            let mut t = Text::new(String::from("ABC\nDEF"));
            assert_eq!(t.br_indexes.0, [0, 3]);
            t.update(
                Change::Insert {
                    at: GridIndex { row: 0, col: 1 },
                    text: "Hello, World!".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "AHello, World!BC\nDEF");
            assert_eq!(t.br_indexes.0, [0, 16]);
        }

        #[test]
        fn multi_byte() {
            let mut t = Text::new("シュタインズ・ゲートは素晴らしいです。".to_string());
            assert_eq!(t.br_indexes.0, [0]);
            t.update(
                Change::Insert {
                    at: GridIndex { row: 0, col: 3 },
                    text: "\nHello, ゲートWorld!\n".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "シ\nHello, ゲートWorld!\nュタインズ・ゲートは素晴らしいです。"
            );
            assert_eq!(t.br_indexes, [0, 3, 26]);
            assert_eq!(
                &t.text[t.br_indexes.0[1] + 1..t.br_indexes.0[2]],
                "Hello, ゲートWorld!"
            );
            assert_eq!(
                &t.text[t.br_indexes.0[2] + 1..],
                "ュタインズ・ゲートは素晴らしいです。"
            )
        }

        #[test]
        fn long_text_single_byte() {
            let mut t = Text::new(
                "1234567\nABCD\nHELLO\nWORLD\nSOMELONGLINEFORTESTINGVARIOUSCASES\nAHAHHAHAH"
                    .to_string(),
            );

            assert_eq!(t.br_indexes.0, [0, 7, 12, 18, 24, 59]);

            t.update(
                Change::Insert {
                    at: GridIndex { row: 4, col: 5 },
                    text: "Apple Juice\nBananaMilkshake\nWobbly".to_string(),
                },
                &mut (),
            );

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

            t.update(
                Change::Insert {
                    at: GridIndex { row: 2, col: 3 },
                    text: "Olá, mundo!\nWaltuh Put the fork away Waltuh.".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "シュタ\nHello, ゲートWorld!\nイOlá, mundo!\nWaltuh Put the fork away Waltuh.ンズ・ゲートは素晴らしいです。\nこんにちは世界！"
            );

            assert_eq!(t.br_indexes, [0, 9, 32, 48, 126]);

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
                "イOlá, mundo!"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(3)..t.br_indexes.0[4]],
                "Waltuh Put the fork away Waltuh.ンズ・ゲートは素晴らしいです。"
            );
            assert_eq!(&t.text[t.br_indexes.row_start(4)..], "こんにちは世界！");
        }
    }

    mod replace {
        use super::*;

        #[test]
        fn in_line_start() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24]);

            t.update(
                Change::Replace {
                    start: GridIndex { row: 0, col: 3 },
                    end: GridIndex { row: 0, col: 5 },
                    text: "This Should replace some stuff".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "HelThis Should replace some stuff, World!\nBye World!\nhahaFunny"
            );
            assert_eq!(t.br_indexes, [0, 41, 52]);
        }

        #[test]
        fn in_line_middle() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24]);

            t.update(
                Change::Replace {
                    start: GridIndex { row: 1, col: 3 },
                    end: GridIndex { row: 1, col: 5 },
                    text: "This Should replace some stuff".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "Hello, World!\nByeThis Should replace some stufforld!\nhahaFunny"
            );
            assert_eq!(t.br_indexes, [0, 13, 52]);
        }

        #[test]
        fn in_line_end() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 0, col: 4 },
                    end: GridIndex { row: 0, col: 13 },
                    text: "Wappow! There he stood.".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "HellWappow! There he stood.\nBye World!\nhahaFunny");
            assert_eq!(t.br_indexes, [0, 27, 38]);
        }

        #[test]
        fn across_first_line() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 0, col: 5 },
                    end: GridIndex { row: 1, col: 3 },
                    text: "This replaced with the content in the first line\n and second line"
                        .to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "HelloThis replaced with the content in the first line\n and second line World!\nhahaFunny");
            assert_eq!(t.br_indexes, [0, 53, 77]);
        }

        #[test]
        fn across_start_and_end_line() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 0, col: 3 },
                    end: GridIndex { row: 3, col: 6 },
                    text: "What a wonderful world!\nWowzers\nSome Random text".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "HelWhat a wonderful world!\nWowzers\nSome Random textsting!"
            );

            assert_eq!(t.br_indexes, [0, 26, 34]);
        }

        #[test]
        fn across_end_line() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 2, col: 3 },
                    end: GridIndex { row: 3, col: 6 },
                    text: "What a wonderful world!\nWowzers\nSome Random text".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "Hello, World!\nBye World!\nhahWhat a wonderful world!\nWowzers\nSome Random textsting!"
            );

            assert_eq!(t.br_indexes, [0, 13, 24, 51, 59]);
        }

        #[test]
        fn middle_in_line() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 2, col: 1 },
                    end: GridIndex { row: 2, col: 5 },
                    text: "I am in the middle!\nNo one can stop me.".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "Hello, World!\nBye World!\nhI am in the middle!\nNo one can stop me.unny\nInteresting!");
            assert_eq!(t.br_indexes, [0, 13, 24, 45, 69]);
        }

        #[test]
        fn middle_no_br_replacement() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 1, col: 3 },
                    end: GridIndex { row: 1, col: 6 },
                    text: "Look ma, no line breaks".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "Hello, World!\nByeLook ma, no line breaksrld!\nhahaFunny\nInteresting!"
            );
            assert_eq!(t.br_indexes, [0, 13, 44, 54]);
        }

        #[test]
        fn start_no_br_replacement() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 0, col: 3 },
                    end: GridIndex { row: 0, col: 8 },
                    text: "Look ma, no line breaks".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "HelLook ma, no line breaksorld!\nBye World!\nhahaFunny\nInteresting!"
            );
            assert_eq!(t.br_indexes, [0, 31, 42, 52]);
        }

        #[test]
        fn end_no_br_replacement() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 3, col: 3 },
                    end: GridIndex { row: 3, col: 8 },
                    text: "Look ma, no line breaks".to_string(),
                },
                &mut (),
            );

            assert_eq!(
                t.text,
                "Hello, World!\nBye World!\nhahaFunny\nIntLook ma, no line breaksing!"
            );
            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
        }

        #[test]
        fn across_start_and_end_no_br_replacement() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".to_string());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 0, col: 3 },
                    end: GridIndex { row: 3, col: 8 },
                    text: "Look ma, no line breaks".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "HelLook ma, no line breaksing!");
            assert_eq!(t.br_indexes, [0]);
        }
        #[test]
        fn all() {
            let mut t =
                Text::new("SomeText\nSome Other Text\nSome somsoemesome\n wowoas \n\n".to_string());

            assert_eq!(t.br_indexes, [0, 8, 24, 42, 51, 52]);
            t.update(
                Change::Replace {
                    start: GridIndex { row: 0, col: 0 },
                    end: GridIndex { row: 6, col: 0 },
                    text: "Hello, World!\nBye World!".to_string(),
                },
                &mut (),
            );

            assert_eq!(t.text, "Hello, World!\nBye World!");
            assert_eq!(t.br_indexes, [0, 13]);
        }
    }

    // TODO: add mixed tests using all of the possible changes
}
