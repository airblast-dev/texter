use std::fmt::Debug;

use lsp_types::{Position, TextDocumentContentChangeEvent};

use crate::core::text::Text;

#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct GridIndex {
    pub row: usize,
    pub col: usize,
}

impl From<Position> for GridIndex {
    fn from(value: Position) -> Self {
        GridIndex {
            row: value.line as usize,
            col: value.character as usize,
        }
    }
}

impl GridIndex {
    pub fn normalize(&mut self, text: &mut Text) {
        let br_indexes = &mut text.br_indexes;
        let row_count = br_indexes.row_count();

        if self.row < row_count - 1 {
            let row_start = br_indexes.row_start(self.row);
            let row_end = br_indexes.row_start(self.row + 1);
            let base_line = &text.text[row_start..row_end];
            // TODO: add checks for the behavior.
            // TODO: we probably could do better checks and optimize this.
            let pure_line = base_line.trim_end_matches(['\r', '\n']);
            // based on the LSP standard these two characters are considered EOL.
            // A lsp_types::Range should not point to EOL bytes, or beyond a single row.
            // The documented behavior we should follow is to exclusively clamp the value to the end of the row
            // excluding the EOL bytes. In other words, the character value can at most point to
            // the index of first EOL byte.

            // we should only have at most two bytes ("\r\n") trimmed.
            // this check and the trimming above should be a bit more sophisticated.
            assert!(base_line.len().abs_diff(pure_line.len()) < 3);
            self.col = self.col.min((text.encoding.exclusive)(pure_line, self.col));
        }

        assert!(
            row_count >= self.row,
            "Row value should be at most, row_count"
        );

        if self.row == row_count {
            br_indexes.insert_indexes(self.row, [br_indexes.last_row()].into_iter());
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
