use tree_sitter::{InputEdit, Point, Tree};

use crate::{change::GridIndex, core::br_indexes::BrIndexes};

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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
    fn update(&mut self, ctx: UpdateContext);
}

impl Updateable for () {
    fn update(&mut self, _: UpdateContext) {}
}

impl Updateable for Tree {
    fn update(&mut self, ctx: UpdateContext) {
        self.edit(&edit_from_ctx(ctx));
    }
}

impl<'a, T> Updateable for T
where
    T: 'a + FnMut(UpdateContext),
{
    fn update(&mut self, ctx: UpdateContext) {
        self(ctx)
    }
}

fn edit_from_ctx(ctx: UpdateContext) -> InputEdit {
    let old_br = ctx.old_breaklines;
    let new_br = ctx.breaklines;
    match ctx.change {
        ChangeContext::Delete { start, end } => {
            let start_byte = old_br.row_start(start.row) + start.col;
            let end_byte = old_br.row_start(end.row) + end.col;

            InputEdit {
                start_position: start.into(),
                old_end_position: end.into(),
                new_end_position: start.into(),
                start_byte,
                old_end_byte: end_byte,
                new_end_byte: start_byte,
            }
        }
        ChangeContext::Insert {
            inserted_br_indexes,
            position,
            text,
        } => {
            let start_byte = old_br.row_start(position.row) + position.col;
            let new_end_byte = start_byte + text.len();
            InputEdit {
                start_byte,
                old_end_byte: start_byte,
                new_end_byte,
                start_position: position.into(),
                old_end_position: position.into(),
                new_end_position: Point {
                    row: position.row + inserted_br_indexes.len(),
                    column: inserted_br_indexes
                        .last()
                        .map(|bri| text.len() - (bri - start_byte))
                        .unwrap_or(text.len())
                        + position.col,
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
            InputEdit {
                start_byte,
                start_position: start.into(),
                old_end_position: end.into(),
                old_end_byte,
                new_end_byte: start_byte + text.len(),
                new_end_position: {
                    if let [.., last] = inserted_br_indexes {
                        Point {
                            row: start.row + inserted_br_indexes.len(),
                            column: text.len() - (last - start_byte),
                        }
                    } else {
                        Point {
                            row: start.row,
                            column: start.col + text.len(),
                        }
                    }
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
