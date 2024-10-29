use std::{fmt::Debug, hash::Hash};

use crate::utils::string_ext::fast_char_iter;

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
