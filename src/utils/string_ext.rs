pub(crate) trait InsertManyStr {
    fn insert_many<const N: usize>(&mut self, pos: usize, items: [&str; N]);
}

impl InsertManyStr for String {
    fn insert_many<const N: usize>(&mut self, pos: usize, items: [&str; N]) {
        assert!(pos <= self.len());
        let added_len = items.into_iter().map(str::len).sum();
        self.reserve(added_len);
        let len = self.len();

        unsafe {
            // calling as_mut_vec here gets rid of miri warnings
            let ins_ptr = self.as_mut_vec().as_mut_ptr().add(pos);
            std::ptr::copy(ins_ptr, ins_ptr.add(added_len), len - pos);
            let mut offset = 0;
            items.into_iter().for_each(|s| {
                std::ptr::copy_nonoverlapping(s.as_ptr(), ins_ptr.add(offset), s.len());
                offset += s.len();
            });

            self.as_mut_vec().set_len(len + added_len);
        };
    }
}

#[cfg(test)]
mod tests {
    use super::InsertManyStr;

    #[test]
    fn prepend_many() {
        let mut s = String::from("Hello, World!");
        s.insert_many(0, ["A, B, C", "D"]);

        assert_eq!(s, "A, B, CDHello, World!");
    }

    #[test]
    fn insert_many() {
        let mut s = String::from("Hello, World!");
        s.insert_many(5, ["A, B, C", "D"]);

        assert_eq!(s, "HelloA, B, CD, World!");
    }

    #[test]
    fn append_many() {
        let mut s = String::from("Hello, World!");
        s.insert_many(13, ["A, B, C", "D"]);

        assert_eq!(s, "Hello, World!A, B, CD");
    }

    #[test]
    #[should_panic]
    fn oob_panics() {
        let mut s = String::from("Hello, World!");
        s.insert_many(14, ["A, B, C", "D"]);

        assert_eq!(s, "Hello, World!A, B, CD");
    }
}
