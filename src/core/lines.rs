use std::iter::FusedIterator;

use memchr::{memchr2_iter, Memchr2};

use crate::utils::trim_eol_from_end;

#[derive(Clone, Debug)]
pub(crate) struct FastEOL<'a> {
    haystack: &'a [u8],
    iter: Memchr2<'a>,
    r: Option<usize>,
    last_found: usize,
}

const RC: u8 = b'\r';
const BR: u8 = b'\n';

impl<'a> FastEOL<'a> {
    pub(crate) fn new(haystack: &'a str) -> Self {
        let iter = memchr2_iter(RC, BR, haystack.as_bytes());
        Self {
            iter,
            haystack: haystack.as_bytes(),
            last_found: 0,
            r: None,
        }
    }
}

impl Iterator for FastEOL<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
        self.last_found = next.unwrap_or_default();
        let Some(mut n) = next else {
            return self.r.take();
        };

        match self.haystack[n] {
            RC => {
                if let Some(r) = self.r.as_mut() {
                    if *r + 1 == n {
                        std::mem::swap(&mut n, r);
                        return next;
                    }
                }

                if self.haystack.get(n + 1).is_some_and(|mbr| *mbr == BR) {
                    self.iter.next();
                    Some(n + 1)
                } else {
                    next
                }
            }
            BR => {
                self.r = None;
                next
            }
            // adding this to a cold path, or swapping it out for its unsafe variant worsens
            // performance for some reason.
            _ => unreachable!("the byte value should only be a line break or carriage return"),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.r.is_some() as usize,
            Some(self.haystack.len() - self.last_found),
        )
    }
}

impl FusedIterator for FastEOL<'_> {}

#[derive(Clone, Debug)]
pub struct TextLines<'a> {
    lf_indexes: &'a [usize],
    s: &'a str,
    cursor: usize,
}

impl<'a> TextLines<'a> {
    pub(crate) fn new(s: &'a str, lfs: &'a [usize]) -> TextLines<'a> {
        if let Some(l) = lfs.last() {
            // panic if the content is out of sync
            // we do not do full checks as it makes things very slow
            // this only checks if the content is out of sync in an obvious way
            assert!(lfs.is_sorted());
            assert!(*l < s.len() || *l == 0);
        }
        Self {
            lf_indexes: lfs,
            s,
            cursor: 0,
        }
    }
}

impl<'a> Iterator for TextLines<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.nth(0)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let mut start = *self.lf_indexes.get(self.cursor + n)?;

        start += (start != 0) as usize;
        let end = self
            .lf_indexes
            .get(self.cursor + n + 1)
            .copied()
            .unwrap_or(self.s.len());

        self.cursor += n + 1;
        Some(trim_eol_from_end(&self.s[start..end]))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let b = self.lf_indexes.len() - self.cursor;
        (b, Some(b))
    }
}

impl FusedIterator for TextLines<'_> {}
impl ExactSizeIterator for TextLines<'_> {}

#[cfg(test)]
mod tests {
    use super::{FastEOL, TextLines};

    #[test]
    fn br() {
        let hs = "123\n45678\n910";
        let lines: Vec<_> = FastEOL::new(hs).collect();
        assert_eq!(lines, [3, 9]);
    }

    #[test]
    fn r() {
        let hs = "123\r45678\r910";
        let lines: Vec<_> = FastEOL::new(hs).collect();
        assert_eq!(lines, [3, 9]);
    }

    #[test]
    fn rbr() {
        let hs = "123\r\n45678\r\n910";
        let lines: Vec<_> = FastEOL::new(hs).collect();
        assert_eq!(lines, [4, 11]);
    }

    #[test]
    fn rbr_mix() {
        let hs = "\r\r\r\n123\r45678\r\n910\n123\r123\n123123\n\r\r";
        let lines: Vec<_> = FastEOL::new(hs).collect();
        assert_eq!(lines, [0, 1, 3, 7, 14, 18, 22, 26, 33, 34, 35]);
    }

    #[test]
    fn text_lines() {
        let s = "abc\n\r123\n\nbasdasd\n\n\n";
        let indexes = &[0, 3, 4, 8, 9, 17, 18, 19];
        let mut lines = TextLines::new(s, indexes);
        assert_eq!(lines.next(), Some("abc"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some("123"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some("basdasd"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some(""));
    }

    #[test]
    #[allow(clippy::iter_nth_zero)]
    fn text_lines_nth() {
        let s = "abc\n\r123\n\nbasdasd\n\n\n";
        let indexes = &[0, 3, 4, 8, 9, 17, 18, 19];
        let mut lines = TextLines::new(s, indexes);

        assert_eq!(lines.nth(0), Some("abc"));
        assert_eq!(lines.nth(0), Some(""));
        assert_eq!(lines.nth(0), Some("123"));
        assert_eq!(lines.nth(0), Some(""));
        assert_eq!(lines.nth(0), Some("basdasd"));
        assert_eq!(lines.nth(0), Some(""));
        assert_eq!(lines.nth(0), Some(""));
        assert_eq!(lines.nth(0), Some(""));
    }

    #[test]
    fn text_lines_skip() {
        let s = "abc\n\r123\n\nbasdasd\n\n\n";
        let indexes = &[0, 3, 4, 8, 9, 17, 18, 19];
        let mut lines = TextLines::new(s, indexes).skip(2);
        assert_eq!(lines.next(), Some("123"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some("basdasd"));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), Some(""));
    }
}
