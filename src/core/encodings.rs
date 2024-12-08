use crate::error::Error;

pub(crate) type EncodingFn = fn(&str, usize) -> Result<usize, Error>;
pub(crate) type EncodingFns = [EncodingFn; 2];

pub(crate) const UTF8: EncodingFns = [utf8::to, utf8::from];

pub(crate) const UTF16: EncodingFns = [utf16::to, utf16::from];

pub(crate) const UTF32: EncodingFns = [utf32::to, utf32::from];

pub mod utf8 {

    use crate::error::{Encoding, Error};

    #[inline]
    pub(super) fn to(s: &str, nth: usize) -> Result<usize, Error> {
        if !s.is_char_boundary(nth) {
            return Err(Error::InBetweenCharBoundries {
                encoding: Encoding::UTF8,
            });
        }

        Ok(nth.min(s.len()))
    }

    #[inline]
    pub(super) fn from(s: &str, nth: usize) -> Result<usize, Error> {
        to(s, nth)
    }
}

pub mod utf16 {
    use crate::error::{Encoding, Error};

    /// Converts UTF16 indexes to UTF8 indexes but also allows code point + 1 to be used in range operations.
    pub(super) fn to(s: &str, nth: usize) -> Result<usize, Error> {
        let mut total_code_points = 0;
        if nth == 0 {
            return Ok(0);
        }
        for (utf8_index, utf8_len, utf16_len) in s
            .char_indices()
            .map(|(i, c)| (i, c.len_utf8(), c.len_utf16()))
        {
            if total_code_points > nth {
                return Err(Error::InBetweenCharBoundries {
                    encoding: Encoding::UTF16,
                });
            }
            total_code_points += utf16_len;
            if total_code_points == nth {
                return Ok(utf8_index + utf8_len);
            }
        }

        Ok(nth.min(s.len()))
    }

    pub(super) fn from(s: &str, col: usize) -> Result<usize, Error> {
        let mut utf8_len = 0;
        let mut utf16_len = 0;
        for c in s.chars() {
            if utf8_len == col {
                break;
            }
            utf8_len += c.len_utf8();
            utf16_len += c.len_utf16();
        }

        Ok(utf16_len)
    }
}

mod utf32 {
    use crate::error::Error;

    #[inline]
    pub(super) fn to(s: &str, nth: usize) -> Result<usize, Error> {
        Ok(s.char_indices().map(|(i, _)| i).nth(nth).unwrap_or(s.len()))
    }

    pub(super) fn from(s: &str, nth: usize) -> Result<usize, Error> {
        let mut len_utf8 = 0;
        let mut i = 0;
        for c in s.chars() {
            if nth == len_utf8 {
                break;
            }
            i += 1;

            len_utf8 += c.len_utf8();
        }

        Ok(i)
    }
}
