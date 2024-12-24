use core::str;
use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{Debug, Display},
    ops::Range,
};

use tracing::instrument;

use super::{
    encodings::{EncodingFns, UTF16, UTF32, UTF8},
    eol_indexes::EolIndexes,
    lines::{FastEOL, TextLines},
};

use crate::{
    change::{correct_positions, Change, GridIndex},
    error::{Error, Result},
    updateables::{ChangeContext, UpdateContext, Updateable},
};

/// An efficient way to store and process changes made to a text.
///
/// Any method that performs a change on the text also accepts an [`Updateable`] which will be
/// provided with a view of some of the computed values. In case you do not want to provide an
/// [`Updateable`] you may simply provide a `&mut ()` as the argument.
#[derive(Clone, Debug)]
pub struct Text {
    /// The EOL byte positions of the text.
    ///
    /// In case of multibyte EOL patterns (such as `\r\n`) the values point to the last byte.
    ///
    /// If modifying a [`Text`], the changes should also be reflected in [`EolIndexes`].
    /// This is already done when interacting with the implemented methods, but if the string is
    /// manually modified you should reflect to changes here as well.
    pub br_indexes: EolIndexes,
    /// The EOL positions of the text, from the previous update.
    ///
    /// The same rules and restrictions that apply to the current [`EolIndexes`] also apply
    /// here. With one exception, that is until the first update is provided the value will not
    /// store any information. Calling any of the values methods before an update is processed
    /// will very likely result in a panic.
    ///
    /// This is provided to the [`Updateable`] passed to [`Self::update`] to avoid recalculating
    /// positions.
    pub old_br_indexes: EolIndexes,
    /// The text that is stored.
    ///
    /// When an insertion is performed on line count, a line break is inserted.
    /// This means the string stored is not always an exact one to one copy of its source.
    /// If you are to compare the text with its source you may want to normalize their EOL bytes
    /// before doing so.
    ///
    /// When manually modifying the string outside of the provided methods, it is up to the user to
    /// assure that `Text.br_indexes` is alligned with what is present in the string. Not
    /// doing so will result in a panic or incorrect results. If you are only modifying the value through the
    /// provided methods, and only reading from the value, this is not an issue as the implemented methods
    /// guarantee that all of the fields are in sync with each other. Before manually modifying the
    /// value, the current `br_indexes` field should be cloned to `old_br_indexes` and the changes
    /// made on the text should also be reflected to `br_indexes`.
    ///
    /// This is required to correctly update an [`Updateable`] if one is provided.
    pub text: String,
    pub(crate) encoding: EncodingFns,
}

impl Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        self.encoding == other.encoding
            && self.br_indexes == other.br_indexes
            && self.text == other.text
    }
}

impl Text {
    /// Creates a new [`Text`] that expects UTF-8 encoded positions.
    ///
    /// You should generally prefer this method instead of [`Text::new_utf16`] or [`Text::new_utf32`]
    /// and then transform the positions manually when using multiple encoding positions.
    pub fn new(text: String) -> Self {
        let br_indexes = EolIndexes::new(&text);
        Text {
            text,
            br_indexes,
            old_br_indexes: EolIndexes(vec![]),
            encoding: UTF8,
        }
    }

    /// Creates a new [`Text`] that expects UTF-16 encoded positions.
    pub fn new_utf16(text: String) -> Self {
        let br_indexes = EolIndexes::new(&text);
        Text {
            text,
            br_indexes,
            old_br_indexes: EolIndexes(vec![]),
            encoding: UTF16,
        }
    }

    /// Creates a new [`Text`] that expects UTF-32 encoded positions.
    pub fn new_utf32(text: String) -> Self {
        let br_indexes = EolIndexes::new(&text);
        Text {
            text,
            br_indexes,
            old_br_indexes: EolIndexes(vec![]),
            encoding: UTF32,
        }
    }

    /// Perform an a change on the text.
    ///
    /// The positions in the provided [`Change`] will be transformed to the expected encoding
    /// depending on how the [`Text`] was constructed.
    #[instrument(skip(change, updateable))]
    pub fn update<'a, U: Updateable, C: Into<Change<'a>>>(
        &mut self,
        change: C,
        updateable: &mut U,
    ) -> Result<()> {
        // not sure why but my editor gets confused without specifying the type
        let change: Change = change.into();

        match change {
            Change::Delete { start, end } => self.delete(start, end, updateable),
            Change::Insert { text, at } => self.insert(&text, at, updateable),
            Change::Replace { text, start, end } => self.replace(&text, start, end, updateable),
            Change::ReplaceFull(s) => self.replace_full(s, updateable),
        }
    }

