use std::iter::FusedIterator;

use memchr::{memchr2_iter, Memchr2};

#[derive(Clone, Debug)]
pub(crate) struct FastEOL<'a> {
    haystack: &'a [u8],
    iter: Memchr2<'a>,
    r: Option<usize>,
}

const RC: u8 = b'\r';
const BR: u8 = b'\n';

impl<'a> FastEOL<'a> {
    pub(crate) fn new(haystack: &'a str) -> Self {
        let iter = memchr2_iter(RC, BR, haystack.as_bytes());
        Self {
            iter,
            haystack: haystack.as_bytes(),
            r: None,
        }
    }
}

impl Iterator for FastEOL<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
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
            _ => unreachable!("the byte value should only be a line break or carriage return"),
        }
    }
}

impl FusedIterator for FastEOL<'_> {}

#[cfg(test)]
mod tests {
    use super::FastEOL;

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
}
