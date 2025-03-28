use std::{iter::FusedIterator, num::NonZeroUsize};

use super::lines::FastEOL;

#[derive(Debug, PartialEq, Eq)]
pub struct EolIndexes(pub Vec<usize>);

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
    ///
    /// Returns None if the nth row does not exist.
    #[inline(always)]
    pub fn row_start(&self, row: usize) -> Option<usize> {
        // we increment by one if it is not zero since the index points to a break line,
        // and the first row should start at zero.
        self.0.get(row).map(|rs| rs + (row != 0) as usize)
    }

    /// Inserts the provided indexes at the provided position.
    ///
    /// Returns a range to get a slice of the inserted indexes.
    #[inline]
    pub fn insert_indexes<I: Iterator<Item = usize>>(
        &mut self,
        at: usize,
        indexes: I,
    ) -> std::ops::Range<usize> {
        // A slightly more efficient way to insert multiple values in a Vec.
        // Can be thought of as inserting using Vec::splice with optimal cases.
        let old_len = self.row_count().get();
        self.0.extend(indexes);
        let new_len = self.row_count().get();
        self.0[at..].rotate_right(new_len - old_len);
        at..at + (new_len - old_len)
    }

    /// Insert the provided index at the position.
    pub fn insert_index(&mut self, at: usize, index: usize) {
        self.0.insert(at, index);
    }

    /// Removes the indexes between start and end, not including start, but including end.
    ///
    /// Does nothing if start + 1 > end.
    #[inline]
    pub fn remove_indexes(&mut self, start: usize, end: usize) {
        if start + 1 > end {
            return;
        }
        self.0.drain(start + 1..=end);
    }

    /// Replace the indexes excluding start and including end.
    ///
    /// Internally is similar to [`Vec::splice`], but with ideal cases and some other optimizations
    /// since we are dealing with integers.
    ///
    /// In case use in unsafe code, the uninitialized portion may contain arbitrary values, as the
    /// uninitialized section is used as scratch memory. This ofcourse does not concern any safe
    /// code.
    ///
    /// # Panics
    ///
    /// Panics if start > end or end > row_count.
    #[inline]
    pub fn replace_indexes<I>(
        &mut self,
        start: usize,
        end: usize,
        mut replacement: I,
    ) -> std::ops::Range<usize>
    where
        I: Iterator<Item = usize> + FusedIterator,
    {
        assert!(start <= end);
        assert!(end <= self.row_count().get());

        // replace as many the existing values in the range as possible
        let replacing_len = end - start;
        let i = self.0[start + 1..end + 1]
            .iter_mut()
            .zip(replacement.by_ref())
            .map(|(old, new)| *old = new)
            .count();

        // calculate the slice start bound that will be rotated
        let rotate_start = if i < replacing_len {
            end - (replacing_len - i) + 1
        } else {
            end + 1
        };

        let cur_len = self.row_count().get();

        // add any remaining value to the end
        // these will be rotated to their correct position below
        // we do this to avoid shifting the values multiple times
        // with this we end up shifting only once
        self.0.extend(replacement);
        let insert_count = self.row_count().get() - cur_len;
        // no values were appended to the end, meaning we either have fully filled the replacing
        // range, or we have values we need to remove
        if insert_count == 0 {
            // i is always <= replacing_len
            self.0[start + 1 + i..].rotate_left(replacing_len - i);

            let new_len = self.row_count().get() - (replacing_len - i);

            // the set len below should never grow the vec
            // debug_assert is probably better but better be safe than sorry
            assert!(new_len <= self.0.len());

            // SAFETY: safety requirements of set_len require that the range is initialized which is already
            // done. This branch should never grow the vec, the assertion above checks that
            //
            // this is slightly faster than truncating as no checks or drops need to be performed.
            // instead all is dealt with when the vec is dropped.
            unsafe {
                self.0.set_len(new_len);
            }
        } else {
            self.0[rotate_start..].rotate_right(insert_count);
        }

        start + 1..start + 1 + insert_count
    }

    /// Add an offset to all rows after the provided row number excluding itself.
    ///
    /// If the row > row_count the function returns early.
    #[inline(always)]
    pub(crate) fn add_offsets(&mut self, row: usize, by: usize) {
        if row >= self.row_count().get() {
            return;
        }
        self.0[row + 1..].iter_mut().for_each(|bi| *bi += by);
    }

    /// Sub an offset to all rows after the provided row number excluding itself.
    ///
    /// If the row > row_count the function returns early.
    #[inline(always)]
    pub(crate) fn sub_offsets(&mut self, row: usize, by: usize) {
        if row >= self.row_count().get() {
            return;
        }
        self.0[row + 1..].iter_mut().for_each(|bi| *bi -= by);
    }

    /// Returns true if the provided row index is for the last row.
    ///
    /// # Panics
    ///
    /// When the buffer contains less than 1 element.
    #[inline(always)]
    pub fn is_last_row(&self, row: usize) -> bool {
        let len = self.row_count();
        len.get() - 1 == row
    }

    /// Get the number of rows present.
    ///
    /// # Panics
    ///
    /// When the buffer contains less than 1 element.
    #[inline(always)]
    pub fn row_count(&self) -> NonZeroUsize {
        let len = self.0.len();
        let Some(len) = NonZeroUsize::new(len) else {
            no_row();
        };

        len
    }

    /// Get the first byte index of the last row.
    ///
    /// # Panics
    ///
    /// When the buffer contains less than 1 element.
    #[inline(always)]
    pub fn last_row_start(&self) -> usize {
        self.row_start(self.row_count().get() - 1).unwrap()
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
    use crate::core::eol_indexes::EolIndexes;

    const S: &str = "ads\nasdas\n\n\nasdad\n\nasdasd\nasd\na\n";

    #[test]
    fn new() {
        let br = EolIndexes::new(S);
        assert_eq!(br.0, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);
    }

    #[test]
    fn row_start() {
        let br = EolIndexes::new(S);
        assert_eq!(br.row_start(0), Some(0));
        assert_eq!(br.row_start(1), Some(4));
        assert_eq!(br.row_start(2), Some(10));
        assert_eq!(br.row_start(3), Some(11));
        assert_eq!(br.row_start(4), Some(12));
        assert_eq!(br.row_start(5), Some(18));
        assert_eq!(br.row_start(6), Some(19));
        assert_eq!(br.row_start(7), Some(26));
        assert_eq!(br.row_start(8), Some(30));
        assert_eq!(br.row_start(9), Some(32));
        assert_eq!(br.row_start(10), None);
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
