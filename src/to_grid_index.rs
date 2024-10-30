use lsp_types::TextDocumentContentChangeEvent;

use crate::change::{Change, GridIndex};

trait ToChange {
    fn to_change(&self, s: &str) -> Change;
}

#[cfg(feature = "lsp-types")]
impl ToChange for TextDocumentContentChangeEvent {
    fn to_change(&self, _: &str) -> Change {
        let Some(range) = self.range else {
            return Change::ReplaceFull(self.text.clone());
        };

        if self.text.is_empty() {
            return Change::Delete {
                start: GridIndex {
                    row: range.start.line as usize,
                    col: range.start.character as usize,
                },
                end: GridIndex {
                    row: range.end.line as usize,
                    col: range.end.character as usize,
                },
            };
        }

        if range.start == range.end {
            return Change::Insert {
                at: GridIndex {
                    row: range.start.line as usize,
                    col: range.start.character as usize,
                },
                text: self.text.clone(),
            };
        }

        return Change::Replace {
            start: GridIndex {
                row: range.start.line as usize,
                col: range.start.character as usize,
            },
            end: GridIndex {
                row: range.end.line as usize,
                col: range.end.character as usize,
            },
            text: self.text.clone(),
        };
    }
}
