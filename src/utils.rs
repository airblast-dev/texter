#[inline]
pub(crate) fn trim_eol_from_end(base_line: &str) -> &str {
    // TODO: add checks for the behavior.
    let eol_len = match base_line.as_bytes() {
        // This pattern should come first as the following pattern could cause an EOL to be
        // included.
        [.., b'\r', b'\n'] => 2,
        [.., b'\n' | b'\r'] => 1,
        _ => 0,
    };

    // SAFETY: Since the provided range is based on the length of the str - EOL bytes,
    // worst we can get is an empty str. We only matched on ascii character bytes,
    // and any byte of a multibyte UTF8 character cannot match with any ascii byte.
    let r = unsafe { base_line.get_unchecked(..base_line.len() - eol_len) };

    // Using a debug assert as the line could be long.
    debug_assert!(!r.contains(['\r', '\n']));
    r
}

#[cfg(test)]
mod tests {
    use super::trim_eol_from_end;

    #[test]
    fn non_last_row_trimming() {
        for normalized in [
            "Hello, World",
            "Hello, World\r",
            "Hello, World\r\n",
            "Hello, World\n",
        ]
        .into_iter()
        .map(trim_eol_from_end)
        {
            assert_eq!("Hello, World", normalized);
        }
    }
}
