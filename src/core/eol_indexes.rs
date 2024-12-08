use crate::error::Error;

use super::lines::FastEOL;

#[derive(Debug, PartialEq, Eq)]
pub struct EolIndexes(pub(crate) Vec<usize>);

impl Default for EolIndexes {
    fn default() -> Self {
        Self(vec![0])
    }
}

impl Clone for EolIndexes {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }

    // The derived impl does not add this, and instead creates a new Vec instead of reusing the
    // allocation.
    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
    }
}

// Mainly used to remove duplicate code in tests.
impl<S: AsRef<[usize]>> PartialEq<S> for EolIndexes {
    fn eq(&self, other: &S) -> bool {
        self.0 == other.as_ref()
    }
}

impl EolIndexes {
    #[inline]
    pub fn new(s: &str) -> Self {
        let iter = FastEOL::new(s);
        let mut byte_indexes = vec![0];
        byte_indexes.extend(iter);
        Self(byte_indexes)
    }

    /// The index to the first byte in the row.
    #[inline(always)]
    pub fn row_start(&self, row: usize) -> Result<usize, Error> {
        // we increment by one if it is not zero since the index points to a break line,
        // and the first row should start at zero.
        let row_start = self.0.get(row).ok_or(Error::OutOfBoundsRow {
            max: self.row_count() - 1,
            current: row,
        })?;
        Ok(row_start + (row != 0) as usize)
    }

    /// Inserts the provided indexes at the provided position.
    ///
    /// Returns a slice of the inserted indexes.
    #[inline]
    pub(crate) fn insert_indexes<I: Iterator<Item = usize>>(
        &mut self,
        at: usize,
        indexes: I,
    ) -> std::ops::Range<usize> {
        // A slightly more efficient way to insert multiple values in a Vec.
        // Can be thought of as inserting using Vec::splice with optimal cases.
        let old_len = self.0.len();
        self.0.extend(indexes);
        let new_len = self.0.len();
        self.0[at..].rotate_right(new_len - old_len);
        at..at + (new_len - old_len)
    }

    pub(crate) fn insert_index(&mut self, at: usize, index: usize) {
        self.0.insert(at, index);
    }

    /// Removes the indexes between start and end, not including start, but including end.
    #[inline]
    pub(crate) fn remove_indexes(&mut self, start: usize, end: usize) {
        if start + 1 > end {
            return;
        }
        self.0.drain(start + 1..=end);
    }

    #[inline]
    pub(crate) fn replace_indexes<I: Iterator<Item = usize>>(
        &mut self,
        start: usize,
        end: usize,
        mut replacement: I,
    ) -> std::ops::Range<usize> {
        let replacing_len = end - start;
        let mut i = 0;
        for (index, new) in (1..replacing_len + 1).zip(replacement.by_ref()) {
            self.0[start + index] = new;
            i = index;
        }

        let rotate_start = if i < replacing_len {
            end - (replacing_len - i) + 1
        } else {
            end + 1
        };

        let cur_len = self.0.len();
        self.0.extend(replacement);
        let insert_count = self.0.len() - cur_len;
        if insert_count == 0 {
            self.0[start + 1 + i..].rotate_left(replacing_len - i);
            self.0.truncate(self.0.len() - (replacing_len - i));
        } else {
            self.0[rotate_start..].rotate_right(insert_count);
        }

        start + 1..start + 1 + insert_count
    }

    /// Add an offset to all rows after the provided row number excluding itself.
    #[inline(always)]
    pub(crate) fn add_offsets(&mut self, row: usize, by: usize) {
        if row >= self.0.len() {
            return;
        }
        self.0[row + 1..].iter_mut().for_each(|bi| *bi += by);
    }

    /// Sub an offset to all rows after the provided row number excluding itself.
    #[inline(always)]
    pub(crate) fn sub_offsets(&mut self, row: usize, by: usize) {
        if row >= self.0.len() {
            return;
        }
        self.0[row + 1..].iter_mut().for_each(|bi| *bi -= by);
    }

