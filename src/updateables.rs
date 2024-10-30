use std::cmp::Ordering;

use tree_sitter::{InputEdit, Point, Tree};

use crate::{change::GridIndex, core::br_indexes::BrIndexes};

pub enum ChangeContext<'a> {
    Insert {
        inserted_br_indexes: &'a [usize],
        position: GridIndex,
        text: &'a str,
    },
    Delete {
        start: GridIndex,
        end: GridIndex,
    },
    Replace {
        start: GridIndex,
        end: GridIndex,
        text: &'a str,
        inserted_br_indexes: &'a [usize],
    },
    ReplaceFull {
        text: &'a str,
    },
}

pub struct UpdateContext<'a> {
    /// A context change that is being used to update the [`Text`].
    pub change: ChangeContext<'a>,
    /// The new breakline positions.
    pub breaklines: &'a BrIndexes,
    /// The old breakline positions.
    pub old_breaklines: &'a BrIndexes,
    /// The old string.
    pub old_str: &'a str,
}

pub trait Updateable {
    fn update(&mut self, ctx: UpdateContext<'_>);
}

impl Updateable for () {
    fn update(&mut self, _: UpdateContext<'_>) {}
}

impl Updateable for Tree {
    fn update(&mut self, ctx: UpdateContext<'_>) {
        self.edit(&edit_from_ctx(ctx));
    }
}

fn edit_from_ctx(ctx: UpdateContext<'_>) -> InputEdit {
    let old_br = ctx.old_breaklines;
    let new_br = ctx.breaklines;
    match ctx.change {
        ChangeContext::Insert {
            inserted_br_indexes,
            position,
            text,
        } => {
            let pos = position;
            let start = old_br.row_start(pos.row) + pos.col;
            let start_point = Point {
                row: pos.row,
                column: pos.col,
            };
            let end_point = {
                let row = pos.row + inserted_br_indexes.len();
                let col = inserted_br_indexes
                    .last()
                    .copied()
                    .map(|li| text.len() - li - start)
                    .unwrap_or(old_br.row_start(pos.row) + text.len());
                Point { row, column: col }
            };
            InputEdit {
                start_byte: start,
                old_end_byte: start,
                new_end_byte: start + text.len(),
                start_position: start_point,
                old_end_position: start_point,
                new_end_position: end_point,
            }
        }
        ChangeContext::Delete { start, end } => {
            let start_point = Point {
                row: start.row,
                column: end.col,
            };
            let start_byte = old_br.row_start(start.row) + start.col;
            let end_byte = old_br.row_start(end.row) + end.col;
            InputEdit {
                start_position: start_point,
                new_end_position: start_point,
                start_byte,
                new_end_byte: start_byte,
                old_end_byte: end_byte,
                old_end_position: Point {
                    row: end.row,
                    column: end.col,
                },
            }
        }
        ChangeContext::Replace {
            start,
            end,
            text,
            inserted_br_indexes,
        } => {
            let start_byte = old_br.row_start(start.row) + start.col;
            let old_end_byte = old_br.row_start(end.row) + end.col;
            let new_end_byte = {
                let old_text_len = old_end_byte - start_byte;
                match old_text_len.cmp(&text.len()) {
                    Ordering::Greater => old_end_byte - (old_text_len - text.len()),
                    Ordering::Less => old_end_byte + (text.len() - old_text_len),
                    Ordering::Equal => old_end_byte,
                }
            };
            let (new_end_row, new_end_col) = {
                match inserted_br_indexes.last() {
                    Some(last) => (
                        start.row + inserted_br_indexes.len(),
                        text.len() - (last - start_byte),
                    ),
                    None => (start.row, start.col + text.len()),
                }
            };
            InputEdit {
                start_byte,
                old_end_byte,
                start_position: Point {
                    row: start.row,
                    column: start.col,
                },
                old_end_position: Point {
                    row: end.row,
                    column: end.col,
                },
                new_end_byte,
                new_end_position: Point {
                    row: new_end_row,
                    column: new_end_col,
                },
            }
        }
        ChangeContext::ReplaceFull { text } => InputEdit {
            start_byte: 0,
            old_end_byte: ctx.old_str.len(),
            new_end_byte: text.len(),
            start_position: Point { row: 0, column: 0 },
            old_end_position: Point {
                row: old_br.row_count() - 1,
                column: ctx.old_str.len() - old_br.last_row(),
            },
            new_end_position: Point {
                row: new_br.row_count() - 1,
                column: text.len() - new_br.last_row(),
            },
        },
    }
}
