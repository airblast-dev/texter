use std::{fmt::Display, num::NonZeroUsize};

/// A type alias for the libraries result type. ([`Result<(), Error>`])
pub type Result<T> = std::result::Result<T, Error>;

/// The error type returned upon failed conversions and edits across the library.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    OutOfBoundsRow { max: usize, current: usize },
    InBetweenCharBoundries { encoding: Encoding },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    UTF8,
    UTF16,
    UTF32,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfBoundsRow { max, current } => {
                write!(f, "Current max row index is {max}, {current} was provided.")
            }
            Self::InBetweenCharBoundries { encoding } => {
                write!(
                    f,
                    "Provided column position is between char boundries for {encoding:?}."
                )
            }
        }
    }
}

impl Error {
    #[inline]
    pub(crate) fn oob_row(row_count: NonZeroUsize, current: usize) -> Self {
        Self::OutOfBoundsRow {
            max: row_count.get() - 1,
            current,
        }
    }
}

impl std::error::Error for Error {}
