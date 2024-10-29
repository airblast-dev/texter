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
        fast_char_iter(s)
            .nth(nth)
            .expect("Char index should never be out of bounds")
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
            char_oob(char_count, nth)
        }
    }
}

pub mod utf16 {

    pub fn utf16(s: &str, nth: usize) -> usize {
        let mut total_code_points = 0;
        let i = s
            .char_indices()
            .take_while(|(_, c)| {
                total_code_points += c.len_utf16();
                if total_code_points > nth {
                    panic!("we should never receive positions between characters");
                }
                total_code_points <= nth
            })
            .map(|(i, _)| i)
            .last()
            .expect("UTF16 position out of bounds");

        i
    }

    pub fn utf16_exclusive(s: &str, nth: usize) -> usize {
        if nth == s.encode_utf16().count() {
            return s.len();
        }
        let mut total_code_points = 0;
        let i = s
            .char_indices()
            .take_while(|(_, c)| {
                total_code_points += c.len_utf16();
                if total_code_points > nth {
                    panic!("we should never receive positions between characters");
                }
                total_code_points <= nth
            })
            .map(|(i, _)| i)
            .last()
            .expect("UTF16 position out of bounds");

        i
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn char_oob(ch_count: usize, ch_index: usize) -> ! {
    panic!(
        "exclusive char index {ch_index} should never more than character count ({ch_count}) + 1"
    )
}
