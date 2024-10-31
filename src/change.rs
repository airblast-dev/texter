use std::fmt::Debug;

use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

use crate::core::br_indexes::BrIndexes;

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
    pub(crate) fn normalize(&self, text: &mut String, br_indexes: &mut BrIndexes) {
        // TODO: clamp column value to not allow EOL values.
        let grid_index = match self {
            Change::Delete { end, .. } => end,
            Change::Insert { at, .. } => at,
            Change::Replace { end, .. } => end,
            Change::ReplaceFull(_) => return,
        };

        let row_count = br_indexes.row_count();

        assert!(
            row_count >= grid_index.row,
            "Row value should be at most, row_count"
        );

        if grid_index.row == row_count {
            br_indexes.insert_indexes(grid_index.row, [br_indexes.last_row()].into_iter());
            text.push('\n');
        }
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
