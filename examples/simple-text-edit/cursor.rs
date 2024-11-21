use texter::{
    change::GridIndex,
    updateables::{ChangeContext, Updateable},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Cursor {
    /// The shared row index.
    ///
    /// Used when constructing the cursor position and byte position.
    pub row: usize,
    /// The byte position that corresponds to the cursors position.
    pub col_byte: usize,
    /// The cursors current position.
    pub col_cursor: usize,
}

impl Updateable for Cursor {
    fn update(&mut self, ctx: texter::updateables::UpdateContext) {
        match ctx.change {
            // The goal of this is to move the cursor right after the inserted content.
            ChangeContext::Insert {
                text,
                inserted_br_indexes,
                position,
            } => {
                let Some(last_row_start) = inserted_br_indexes.last().map(|i| i + 1) else {
                    // The inserted text does not contain any new lines.
                    // Simply offset the current values.
                    self.col_cursor += text.chars().count();
                    self.col_byte += text.len();
                    return;
                };

                let start_byte = ctx.breaklines.row_start(position.row) + position.col;

                // Move increment the row by the number of new rows inserted.
                self.row += inserted_br_indexes.len();

                // By adding the start byte and the length of the text we get the new end byte.
                let end_byte = start_byte + text.len();

                // Move the byte position by subtracting the new end position with the last rows
                // start.
                self.col_byte = end_byte - last_row_start;

                // Move the cursor to after the end of the text.
                // For example with an insertion of "123\n345" we wouldnt want to move the cursor
                // to the right by 7 cells. Instead we increment the row by the number of new lines
                // then move the cursors column by the inserted texts length after the last EOL.
                //
                // - (EOL being End of Line such as "\r\n" or "\n")
                // - The +1 is only to move it after the text like the editor nano
                self.col_cursor = text[end_byte - last_row_start + 1..].chars().count();
            }
            // After deleting some text we want to point to the position of the deleted character
            // in nano (editor) like fashion.
            ChangeContext::Delete { start, .. } => {
                // Changing the row only matters when deleting beyond the start of a line, where
                // we wrap to the end of the previous line.
                self.row = start.row;
                self.col_byte = start.col;
                let start_byte = ctx.old_breaklines.row_start(start.row);
                self.col_cursor = ctx.old_str[start_byte..start_byte + start.col]
                    .chars()
                    .count();
            }
            _ => unimplemented!("Any other change variant is not used by the demo."),
        }
    }
}

impl Cursor {
    pub fn byte_pos(&self) -> GridIndex {
        GridIndex {
            row: self.row,
            col: self.col_byte,
        }
    }
}