    /// Delete between the start and end [`GridIndex`] with the end being exclusive.
    ///
    /// Updates the current [`EolIndexes`] to align to the string.
    /// The [`GridIndex`] columns value is clamped to the end of the string excluding
    /// the EOL bytes.
    ///
    /// # Panics
    ///
    /// If the [`EolIndexes`] of [`Text`] has a length of zero.
    #[inline]
    pub fn delete<U: Updateable>(
        &mut self,
        mut start: GridIndex,
        mut end: GridIndex,
        updateable: &mut U,
    ) -> Result<()> {
        self.update_prep();
        start.normalize(self)?;
        end.normalize(self)?;
        correct_positions(&mut start, &mut end);
        let max_row = self.br_indexes.row_count();
        let row_start_index = self
            .nth_row(start.row)
            .ok_or(Error::oob_row(max_row, start.row))?;
        let row_end_index = self
            .nth_row(end.row)
            .ok_or(Error::oob_row(max_row, end.row))?;
        let start_byte = row_start_index + start.col;
        let end_byte = row_end_index + end.col;
        let byte_range = start_byte..end_byte;
        let br_offset = end_byte - start_byte;

        self.br_indexes.remove_indexes(start.row, end.row);
        self.br_indexes.sub_offsets(start.row, br_offset);

        updateable.update(UpdateContext {
            change: ChangeContext::Delete { start, end },
            breaklines: &self.br_indexes,
            old_breaklines: &self.old_br_indexes,
            old_str: self.text.as_str(),
        })?;

        self.text.drain(byte_range);

        Ok(())
    }

    /// Insert the provided string at the provided [`GridIndex`].
    ///
    /// Updates the current [`EolIndexes`] to align to the string.
    /// The [`GridIndex`] columns value is clamped to the end of the string excluding
    /// the EOL bytes.
    ///
    /// # Panics
    ///
    /// If the [`EolIndexes`] of [`Text`] has a length of zero.
    #[inline]
    pub fn insert<U: Updateable>(
        &mut self,
        s: &str,
        mut at: GridIndex,
        updateable: &mut U,
    ) -> Result<()> {
        self.update_prep();
        at.normalize(self)?;
        let row_count = self.br_indexes.row_count();
        let row_end_index = self
            .nth_row(at.row)
            .ok_or(Error::oob_row(row_count, at.row))?;
        let end_byte = row_end_index + at.col;
        let br_indexes = FastEOL::new(s).map(|i| i + end_byte);
        self.br_indexes.add_offsets(at.row, s.len());
        let inserted_br_indexes = {
            let r = self.br_indexes.insert_indexes(at.row + 1, br_indexes);
            &self.br_indexes.0[r]
        };

        updateable.update(UpdateContext {
            change: ChangeContext::Insert {
                inserted_br_indexes,
                position: at,
                text: s,
            },
            breaklines: &self.br_indexes,
            old_breaklines: &self.old_br_indexes,
            old_str: self.text.as_str(),
        })?;

        self.text.insert_str(end_byte, s);

        Ok(())
    }

