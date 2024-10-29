use std::fmt::Debug;

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
}

#[derive(Clone, Copy, Debug)]
pub struct GridIndex {
    pub row: usize,
    pub col: usize,
}
