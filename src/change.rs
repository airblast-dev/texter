use core::str;
use std::cmp::Ordering;

use lsp_types::{Position, TextDocumentContentChangeEvent};
use tree_sitter::Point;

use crate::core::text::Text;

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
        let grid_index: &mut GridIndex = match self {
            Change::Delete { end, .. } => end,
            Change::Insert { at, .. } => at,
            Change::Replace { end, .. } => end,
            Change::ReplaceFull(_) => return,
        };

        grid_index.normalize(text);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridIndex {
    pub row: usize,
    pub col: usize,
}

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

impl GridIndex {
    pub fn normalize(&mut self, text: &mut Text) {
        let br_indexes = &text.br_indexes;
        let row_count = br_indexes.row_count();

        if self.row < row_count - 1 {
            let row_start = br_indexes.row_start(self.row);
            let row_end = br_indexes.row_start(self.row + 1);
            let base_line = &text.text[row_start..row_end];
            // TODO: add checks for the behavior.
            let pure_line = match base_line.as_bytes() {
                // This pattern should come first as the following pattern could cause an EOL to be
                // included.
                // SAFETY: Since the provided range is based on the length of the str - EOL bytes,
                // worst we can get is an empty str. We are only matching on ascii character bytes,
                // and any byte of a multibyte UTF8 character cannot match with any ascii byte.
                [.., b'\r', b'\n'] => unsafe {
                    str::from_utf8_unchecked(
                        base_line.as_bytes().get_unchecked(..base_line.len() - 2),
                    )
                },
                // SAFETY: Since the provided range is based on the length of the str - EOL bytes,
                // worst we can get is an empty str. We are only matching on ascii character bytes,
                // and any byte of a multibyte UTF8 character cannot match with any ascii byte.
                [.., b'\n' | b'\r'] => unsafe {
                    str::from_utf8_unchecked(
                        base_line.as_bytes().get_unchecked(..base_line.len() - 1),
                    )
                },
                _ => base_line,
            };

            // using debug assert in case of very long lines
            debug_assert!(!pure_line.contains(['\n', '\r']));
            self.col = self.col.min((text.encoding.exclusive)(pure_line, self.col));
        }

        let br_indexes = &mut text.br_indexes;

        assert!(
            row_count >= self.row,
            "Row value should be at most, row_count"
        );

        if self.row == row_count {
            br_indexes.insert_index(self.row, br_indexes.last_row());
            text.text.push('\n');
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
