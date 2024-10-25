use super::BR_FINDER;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrIndexes(pub(crate) Vec<usize>);

impl<S: AsRef<[usize]>> PartialEq<S> for BrIndexes {
    fn eq(&self, other: &S) -> bool {
        self.0 == other.as_ref()
    }
}

impl BrIndexes {
    pub fn new(s: &str) -> Self {
        let iter = BR_FINDER.find_iter(s.as_bytes());
        let mut byte_indexes = vec![0];
        byte_indexes.extend(iter);
        Self(byte_indexes)
    }

    // The index to the first byte in the row.
    pub fn row_start(&self, row: usize) -> usize {
        // we increment by one if it is not zero since the index points to a break line,
        // and the first row should start at zero.
        self.0[row] + (row != 0) as usize
    }

    pub fn remove_indexes(&mut self, start: usize, end: usize) {
        let start = if start != end { start + 1 } else { return };
        self.0.drain(start..end);
    }

    /// Add an offset to all rows after the provided row number including itself.
    pub fn add_offsets(&mut self, row: usize, by: usize) {
        self.0[row.max(1)..].iter_mut().for_each(|bi| *bi += by);
    }

    /// Sub an offset to all rows after the provided row number including itself.
    pub fn sub_offsets(&mut self, row: usize, by: usize) {
        self.0[row.max(1)..].iter_mut().for_each(|bi| *bi -= by);
    }
}
