use super::BR_FINDER;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BrIndexes(pub(crate) Vec<usize>);

// Mainly used to remove duplicate code in tests.
impl<S: AsRef<[usize]>> PartialEq<S> for BrIndexes {
    fn eq(&self, other: &S) -> bool {
        self.0 == other.as_ref()
    }
}

impl BrIndexes {
    #[inline]
    pub fn new(s: &str) -> Self {
        let iter = BR_FINDER.find_iter(s.as_bytes());
        let mut byte_indexes = vec![0];
        byte_indexes.extend(iter);
        Self(byte_indexes)
    }

    /// The index to the first byte in the row.
    #[inline]
    pub fn row_start(&self, row: usize) -> usize {
        // we increment by one if it is not zero since the index points to a break line,
        // and the first row should start at zero.
        self.0[row] + (row != 0) as usize
    }

    #[inline]
    pub fn insert_indexes<I: Iterator<Item = usize>>(&mut self, at: usize, indexes: I) {
        self.0.splice(at..at, indexes);
    }

    /// Removes the indexes between start and end, not including start, but including end.
    #[inline]
    pub fn remove_indexes(&mut self, start: usize, end: usize) {
        if start + 1 > end {
            return;
        }
        self.0.drain(start + 1..=end);
    }

    pub fn replace_indexes<I: Iterator<Item = usize>>(
        &mut self,
        start: usize,
        end: usize,
        replacement: I,
    ) {
        self.0.splice(start + 1..=end, replacement);
    }

    /// Add an offset to all rows after the provided row number excluding itself.
    #[inline]
    pub fn add_offsets(&mut self, row: usize, by: usize) {
        self.0[row..].iter_mut().skip(1).for_each(|bi| *bi += by);
    }

    /// Sub an offset to all rows after the provided row number excluding itself.
    #[inline]
    pub fn sub_offsets(&mut self, row: usize, by: usize) {
        self.0[row..].iter_mut().skip(1).for_each(|bi| *bi -= by);
    }

    /// Returns true if the provided row number is the last row.
    #[inline]
    pub fn is_last_row(&self, row: usize) -> bool {
        assert!(row < self.0.len());
        self.0.len() == row + 1
    }
}

#[cfg(test)]
mod tests {
    use crate::texter::br_indexes::BrIndexes;

    const S: &str = "ads\nasdas\n\n\nasdad\n\nasdasd\nasd\na\n";

    #[test]
    fn new() {
        let br = BrIndexes::new(S);
        assert_eq!(br.0, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);
    }

    #[test]
    fn row_start() {
        let br = BrIndexes::new(S);
        assert_eq!(br.row_start(0), 0);
        assert_eq!(br.row_start(1), 4);
        assert_eq!(br.row_start(2), 10);
        assert_eq!(br.row_start(3), 11);
        assert_eq!(br.row_start(4), 12);
        assert_eq!(br.row_start(5), 18);
        assert_eq!(br.row_start(6), 19);
        assert_eq!(br.row_start(7), 26);
        assert_eq!(br.row_start(8), 30);
        assert_eq!(br.row_start(9), 32);
    }

    #[test]
    #[should_panic]
    fn row_start_oob() {
        let br = BrIndexes::new(S);
        br.row_start(10);
    }

    #[test]
    fn remove_indexes_all() {
        let mut br = BrIndexes::new(S);
        br.remove_indexes(0, 9);
        assert_eq!(br, [0]);
    }

    #[test]
    fn remove_indexes_from_middle() {
        let mut br = BrIndexes::new(S);
        br.remove_indexes(1, 9);
        assert_eq!(br, [0, 3]);

        let mut br = BrIndexes::new(S);
        br.remove_indexes(3, 5);
        assert_eq!(br, [0, 3, 9, 10, 18, 25, 29, 31]);

        let mut br = BrIndexes::new(S);
        br.remove_indexes(6, 7);
        assert_eq!(br, [0, 3, 9, 10, 11, 17, 18, 29, 31]);
    }

    #[test]
    fn remove_indexes_same_row() {
        let mut br = BrIndexes::new(S);
        br.remove_indexes(0, 0);
        assert_eq!(br, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);

        let mut br = BrIndexes::new(S);
        br.remove_indexes(5, 5);
        assert_eq!(br, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);

        let mut br = BrIndexes::new(S);
        br.remove_indexes(9, 9);
        assert_eq!(br, [0, 3, 9, 10, 11, 17, 18, 25, 29, 31]);
    }

    #[test]
    fn remove_indexes_last_row() {
        let mut br = BrIndexes::new(S);
        br.remove_indexes(4, 9);
        assert_eq!(br, [0, 3, 9, 10, 11]);

        let mut br = BrIndexes::new(S);
        br.remove_indexes(0, 9);
        assert_eq!(br, [0]);
    }

    #[test]
    fn add_offsets() {
        let mut br = BrIndexes::new(S);
        br.add_offsets(3, 10);
        assert_eq!(br.0, [0, 3, 9, 10, 21, 27, 28, 35, 39, 41]);
    }

    #[test]
    fn sub_offsets() {
        let mut br = BrIndexes::new(S);
        br.sub_offsets(0, 2);
        assert_eq!(br.0, [0, 1, 7, 8, 9, 15, 16, 23, 27, 29]);
    }

    #[test]
    fn is_last_row() {
        let br = BrIndexes::new(S);
        assert!(!br.is_last_row(0));
        assert!(!br.is_last_row(1));
        assert!(!br.is_last_row(2));
        assert!(br.is_last_row(9));
    }

    #[test]
    #[should_panic]
    fn is_last_row_oob() {
        let br = BrIndexes::new(S);
        assert!(br.is_last_row(10));
    }
}
