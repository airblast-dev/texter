use std::borrow::Cow;

use crate::{
    core::text::Text,
    updateables::{ChangeContext, UpdateContext, Updateable},
    utils::trim_eol_from_end,
};

/// A [`Change`] to be performed on a [`Text`].
///
/// A change consists of four primitive text operations. These operations are designed to be simple so
/// that any parser, LSP, or other tooling is able to use the information without too much
/// pre-processing.
///
/// All of the end ranges store store the column exclusively, which means the character at end.col
/// will not be deleted or replaced.
///
/// As the mentioned these are only the primitives of actions performed on a [`Text`], for more
/// complex actions you may want to use an [`Actionable`] from [`crate::actions`], or implement an
/// [`Actionable`] yourself.
#[derive(Clone, Debug, PartialEq)]
pub enum Change<'a> {
    /// Delete some text between the ranges of `start..end`.
    Delete { start: GridIndex, end: GridIndex },
    /// Insert some text at the position `at`.
    Insert { at: GridIndex, text: Cow<'a, str> },
    /// Replace the text between `start..end`
    Replace {
        start: GridIndex,
        end: GridIndex,
        text: Cow<'a, str>,
    },
    /// Fully replace the contents of the text.
    ReplaceFull(Cow<'a, str>),
}

impl Change<'_> {
    const EMPTY: Change<'static> = Change::Delete {
        start: GridIndex::BASE_GRID_INDEX,
        end: GridIndex::BASE_GRID_INDEX,
    };
}

/// An action to be transformed into a change or multiple changes.
///
/// Used in defining specific actions such deleting the following word from a position.
///
/// See [`crate::actions`] for actions implemented by this crate.
pub trait Actionable {
    fn to_change<'a>(&'a mut self, text: &Text) -> ActionKind<'a>;
}

// TODO: Probably should add more variants.
/// The kind of action to be performed.
pub enum ActionKind<'a> {
    Once(Change<'a>),
    /// When using this variant the changes are processed in a loop from start to end.
    ///
    /// This means that the second change is applied after the first change. The positions stored
    /// in the changes must be correctly defined to account for any previous changes defined in the
    /// current [`ActionKind::Many`].
    ///
    /// This is the same way the LSP spec requires how edits are to be interpreted.
    Many(Box<[Change<'a>]>),
}

impl Actionable for Change<'_> {
    /// Generally if you are just performing a change, [`Text::update`] or one of the direct
    /// methods should be used.
    ///
    /// However it would be an annoyance if you were to perform numerous dynamically dispatched
    /// (the dyn syntax) actions with one of them just being a change. This allows a change to
    /// be turned into a `Box<dyn Actionable>` or `&dyn Actionable` where needed.
    fn to_change<'a>(&'a mut self, _: &Text) -> ActionKind<'a> {
        let mut ch = Change::EMPTY;
        std::mem::swap(self, &mut ch);
        ActionKind::Once(ch)
    }
}

/// A structure denoting text positions for any encoding.
///
/// Both fields are used as an index, which means the first row is always zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridIndex {
    pub row: usize,
    pub col: usize,
}

impl Updateable for GridIndex {
    /// An implementation of using [`GridIndex`] as a very simple cursor.
    ///
    /// You will almost always want to implement your own cursor as it often requires subtle behavior
    /// changes. This is useful if you just want a dead simple cursor until you implement one specific
    /// to your use case.
    ///
    /// The positions of the [`GridIndex`] are expected, and set to be valid for UTF-8. In case you
    /// want the position as a different encoding you should transform it via
    /// TODO: update this after exposing public interface to easily transform positions.
    fn update(&mut self, ctx: UpdateContext) {
        match ctx.change {
            ChangeContext::Insert {
                position,
                text,
                inserted_br_indexes,
            } => {
                if text.is_empty() {
                    return;
                }
                self.row += inserted_br_indexes.len();
                let start_byte_index = ctx.old_breaklines.row_start(position.row) + position.col;
                let last_lf = inserted_br_indexes
                    .last()
                    .copied()
                    .map(|i| i - start_byte_index)
                    .unwrap_or_default();
                if inserted_br_indexes.is_empty() {
                    self.col += text.len()
                } else {
                    self.col = text.len() - last_lf - 1;
                }
            }
            ChangeContext::Delete { start, .. } => {
                *self = start;
            }
            ChangeContext::Replace {
                start,
                text,
                inserted_br_indexes,
                ..
            } => {
                self.row += inserted_br_indexes.len();
                let start_byte_index = ctx.old_breaklines.row_start(start.row) + start.col;
                let last_lf = inserted_br_indexes
                    .last()
                    .copied()
                    .map(|i| i - start_byte_index)
                    .unwrap_or_default();
                self.col = text.len() - last_lf;
            }
            ChangeContext::ReplaceFull { text } => {
                self.row += ctx.breaklines.last_row();
                self.col += text.len() - ctx.breaklines.row_start(self.row);
            }
        }
    }
}

impl GridIndex {
    /// A cheap to create [`GridIndex`], can be used in cases where a temporary variable is needed
    /// for lifetime purposes.
    const BASE_GRID_INDEX: Self = Self { row: 0, col: 0 };
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
    pub fn normalize(&mut self, text: &mut Text) {
        let br_indexes = &mut text.br_indexes;
        let mut row_count = br_indexes.row_count();
        if self.row == row_count {
            br_indexes.insert_index(self.row, br_indexes.last_row());
            text.text.push('\n');
            row_count += 1;
        }

        let row_start = br_indexes.row_start(self.row);
        let pure_line = if !br_indexes.is_last_row(self.row) && row_count > 1 {
            let row_end = br_indexes.row_start(self.row + 1);
            let base_line = &text.text[row_start..row_end];
            trim_eol_from_end(base_line)
        } else {
            &text.text[row_start..]
        };

        self.col = (text.encoding[0])(pure_line, self.col);

        assert!(
            row_count > self.row,
            "Row value should be at most, row_count"
        );
    }

    /// Transform the positions to the [`Text`]'s expected encoding, from UTF-8 positions.
    pub fn denormalize(&mut self, text: &Text) {
        let br_indexes = &text.br_indexes;
        let row_count = br_indexes.row_count();
        let row_start = br_indexes.row_start(self.row);
        let pure_line = if !br_indexes.is_last_row(self.row) && row_count > 1 {
            let row_end = br_indexes.row_start(self.row + 1);
            let base_line = &text.text[row_start..row_end];
            trim_eol_from_end(base_line)
        } else {
            &text.text[row_start..]
        };

        self.col = (text.encoding[1])(pure_line, self.col);
    }
}