    /// Returns true if the provided row index is for the last row.
    #[inline(always)]
    pub fn is_last_row(&self, row: usize) -> bool {
        let len = self.0.len();
        len - 1 == row
    }

    #[inline(always)]
    pub fn row_count(&self) -> usize {
        let len = self.0.len();
        if len == 0 {
            no_row();
        }
        len
    }

    #[inline(always)]
    pub fn last_row(&self) -> Result<usize, Error> {
        // Cannot panic, Self::row_count should always return at least 1.
        self.row_start(self.row_count() - 1)
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn no_row() -> ! {
    panic!("the row count should never be less than one")
}

#[cfg(test)]
mod tests {
    use crate::{core::eol_indexes::EolIndexes, error::Error};

    const S: &str = "ads\nasdas\n\n\nasdad\n\nasdasd\nasd\na\n";

    #[test]
    fn new() {
        let br = EolIndexes::new(S);
        assert_eq!(br.0, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);
    }

    #[test]
    fn row_start() {
        let br = EolIndexes::new(S);
        assert_eq!(br.row_start(0), Ok(0));
        assert_eq!(br.row_start(1), Ok(4));
        assert_eq!(br.row_start(2), Ok(10));
        assert_eq!(br.row_start(3), Ok(11));
        assert_eq!(br.row_start(4), Ok(12));
        assert_eq!(br.row_start(5), Ok(18));
        assert_eq!(br.row_start(6), Ok(19));
        assert_eq!(br.row_start(7), Ok(26));
        assert_eq!(br.row_start(8), Ok(30));
        assert_eq!(br.row_start(9), Ok(32));
        assert_eq!(
            br.row_start(10),
            Err(Error::OutOfBoundsRow {
                max: 9,
                current: 10
            })
        );
    }

    #[test]
    fn remove_indexes_all() {
        let mut br = EolIndexes::new(S);
        br.remove_indexes(0, 9);
        assert_eq!(br, [0]);
    }

    #[test]
    fn remove_indexes_from_middle() {
        let mut br = EolIndexes::new(S);
        br.remove_indexes(1, 9);
        assert_eq!(br, [0, 3]);

        let mut br = EolIndexes::new(S);
        br.remove_indexes(3, 5);
        assert_eq!(br, [0, 3, 9, 10, 18, 25, 29, 31]);

        let mut br = EolIndexes::new(S);
        br.remove_indexes(6, 7);
        assert_eq!(br, [0, 3, 9, 10, 11, 17, 18, 29, 31]);
    }

    #[test]
    fn remove_indexes_same_row() {
        let mut br = EolIndexes::new(S);
        br.remove_indexes(0, 0);
        assert_eq!(br, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);

        let mut br = EolIndexes::new(S);
        br.remove_indexes(5, 5);
        assert_eq!(br, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);

        let mut br = EolIndexes::new(S);
        br.remove_indexes(9, 9);
        assert_eq!(br, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);
    }

    #[test]
    fn remove_indexes_last_row() {
        let mut br = EolIndexes::new(S);
        br.remove_indexes(4, 9);
        assert_eq!(br, [0, 3, 9, 10, 11]);

        let mut br = EolIndexes::new(S);
        br.remove_indexes(0, 9);
        assert_eq!(br, [0]);
    }

    #[test]
    fn add_offsets() {
        let mut br = EolIndexes::new(S);
        br.add_offsets(3, 10);
        assert_eq!(br.0, [0, 3, 9, 10, 21, 27, 28, 35, 39, 41]);
    }

    #[test]
    fn sub_offsets() {
        let mut br = EolIndexes::new(S);
        br.sub_offsets(0, 2);
        assert_eq!(br.0, [0, 1, 7, 8, 9, 15, 16, 23, 27, 29]);
    }

    #[test]
    fn is_last_row() {
        let br = EolIndexes::new(S);
        assert!(!br.is_last_row(0));
        assert!(!br.is_last_row(1));
        assert!(!br.is_last_row(2));
        assert!(br.is_last_row(9));
    }

    #[test]
    #[should_panic]
    fn is_last_row_oob() {
        let br = EolIndexes::new(S);
        assert!(br.is_last_row(10));
    }
}
