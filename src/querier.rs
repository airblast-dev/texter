use std::{borrow::Cow, fmt::Display, iter::FusedIterator, ops::RangeBounds};

use crate::core::text::Text;

pub trait QueryIter<'a>: Iterator<Item = &'a str> + Clone {}
impl<'a, T: 'a> QueryIter<'a> for T where T: Iterator<Item = &'a str> + FusedIterator + Clone {}

pub trait Queryable: Display {
    fn get<RB: RangeBounds<usize>>(&self, r: RB) -> Option<impl QueryIter>;
    fn get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Cow<'_, str>> {
        self.get(r).map(Cow::from_iter)
    }

    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Queryable for &str {
    fn get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Cow<'_, str>> {
        let sb = r.start_bound().cloned();
        let eb = r.end_bound().cloned();
        str::get(self, (sb, eb)).map(Cow::Borrowed)
    }

    fn len(&self) -> usize {
        str::len(self)
    }

    fn is_empty(&self) -> bool {
        str::is_empty(self)
    }

    fn get<RB: RangeBounds<usize>>(&self, r: RB) -> Option<impl QueryIter> {
        str::get(self, (r.start_bound().cloned(), r.end_bound().cloned()))
            .map(std::iter::once)
            .map(Iterator::fuse)
    }
}

impl Queryable for &Text {
    fn get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Cow<'_, str>> {
        let sb = r.start_bound().cloned();
        let eb = r.end_bound().cloned();
        self.text.as_str().get((sb, eb)).map(Cow::Borrowed)
    }

    fn len(&self) -> usize {
        self.text.len()
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn get<RB: RangeBounds<usize>>(&self, r: RB) -> Option<impl QueryIter> {
        self.text
            .get((r.start_bound().cloned(), r.end_bound().cloned()))
            .map(std::iter::once)
            .map(Iterator::fuse)
    }
}