    /// Replace start..end with the provided string.
    ///
    /// Updates the current [`EolIndexes`] to align to the string.
    /// The [`GridIndex`] columns value is clamped to the end of the string excluding
    /// the EOL bytes.
    ///
    /// This is more optimized than calling [`String::replace_range`] and then updating the 
    /// [`EolIndexes`] manually.
    ///
    /// # Panics
    ///
    /// If the [`EolIndexes`] of [`Text`] has a length of zero.
    #[inline]
    pub fn replace<U: Updateable>(
        &mut self,
        s: &str,
        mut start: GridIndex,
        mut end: GridIndex,
        updateable: &mut U,
    ) -> Result<()> {
        self.update_prep();
        start.normalize(self)?;
        end.normalize(self)?;
        correct_positions(&mut start, &mut end);
        let row_count = self.br_indexes.row_count();
        let row_start_index = self
            .nth_row(start.row)
            .ok_or(Error::oob_row(row_count, start.row))?;
        let row_end_index = self
            .nth_row(end.row)
            .ok_or(Error::oob_row(row_count, end.row))?;
        let start_byte = row_start_index + start.col;
        let end_byte = row_end_index + end.col;
        let byte_range = start_byte..end_byte;
        let old_len = end_byte - start_byte;
        let new_len = s.len();

        match old_len.cmp(&new_len) {
            Ordering::Less => self.br_indexes.add_offsets(end.row, new_len - old_len),
            Ordering::Greater => self.br_indexes.sub_offsets(end.row, old_len - new_len),
            Ordering::Equal => {}
        }

        let inserted = {
            let r = self.br_indexes.replace_indexes(
                start.row,
                end.row,
                FastEOL::new(s).map(|bri| bri + start_byte),
            );
            &self.br_indexes.0[r]
        };

        updateable.update(UpdateContext {
            change: ChangeContext::Replace {
                start,
                end,
                text: s,
                inserted_br_indexes: inserted,
            },
            breaklines: &self.br_indexes,
            old_breaklines: &self.old_br_indexes,
            old_str: self.text.as_str(),
        })?;

        // String::replace_range contains quite a bit of checks that we do not need.
        // It also internally uses splicing, which (probably) causes the elements to be
        // moved quite a bit when the replacing string exceeds the replaced str length.
        //
        // TODO: replace with safer implenetation.
        // this works and performs very well, problem is there is a ton of unsafe that isn't really
        // needed. We cannot remove all of the unsafe stuff, but I am pretty sure we should be able
        // to rewrite this with 2-3 unsafe calls at most.
        #[inline(always)]
        fn fast_replace_range(text: &mut String, range: Range<usize>, s: &str) {
            let len = text.len();
            assert!(text.is_char_boundary(range.start));
            assert!(text.is_char_boundary(range.end));
            assert!(range.start <= range.end);
            let v = unsafe { text.as_mut_vec() };
            let range_dif = range.end - range.start;
            if range_dif < s.len() {
                v.reserve(s.len() - range_dif);
            }
            let v_ptr = v.as_mut_ptr();
            // SAFETY: We checked the range end is a char boundary which also means it is
            // safe to offset as it also means it is in bounds.
            let end_ptr = unsafe { v_ptr.add(range.end) };

            // In case this panics and it is attempted to be read through unsafe code we
            // dont want to expose possibly invalid UTF-8.
            unsafe { v.set_len(0) };

            // ideally we can remove the branch, but not sure how to do it without
            // introducing safety, or panic problems.
            let new_len = match range_dif.cmp(&s.len()) {
                Ordering::Less => {
                    let dif = s.len() - range_dif;
                    // maybe rotating is faster?
                    unsafe {
                        // SAFETY: range start and end are a char boundary.
                        // We have already reserved the necessary space above so it is safe
                        // to move over the contents.
                        std::ptr::copy(end_ptr, end_ptr.add(dif), len - range.end);
                        len + dif
                    }
                }
                Ordering::Greater => {
                    let dif = range_dif - s.len();
                    unsafe {
                        // SAFETY: range start and end are a char boundary.
                        // Since we are subtracting the new str's len from end - start, it
                        // cannot point to out of bounds.
                        std::ptr::copy(end_ptr, end_ptr.sub(dif), len - range.end);
                        len - dif
                    }
                }
                Ordering::Equal => len,
            };

            unsafe {
                // SAFETY: range start is in a char boundary, we have already reserved
                // space if needed, and moved over the old contents.
                std::ptr::copy_nonoverlapping(s.as_ptr(), v_ptr.add(range.start), s.len());
                // SAFETY: all of the values of the inner Vec is now initialized and valid UTF-8
                v.set_len(new_len);
            };

            // since the length of the string could be very long, we use debug_assert.
            // the assertions at the start of the function already require that the
            // following assertion is true. just another check to be sure.
            debug_assert!(str::from_utf8(v).is_ok());
        }

        fast_replace_range(&mut self.text, byte_range, s);

        Ok(())
    }

