pub mod utf8 {

    use super::char_oob;

    #[inline]
    pub(crate) fn fast_char_iter(s: &str) -> impl use<'_> + Iterator<Item = usize> {
        s.as_bytes()
            .iter()
            .enumerate()
            .filter(|(_, ci)| (**ci as i8) >= -0x40)
            .map(|(bi, _)| bi)
    }
    pub fn utf8(s: &str, nth: usize) -> usize {
        if let Some(i) = fast_char_iter(s).nth(nth) {
            i
        } else {
            char_oob()
        }
    }

    pub fn utf8_exclusive(s: &str, nth: usize) -> usize {
        let mut char_count = 0;
        let nth_byte = fast_char_iter(s).inspect(|_| char_count += 1).nth(nth);
        if let Some(nth_byte) = nth_byte {
            return nth_byte;
        }

        if char_count == nth {
            s.len()
        } else {
            char_oob()
        }
    }
}

pub mod utf16 {
    use crate::texter::encodings::char_oob;

    use super::char_oob_ex;

    pub fn utf16(s: &str, nth: usize) -> usize {
        let mut total_code_points = 0;
        dbg!(nth);
        if nth == 0 && s.is_empty() {
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

    pub fn utf16_exclusive(s: &str, nth: usize) -> usize {
        let mut total_code_points = 0;
        if nth == 0 && s.is_empty() {
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
