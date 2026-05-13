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

    /// Check if the error is caused by a single missing newline
    ///
    /// Some LSP servers and clients may assume an extra newline exists at the end.
    /// In the majority of these cases the correct fix is to append an empty line to the string.
    /// This function returns [`true`] if the error can be fixed/ignored by appending a newline.
    #[inline]
    pub fn is_missing_newline(self) -> bool {
        if let Self::OutOfBoundsRow { max, current } = self {
            max + 1 == current
        } else {
            false
        }
    }
}

impl std::error::Error for Error {}
