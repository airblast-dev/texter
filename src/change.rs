use std::borrow::Cow;

use crate::{core::text::Text, error::Result, utils::trim_eol_from_end};

/// A [`Change`] to be performed on a [`Text`].
///
/// A change consists of four primitive text operations. These operations are designed to be simple so
/// that any parser, LSP, or other tooling is able to use the information without too much
/// pre-processing.
///
/// All of the end ranges store store the column exclusively, which means the character at end.col
/// will not be deleted or replaced.
#[derive(Clone, Debug, PartialEq)]
pub enum Change<'a> {
    /// Delete some text between the ranges of `start..end`.
    Delete { start: GridIndex, end: GridIndex },
    /// Insert some text at the position `at`.
    Insert { at: GridIndex, text: Cow<'a, str> },
    /// Replace the text between `start..end`
    ///
    /// Internally uses a more efficient solution than [`String::replace_range`] as it uses
    /// [`Vec::splice`] which is very slow for string operations.
    Replace {
        start: GridIndex,
        end: GridIndex,
        text: Cow<'a, str>,
    },
    /// Fully replace the contents of the text.
    ReplaceFull(Cow<'a, str>),
}

/// A structure denoting text positions for any encoding.
///
/// Both fields are used as an index, which means the first row is always zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridIndex {
    pub row: usize,
    pub col: usize,
}

#[cfg(feature = "tree-sitter")]
mod ts {
    use std::cmp::Ordering;

    use tree_sitter::Point;

    use super::GridIndex;
    impl PartialEq<Point> for GridIndex {
        fn eq(&self, other: &Point) -> bool {
            self.row == other.row && self.col == other.column
        }
    }

    impl PartialOrd<Point> for GridIndex {
        fn partial_cmp(&self, other: &Point) -> Option<std::cmp::Ordering> {
            match self.row.cmp(&other.row) {
                Ordering::Equal => self.col.partial_cmp(&other.column),
                s => Some(s),
            }
        }
    }
    impl From<Point> for GridIndex {
        fn from(value: Point) -> Self {
            GridIndex {
                row: value.row,
                col: value.column,
            }
        }
    }

    impl From<GridIndex> for Point {
        fn from(value: GridIndex) -> Self {
            Point {
                row: value.row,
                column: value.col,
            }
        }
    }
}

#[cfg(feature = "lsp-types")]
mod lspt {
    use lsp_types::{Position, TextDocumentContentChangeEvent};

    use super::{Change, GridIndex};
    impl From<Position> for GridIndex {
        fn from(value: Position) -> Self {
            GridIndex {
                row: value.line as usize,
                col: value.character as usize,
            }
        }
    }

    impl From<GridIndex> for Position {
        fn from(value: GridIndex) -> Self {
            Position {
                line: value.row as u32,
                character: value.col as u32,
            }
        }
    }

    impl From<TextDocumentContentChangeEvent> for Change<'static> {
        fn from(value: TextDocumentContentChangeEvent) -> Self {
            let Some(range) = value.range else {
                return Change::ReplaceFull(value.text.into());
            };

            if value.text.is_empty() {
                return Change::Delete {
                    start: range.start.into(),
                    end: range.end.into(),
                };
            }

            if range.start == range.end {
                return Change::Insert {
                    at: range.start.into(),
                    text: value.text.into(),
                };
            }

            Change::Replace {
                start: range.start.into(),
                end: range.end.into(),
                text: value.text.into(),
            }
        }
    }

    impl<'a> From<&'a TextDocumentContentChangeEvent> for Change<'a> {
        fn from(value: &'a TextDocumentContentChangeEvent) -> Self {
            let Some(range) = value.range else {
                return Change::ReplaceFull((&value.text).into());
            };

            if value.text.is_empty() {
                return Change::Delete {
                    start: range.start.into(),
                    end: range.end.into(),
                };
            }

            if range.start == range.end {
                return Change::Insert {
                    at: range.start.into(),
                    text: (&value.text).into(),
                };
            }

            Change::Replace {
                start: range.start.into(),
                end: range.end.into(),
                text: (&value.text).into(),
            }
        }
    }
}

impl GridIndex {
    /// Transform the positions from the [`Text`]'s expected encoding, to UTF-8 positions.
    ///
    /// If the row value of the [`GridIndex`] is same as the number of rows, this will insert a
    /// line break.
    pub fn normalize(&mut self, text: &mut Text) -> Result<()> {
        let br_indexes = &mut text.br_indexes;
        let mut row_count = br_indexes.row_count();
        if self.row == row_count {
            br_indexes.insert_index(self.row, br_indexes.last_row()?);
            text.text.push('\n');
            row_count += 1;
        }

        let row_start = br_indexes.row_start(self.row)?;
        let pure_line = if !br_indexes.is_last_row(self.row) && row_count > 1 {
            let row_end = br_indexes.row_start(self.row + 1)?;
            let base_line = &text.text[row_start..row_end];
            trim_eol_from_end(base_line)
        } else {
            &text.text[row_start..]
        };

        self.col = (text.encoding[0])(pure_line, self.col)?;

        Ok(())
    }

    /// Transform the positions to the [`Text`]'s expected encoding, from UTF-8 positions.
    pub fn denormalize(&mut self, text: &Text) -> Result<()> {
        let br_indexes = &text.br_indexes;
        let row_count = br_indexes.row_count();
        let row_start = br_indexes.row_start(self.row)?;
        let pure_line = if !br_indexes.is_last_row(self.row) && row_count > 1 {
            let row_end = br_indexes.row_start(self.row + 1)?;
            let base_line = &text.text[row_start..row_end];
            trim_eol_from_end(base_line)
        } else {
            &text.text[row_start..]
        };

        self.col = (text.encoding[1])(pure_line, self.col)?;

        Ok(())
    }
}
