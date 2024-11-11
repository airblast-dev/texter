pub(crate) type EncodingFn = fn(&str, usize) -> usize;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Encoding {
    pub inclusive: EncodingFn,
    pub exclusive: EncodingFn,
}

pub(crate) const UTF8: fn(&str, usize) -> usize = utf8::exclusive;

pub(crate) const UTF16: fn(&str, usize) -> usize = utf16::exclusive;

pub(crate) const UTF32: fn(&str, usize) -> usize = utf32::exclusive;

pub mod utf8 {

    use super::{between_code_points, char_oob, char_oob_ex};

    /// Finds the byte index for the
    #[inline]
    pub(super) fn inclusive(s: &str, nth: usize) -> usize {
        if s.len() < nth {
            char_oob(s.len(), nth);
        };
        if !s.is_char_boundary(nth) {
            between_code_points();
        }
        nth
    }

    #[inline]
    pub(super) fn exclusive(s: &str, nth: usize) -> usize {
        if s.len() < nth {
            char_oob_ex(s.len(), nth);
        };
        if !s.is_char_boundary(nth) {
            between_code_points();
        }
        nth
    }
}

pub mod utf16 {
    use super::between_code_points;
    use super::char_oob;

    use super::char_oob_ex;

    /// Converts UTF16 indexes to UTF8 indexes.
    pub(super) fn inclusive(s: &str, nth: usize) -> usize {
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

        char_oob(total_code_points, nth)
    }

    /// Converts UTF16 indexes to UTF8 indexes but also allows code point + 1 to be used in range operations.
    pub(super) fn exclusive(s: &str, nth: usize) -> usize {
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

        char_oob_ex(total_code_points, nth)
    }
}

mod utf32 {
    use super::{char_oob, char_oob_ex};

    #[inline]
    pub(super) fn inclusive(s: &str, nth: usize) -> usize {
        let mut counter = 0;
        let Some((i, _)) = s.char_indices().inspect(|_| counter += 1).nth(nth) else {
            char_oob(counter, nth);
        };

        i
    }

    #[inline]
    pub(super) fn exclusive(s: &str, nth: usize) -> usize {
        let mut counter = 0;
        let Some((i, _)) = s.char_indices().inspect(|_| counter += 1).nth(nth) else {
            if counter + 1 == nth {
                return s.len();
            }
            char_oob_ex(counter, nth);
        };

        i
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn char_oob(byte_index: usize, byte_count: usize) -> ! {
    panic!("byte index should never more than byte count -> {byte_index} <= {byte_count}")
}

#[cold]
#[inline(never)]
#[track_caller]
fn char_oob_ex(byte_index: usize, byte_count: usize) -> ! {
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
