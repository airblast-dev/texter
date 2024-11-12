pub(crate) type EncodingFn = fn(&str, usize) -> usize;
pub(crate) type EncodingFns = [EncodingFn; 2];

pub(crate) const UTF8: EncodingFns = [utf8::to, utf8::from];

pub(crate) const UTF16: EncodingFns = [utf16::to, utf16::from];

pub(crate) const UTF32: EncodingFns = [utf32::to, utf32::from];

pub mod utf8 {

    use super::{between_code_points, char_oob};

    #[inline]
    pub(super) fn to(s: &str, nth: usize) -> usize {
        if s.len() < nth {
            char_oob(s.len(), nth);
        };
        if !s.is_char_boundary(nth) {
            between_code_points();
        }
        nth
    }

    #[inline]
    pub(super) fn from(s: &str, nth: usize) -> usize {
        to(s, nth)
    }
}

pub mod utf16 {
    use super::{between_code_points, char_oob};

    /// Converts UTF16 indexes to UTF8 indexes but also allows code point + 1 to be used in range operations.
    pub(super) fn to(s: &str, nth: usize) -> usize {
        let mut total_code_points = 0;
        if nth == 0 {
            return 0;
        }
        for (utf8_index, utf8_len, utf16_len) in s
            .char_indices()
            .map(|(i, c)| (i, c.len_utf8(), c.len_utf16()))
        {
            if total_code_points > nth {
                between_code_points();
            }
            total_code_points += utf16_len;
            if total_code_points == nth {
                return utf8_index + utf8_len;
            }
        }

        if total_code_points + 1 == nth {
            return nth;
        }

        char_oob(total_code_points, nth)
    }

    pub(super) fn from(s: &str, col: usize) -> usize {
        let mut utf8_len = 0;
        let mut utf16_len = 0;
        for c in s.chars() {
            if utf8_len == col {
                break;
            }
            utf8_len += c.len_utf8();
            utf16_len += c.len_utf16();
        }

        utf16_len
    }
}

mod utf32 {
    use super::char_oob;

    #[inline]
    pub(super) fn to(s: &str, nth: usize) -> usize {
        let mut counter = 0;
        let Some((i, _)) = s.char_indices().inspect(|_| counter += 1).nth(nth) else {
            if counter + 1 == nth {
                return s.len();
            }
            char_oob(counter, nth);
        };

        i
    }

    pub(super) fn from(s: &str, nth: usize) -> usize {
        let mut len_utf8 = 0;
        let mut i = 0;
        for c in s.chars() {
            if nth == len_utf8 {
                break;
            }
            i += 1;

            len_utf8 += c.len_utf8();
        }

        i
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn char_oob(byte_index: usize, byte_count: usize) -> ! {
    panic!(
        "exclusive byte index should never more than byte count + 1 -> {byte_index} <= {byte_count} + 1"
    )
}

#[cold]
#[inline(never)]
#[track_caller]
fn between_code_points() {
    panic!("position should never be between code points");
}
