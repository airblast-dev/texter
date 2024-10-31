pub mod utf8 {

    use super::char_oob;

    /// Finds the byte index for the
    pub fn utf8(s: &str, nth: usize) -> usize {
        if s.len() <= nth {
            char_oob();
        };
        nth
    }

    pub fn utf8_exclusive(s: &str, nth: usize) -> usize {
        if s.len() < nth {
            char_oob();
        };
        nth
    }
}

pub mod utf16 {
    use super::char_oob;

    use super::char_oob_ex;

    /// Converts UTF16 indexes to UTF8 indexes.
    pub fn utf16(s: &str, nth: usize) -> usize {
        let mut total_code_points = 0;
        if nth == 0 {
            return 0;
        }
        for (utf8_index, utf16_len, utf8_len) in s
            .char_indices()
            .map(|(i, c)| (i, c.len_utf16(), c.len_utf8()))
        {
            total_code_points += utf16_len;
            if total_code_points == nth {
                return utf8_index + utf8_len;
            }
            if total_code_points > nth {
                panic!("UTF16 position should never be between code points");
            }
        }

        char_oob()
    }

    /// Converts UTF16 indexes to UTF8 indexes but also allows code point + 1 to be used in range operations.
    pub fn utf16_exclusive(s: &str, nth: usize) -> usize {
        let mut total_code_points = 0;
        if nth == 0 {
            return 0;
        }
        for (utf8_index, utf16_len, utf8_len) in s
            .char_indices()
            .map(|(i, c)| (i, c.len_utf16(), c.len_utf8()))
        {
            total_code_points += utf16_len;
            if total_code_points == nth {
                return utf8_index + utf8_len;
            }
            if total_code_points > nth {
                panic!("UTF16 position should never be between code points");
            }
        }

        if total_code_points + 1 == nth {
            return nth;
        }

        char_oob_ex()
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn char_oob() -> ! {
    panic!("byte index should never more than byte count")
}

#[cold]
#[inline(never)]
#[track_caller]
fn char_oob_ex() -> ! {
    panic!("exclusive byte index should never more than byte count + 1")
}
