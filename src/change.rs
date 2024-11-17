use core::str;

use crate::{core::text::Text, utils::trim_eol_from_end};

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Delete {
        start: GridIndex,
        end: GridIndex,
    },
    Insert {
        at: GridIndex,
        text: String,
    },
    Replace {
        start: GridIndex,
        end: GridIndex,
        text: String,
    },
    ReplaceFull(String),
}

impl Change {
    /// Normalize the provided the grid index.
    ///
    /// When converting a type to [`Change`], the values may not strictly align with what is
    /// present.
    pub(crate) fn normalize(&mut self, text: &mut Text) {
        let (start, end) = match self {
            Change::Delete { start, end } => (start, end),
            Change::Insert { at, .. } => (&mut GridIndex { row: 0, col: 0 }, at),
            Change::Replace { start, end, .. } => (start, end),
            Change::ReplaceFull(_) => return,
        };

        start.normalize(text);
        end.normalize(text);
    }

    #[inline(always)]
    pub(crate) fn range(&self) -> (GridIndex, GridIndex) {
        match self {
            Change::Replace { start, end, .. } => (*start, *end),
            Change::Insert { at, .. } => (GridIndex::BASE_GRID_INDEX, *at),
            Change::Delete { start, end } => (*start, *end),
            Change::ReplaceFull(_) => (GridIndex::BASE_GRID_INDEX, GridIndex::BASE_GRID_INDEX),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridIndex {
    pub row: usize,
    pub col: usize,
}

impl GridIndex {
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
    impl From<TextDocumentContentChangeEvent> for Change {
        fn from(value: TextDocumentContentChangeEvent) -> Self {
            let Some(range) = value.range else {
                return Change::ReplaceFull(value.text);
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
                    text: value.text,
                };
            }

            Change::Replace {
                start: range.start.into(),
                end: range.end.into(),
                text: value.text,
            }
        }
    }
}

impl GridIndex {
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


