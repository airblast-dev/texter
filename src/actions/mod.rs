use crate::change::{ActionKind, Actionable, Change, GridIndex};

pub struct DeletePreviousChar(pub GridIndex);
impl Actionable for DeletePreviousChar {
    fn to_change<'a>(
        &'a mut self,
        text: &crate::core::text::Text,
    ) -> crate::change::ActionKind<'a> {
        let row = text.get_row(self.0.row);
        let start = if self.0.col > 0 {
            let char_start = row[..self.0.col]
                .chars()
                .next_back()
                .map(|c| c.len_utf8())
                .unwrap();

            GridIndex {
                row: self.0.row,
                col: self.0.col - char_start,
            }
        } else if self.0.row > 0 {
            let prev_row = self.0.row - 1;
            let prev_row_end = text.get_row(prev_row).len();

            GridIndex {
                row: prev_row,
                col: prev_row_end,
            }
        } else {
            GridIndex { row: 0, col: 0 }
        };

        ActionKind::Once(Change::Delete { start, end: self.0 })
    }
}
