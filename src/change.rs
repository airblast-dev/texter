use core::str;

use lsp_types::{Position, TextDocumentContentChangeEvent};

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
        let (start, end) = match self {
            Change::Delete { start, end } => (start, end),
            Change::Insert { at, .. } => (&mut GridIndex { row: 0, col: 0 }, at),
            Change::Replace { start, end, .. } => (start, end),
            Change::ReplaceFull(_) => return,
        };

        start.normalize(text);
        end.normalize_exclusive(text);
    }
}

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

impl GridIndex {
    pub fn normalize(&mut self, text: &mut Text) {
        let br_indexes = &text.br_indexes;
        let row_count = br_indexes.row_count();
        if !br_indexes.is_last_row(self.row) {
            let row_start = br_indexes.row_start(self.row);
            let row_end = br_indexes.row_start(self.row + 1);
            let base_line = &text.text[row_start..row_end];
            let pure_line = normalize_non_last_row(base_line);

            self.col = (text.encoding.inclusive)(pure_line, self.col);
        }

        assert!(
            row_count >= self.row,
            "Row value should be at most, row_count"
        );
    }

    pub fn normalize_exclusive(&mut self, text: &mut Text) {
        let br_indexes = &mut text.br_indexes;
        if self.row == br_indexes.row_count() {
            br_indexes.insert_index(self.row, br_indexes.last_row());
            text.text.push('\n');
        }
        let row_count = br_indexes.row_count();

        if !br_indexes.is_last_row(self.row) {
            let row_start = br_indexes.row_start(self.row);
            let row_end = br_indexes.row_start(self.row + 1);
            let base_line = &text.text[row_start..row_end];
            let pure_line = normalize_non_last_row(base_line);

            self.col = (text.encoding.exclusive)(pure_line, self.col);
        }

        assert!(
            row_count > self.row,
            "Row value should be at most, row_count"
        );
    }
}

fn normalize_non_last_row(base_line: &str) -> &str {
    // TODO: add checks for the behavior.
    match base_line.as_bytes() {
        // This pattern should come first as the following pattern could cause an EOL to be
        // included.
        // SAFETY: Since the provided range is based on the length of the str - EOL bytes,
        // worst we can get is an empty str. We are only matching on ascii character bytes,
        // and any byte of a multibyte UTF8 character cannot match with any ascii byte.
        [.., b'\r', b'\n'] => unsafe {
            str::from_utf8_unchecked(base_line.as_bytes().get_unchecked(..base_line.len() - 2))
        },
        // SAFETY: Since the provided range is based on the length of the str - EOL bytes,
        // worst we can get is an empty str. We are only matching on ascii character bytes,
        // and any byte of a multibyte UTF8 character cannot match with any ascii byte.
        [.., b'\n' | b'\r'] => unsafe {
            str::from_utf8_unchecked(base_line.as_bytes().get_unchecked(..base_line.len() - 1))
        },
        _ => base_line,
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