    #[inline]
    pub fn replace_full<U: Updateable>(
        &mut self,
        s: Cow<'_, str>,
        updateable: &mut U,
    ) -> Result<()> {
        self.br_indexes = EolIndexes::new(&s);
        updateable.update(UpdateContext {
            change: ChangeContext::ReplaceFull { text: s.as_ref() },
            breaklines: &self.br_indexes,
            old_breaklines: &self.old_br_indexes,
            old_str: self.text.as_str(),
        })?;
        match s {
            Cow::Borrowed(s) => {
                self.text.clear();
                self.text.push_str(s);
            }
            Cow::Owned(s) => self.text = s,
        };

        Ok(())
    }

    /// Returns the start of the nth row.
    ///
    /// If the nth row does not exist, None is returned.
    #[inline]
    fn nth_row(&self, nth: usize) -> Option<usize> {
        self.br_indexes.row_start(nth)
    }

    /// Get the nth row.
    ///
    /// The returned slice is trimmed for any EOL bytes.
    /// Returns None if the nth row does not exist.
    #[inline]
    pub fn get_row(&self, nth: usize) -> Option<&str> {
        self.lines().nth(nth)
    }

    /// Returns an [`Iterator`] over the lines present in the [`Text`].
    ///
    /// The [`Iterator`] implementation of [`TextLines`] is optimized so it is usually a good idea
    /// to use the iterator to get string slices.
    ///
    /// # Panics
    ///
    /// If any of the fields of [`Text`] is out of sync, the iterator may panic or return
    /// incorrect results.
    pub fn lines(&self) -> TextLines {
        TextLines::new(self.text.as_str(), &self.br_indexes.0)
    }

    fn update_prep(&mut self) {
        self.old_br_indexes.clone_from(&self.br_indexes);
    }
}

#[cfg(test)]
mod tests {
    use crate::change::GridIndex;

    use super::Text;

    // All index modifying tests must check the resulting string, and breakline indexes.

    #[test]
    fn nth_row() {
        let t = Text::new("Apple\nOrange\nBanana\nCoconut\nFruity".into());
        assert_eq!(t.br_indexes, [0, 5, 12, 19, 27]);
        assert_eq!(t.nth_row(0), Some(0));
        assert_eq!(t.nth_row(1), Some(6));
        assert_eq!(t.nth_row(2), Some(13));
        assert_eq!(t.nth_row(3), Some(20));
        assert_eq!(t.nth_row(4), Some(28));
        assert_eq!(t.nth_row(5), None);
    }

    mod delete {
        use super::*;

        #[test]
        fn single_line() {
            let mut t = Text::new("Hello, World!".into());
            assert_eq!(t.br_indexes, [0]);
            t.delete(
                GridIndex { row: 0, col: 1 },
                GridIndex { row: 0, col: 6 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0]);
            assert_eq!(t.text, "H World!");
        }

