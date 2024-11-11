pub(crate) type EncodingFn = fn(&str, usize) -> usize;

pub(crate) const UTF8: EncodingFn = utf8::exclusive;

pub(crate) const UTF16: EncodingFn = utf16::exclusive;

pub(crate) const UTF32: EncodingFn = utf32::exclusive;

pub mod utf8 {

    use super::{between_code_points, char_oob};

    #[inline]
    pub(super) fn exclusive(s: &str, nth: usize) -> usize {
        if s.len() < nth {
            char_oob(s.len(), nth);
        };
        if !s.is_char_boundary(nth) {
            between_code_points();
        }
        nth
    }
}

pub mod utf16 {
    use super::{between_code_points, char_oob};

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

        char_oob(total_code_points, nth)
    }
}

mod utf32 {
    use super::char_oob;

    #[inline]
    pub(super) fn exclusive(s: &str, nth: usize) -> usize {
        let mut counter = 0;
        let Some((i, _)) = s.char_indices().inspect(|_| counter += 1).nth(nth) else {
            if counter + 1 == nth {
                return s.len();
            }
            char_oob(counter, nth);
        };

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
