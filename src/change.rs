use std::fmt::Debug;

use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

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