        #[test]
        fn multiline() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 1, col: 3 },
                GridIndex { row: 3, col: 2 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 13]);
            assert_eq!(t.text, "Hello, World!\nAppars");
        }

        #[test]
        fn in_line_into_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 0, col: 1 },
                GridIndex { row: 0, col: 4 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 10, 17, 26]);
            assert_eq!(t.text, "Ho, World!\nApples\n Oranges\nPears");
        }

        #[test]
        fn in_line_at_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 0, col: 0 },
                GridIndex { row: 0, col: 4 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 9, 16, 25]);
            assert_eq!(t.text, "o, World!\nApples\n Oranges\nPears");
        }

        #[test]
        fn across_first_line() {
            let mut t = Text::new("Hello, World!\nApplbs\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 0, col: 3 },
                GridIndex { row: 1, col: 4 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 5, 14]);
            assert_eq!(t.text, "Helbs\n Oranges\nPears");
        }

        #[test]
        fn across_last_line() {
            let mut t = Text::new("Hello, World!\nApplbs\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 2, col: 3 },
                GridIndex { row: 3, col: 2 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 13, 20]);
            assert_eq!(t.text, "Hello, World!\nApplbs\n Orars");
        }

        #[test]
        fn in_line_at_middle() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 2, col: 1 },
                GridIndex { row: 2, col: 4 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 13, 20, 26]);
            assert_eq!(t.text, "Hello, World!\nApples\n nges\nPears");
        }

        #[test]
        fn in_line_at_end() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 3, col: 1 },
                GridIndex { row: 3, col: 4 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            assert_eq!(t.text, "Hello, World!\nApples\n Oranges\nPs");
        }

        #[test]
        fn from_start() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 0, col: 0 },
                GridIndex { row: 0, col: 5 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 8, 15, 24]);
            assert_eq!(t.text, ", World!\nApples\n Oranges\nPears");
        }

        #[test]
        fn from_end() {
            let mut t = Text::new("Hello, World!\nApples\n Oranges\nPears".into());
            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            t.delete(
                GridIndex { row: 3, col: 0 },
                GridIndex { row: 3, col: 5 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 13, 20, 29]);
            assert_eq!(t.text, "Hello, World!\nApples\n Oranges\n");
        }

        #[test]
        fn br() {
            let mut t = Text::new("Hello, World!\nBadApple\n".into());
            assert_eq!(t.br_indexes, [0, 13, 22]);
            t.delete(
                GridIndex { row: 1, col: 8 },
                GridIndex { row: 2, col: 0 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 13]);
            assert_eq!(t.text, "Hello, World!\nBadApple");
        }

        #[test]
        fn br_chain() {
            let mut t = Text::new("Hello, World!\n\n\nBadApple\n".into());
            assert_eq!(t.br_indexes, [0, 13, 14, 15, 24]);
            t.delete(
                GridIndex { row: 1, col: 0 },
                GridIndex { row: 2, col: 0 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.br_indexes, [0, 13, 14, 23]);
            assert_eq!(t.text, "Hello, World!\n\nBadApple\n");
        }

        #[test]
        fn long_text_single_byte() {
            let mut t = Text::new(
                "Hello, World!\nBanana\nHuman\nInteresting\nSuper\nMohawk\nShrek is a great movie."
                    .into(),
            );
            assert_eq!(t.br_indexes, [0, 13, 20, 26, 38, 44, 51]);
            t.delete(
                GridIndex { row: 1, col: 3 },
                GridIndex { row: 5, col: 2 },
                &mut (),
            )
            .unwrap();
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
                    .into(),
            );
            assert_eq!(
                t.br_indexes,
                [0, 81, 148, 230, 274, 275, 342, 400, 479, 573, 574, 632, 693, 796]
            );
            t.delete(
                GridIndex { row: 1, col: 3 },
                GridIndex { row: 5, col: 0 },
                &mut (),
            )
            .unwrap();
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
            t.insert("Hello, World!", GridIndex { row: 0, col: 0 }, &mut ())
                .unwrap();

            assert_eq!(t.text, "Hello, World!");
            assert_eq!(t.br_indexes, [0]);
        }

        #[test]
        fn in_start() {
            let mut t = Text::new(String::from("Apples"));
            assert_eq!(t.br_indexes.0, [0]);
            t.insert("Hello, World!", GridIndex { row: 0, col: 0 }, &mut ())
                .unwrap();

            assert_eq!(t.text, "Hello, World!Apples");
            assert_eq!(t.br_indexes, [0]);
        }

        #[test]
        fn in_end() {
            let mut t = Text::new(String::from("Apples"));
            assert_eq!(t.br_indexes.0, [0]);
            t.insert("Hello, \nWorld!\n", GridIndex { row: 0, col: 6 }, &mut ())
                .unwrap();

            assert_eq!(t.text, "ApplesHello, \nWorld!\n");
            assert_eq!(t.br_indexes, [0, 13, 20]);
        }

        #[test]
        fn end_of_multiline() {
            let mut t = Text::new(String::from("Apples\nBashdjad\nashdkasdh\nasdsad"));
            assert_eq!(t.br_indexes.0, [0, 6, 15, 25]);
            t.insert("Hello, \nWorld!\n", GridIndex { row: 3, col: 2 }, &mut ())
                .unwrap();

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
            t.insert("Hello,\n World!\n", GridIndex { row: 1, col: 1 }, &mut ())
                .unwrap();

            assert_eq!(t.text, "ABC\nDHello,\n World!\nEF");
            assert_eq!(t.br_indexes.0, [0, 3, 11, 19]);
        }

        #[test]
        fn single_line_in_middle() {
            let mut t = Text::new(String::from("ABC\nDEF"));
            assert_eq!(t.br_indexes.0, [0, 3]);
            t.insert("Hello, World!", GridIndex { row: 0, col: 1 }, &mut ())
                .unwrap();

            assert_eq!(t.text, "AHello, World!BC\nDEF");
            assert_eq!(t.br_indexes.0, [0, 16]);
        }

        #[test]
        fn multi_byte() {
            let mut t = Text::new("シュタインズ・ゲートは素晴らしいです。".into());
            assert_eq!(t.br_indexes.0, [0]);
            t.insert(
                "\nHello, ゲートWorld!\n",
                GridIndex { row: 0, col: 3 },
                &mut (),
            )
            .unwrap();

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
                "1234567\nABCD\nHELLO\nWORLD\nSOMELONGLINEFORTESTINGVARIOUSCASES\nAHAHHAHAH".into(),
            );

            assert_eq!(t.br_indexes.0, [0, 7, 12, 18, 24, 59]);

            t.insert(
                "Apple Juice\nBananaMilkshake\nWobbly",
                GridIndex { row: 4, col: 5 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "1234567\nABCD\nHELLO\nWORLD\nSOMELApple Juice\nBananaMilkshake\nWobblyONGLINEFORTESTINGVARIOUSCASES\nAHAHHAHAH"
            );
            assert_eq!(t.br_indexes, [0, 7, 12, 18, 24, 41, 57, 93]);

            assert_eq!(
                &t.text[t.br_indexes.row_start(0).unwrap()..t.br_indexes.0[1]],
                "1234567"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(1).unwrap()..t.br_indexes.0[2]],
                "ABCD"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(2).unwrap()..t.br_indexes.0[3]],
                "HELLO"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(3).unwrap()..t.br_indexes.0[4]],
                "WORLD"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(4).unwrap()..t.br_indexes.0[5]],
                "SOMELApple Juice"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(5).unwrap()..t.br_indexes.0[6]],
                "BananaMilkshake"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(6).unwrap()..t.br_indexes.0[7]],
                "WobblyONGLINEFORTESTINGVARIOUSCASES"
            );
            assert_eq!(&t.text[t.br_indexes.row_start(7).unwrap()..], "AHAHHAHAH");
        }

        #[test]
        fn long_text_multi_byte() {
            let mut t = Text::new(
                "シュタ\nHello, ゲートWorld!\nインズ・ゲートは素晴らしいです。\nこんにちは世界！"
                    .to_string(),
            );

            assert_eq!(t.br_indexes, [0, 9, 32, 81]);

            t.insert(
                "Olá, mundo!\nWaltuh Put the fork away Waltuh.",
                GridIndex { row: 2, col: 3 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "シュタ\nHello, ゲートWorld!\nイOlá, mundo!\nWaltuh Put the fork away Waltuh.ンズ・ゲートは素晴らしいです。\nこんにちは世界！"
            );

            assert_eq!(t.br_indexes, [0, 9, 32, 48, 126]);

            assert_eq!(
                &t.text[t.br_indexes.row_start(0).unwrap()..t.br_indexes.0[1]],
                "シュタ"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(1).unwrap()..t.br_indexes.0[2]],
                "Hello, ゲートWorld!"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(2).unwrap()..t.br_indexes.0[3]],
                "イOlá, mundo!"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(3).unwrap()..t.br_indexes.0[4]],
                "Waltuh Put the fork away Waltuh.ンズ・ゲートは素晴らしいです。"
            );
            assert_eq!(
                &t.text[t.br_indexes.row_start(4).unwrap()..],
                "こんにちは世界！"
            );
        }
    }

    mod replace {
        use super::*;

        #[test]
        fn in_line_start() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny".into());

            assert_eq!(t.br_indexes, [0, 13, 24]);

            t.replace(
                "This Should replace some stuff",
                GridIndex { row: 0, col: 3 },
                GridIndex { row: 0, col: 5 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "HelThis Should replace some stuff, World!\nBye World!\nhahaFunny"
            );
            assert_eq!(t.br_indexes, [0, 41, 52]);
        }

        #[test]
        fn in_line_middle() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny".into());

            assert_eq!(t.br_indexes, [0, 13, 24]);

            t.replace(
                "This Should replace some stuff",
                GridIndex { row: 1, col: 3 },
                GridIndex { row: 1, col: 5 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "Hello, World!\nByeThis Should replace some stufforld!\nhahaFunny"
            );
            assert_eq!(t.br_indexes, [0, 13, 52]);
        }

        #[test]
        fn in_line_end() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny".into());

            assert_eq!(t.br_indexes, [0, 13, 24]);
            t.replace(
                "Wappow! There he stood.",
                GridIndex { row: 0, col: 4 },
                GridIndex { row: 0, col: 13 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.text, "HellWappow! There he stood.\nBye World!\nhahaFunny");
            assert_eq!(t.br_indexes, [0, 27, 38]);
        }

        #[test]
        fn across_first_line() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny".into());

            assert_eq!(t.br_indexes, [0, 13, 24]);
            t.replace(
                "This replaced with the content in the first line\n and second line",
                GridIndex { row: 0, col: 5 },
                GridIndex { row: 1, col: 3 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.text, "HelloThis replaced with the content in the first line\n and second line World!\nhahaFunny");
            assert_eq!(t.br_indexes, [0, 53, 77]);
        }

        #[test]
        fn across_start_and_end_line() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".into());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.replace(
                "What a wonderful world!\nWowzers\nSome Random text",
                GridIndex { row: 0, col: 3 },
                GridIndex { row: 3, col: 6 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "HelWhat a wonderful world!\nWowzers\nSome Random textsting!"
            );

            assert_eq!(t.br_indexes, [0, 26, 34]);
        }

        #[test]
        fn across_end_line() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".into());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.replace(
                "What a wonderful world!\nWowzers\nSome Random text",
                GridIndex { row: 2, col: 3 },
                GridIndex { row: 3, col: 6 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "Hello, World!\nBye World!\nhahWhat a wonderful world!\nWowzers\nSome Random textsting!"
            );

            assert_eq!(t.br_indexes, [0, 13, 24, 51, 59]);
        }

        #[test]
        fn middle_in_line() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".into());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.replace(
                "I am in the middle!\nNo one can stop me.",
                GridIndex { row: 2, col: 1 },
                GridIndex { row: 2, col: 5 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.text, "Hello, World!\nBye World!\nhI am in the middle!\nNo one can stop me.unny\nInteresting!");
            assert_eq!(t.br_indexes, [0, 13, 24, 45, 69]);
        }

        #[test]
        fn middle_no_br_replacement() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".into());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.replace(
                "Look ma, no line breaks",
                GridIndex { row: 1, col: 3 },
                GridIndex { row: 1, col: 6 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "Hello, World!\nByeLook ma, no line breaksrld!\nhahaFunny\nInteresting!"
            );
            assert_eq!(t.br_indexes, [0, 13, 44, 54]);
        }

        #[test]
        fn start_no_br_replacement() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".into());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.replace(
                "Look ma, no line breaks",
                GridIndex { row: 0, col: 3 },
                GridIndex { row: 0, col: 8 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "HelLook ma, no line breaksorld!\nBye World!\nhahaFunny\nInteresting!"
            );
            assert_eq!(t.br_indexes, [0, 31, 42, 52]);
        }

        #[test]
        fn end_no_br_replacement() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".into());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.replace(
                "Look ma, no line breaks",
                GridIndex { row: 3, col: 3 },
                GridIndex { row: 3, col: 8 },
                &mut (),
            )
            .unwrap();

            assert_eq!(
                t.text,
                "Hello, World!\nBye World!\nhahaFunny\nIntLook ma, no line breaksing!"
            );
            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
        }

        #[test]
        fn across_start_and_end_no_br_replacement() {
            let mut t = Text::new("Hello, World!\nBye World!\nhahaFunny\nInteresting!".into());

            assert_eq!(t.br_indexes, [0, 13, 24, 34]);
            t.replace(
                "Look ma, no line breaks",
                GridIndex { row: 0, col: 3 },
                GridIndex { row: 3, col: 8 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.text, "HelLook ma, no line breaksing!");
            assert_eq!(t.br_indexes, [0]);
        }
        #[test]
        fn all() {
            let mut t =
                Text::new("SomeText\nSome Other Text\nSome somsoemesome\n wowoas \n\n".into());

            assert_eq!(t.br_indexes, [0, 8, 24, 42, 51, 52]);
            t.replace(
                "Hello, World!\nBye World!",
                GridIndex { row: 0, col: 0 },
                GridIndex { row: 6, col: 0 },
                &mut (),
            )
            .unwrap();

            assert_eq!(t.text, "Hello, World!\nBye World!");
            assert_eq!(t.br_indexes, [0, 13]);
        }
    }

    // TODO: add mixed tests using all of the possible changes
}
