use std::{
    borrow::Cow,
    fmt::Display,
    iter::{FusedIterator, Once},
    ops::RangeBounds,
};

use crate::core::text::Text;

pub trait QueryIter<'a>: Iterator<Item = &'a str> + Clone {}
impl<'a, T: 'a> QueryIter<'a> for T where T: Iterator<Item = &'a str> + FusedIterator + Clone {}

pub trait Queryable: Display {
    type Iter<'a>: QueryIter<'a>
    where
        Self: 'a;
    fn get<RB: RangeBounds<usize>>(&self, r: RB) -> Self::Iter<'_> {
        self.try_get(r)
            .expect("range out of bounds or not on a char boundary")
    }
    fn try_get<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Self::Iter<'_>>;
    fn get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Cow<'_, str> {
        self.try_get_single(r)
            .expect("range out of bounds or not on a char boundary")
    }
    fn try_get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Cow<'_, str>>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Queryable for &str {
    type Iter<'a>
        = Once<&'a str>
    where
        Self: 'a;
    fn get<RB: RangeBounds<usize>>(&self, r: RB) -> Self::Iter<'_> {
        std::iter::once(&self[(r.start_bound().cloned(), r.end_bound().cloned())])
    }

    fn try_get<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Self::Iter<'_>> {
        str::get(self, (r.start_bound().cloned(), r.end_bound().cloned())).map(std::iter::once)
    }

    fn get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Cow<'_, str> {
        let sb = r.start_bound().cloned();
        let eb = r.end_bound().cloned();
        Cow::Borrowed(&self[(sb, eb)])
    }

    fn try_get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Cow<'_, str>> {
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
}

impl Queryable for &Text {
    type Iter<'a>
        = Once<&'a str>
    where
        Self: 'a;
    fn get<RB: RangeBounds<usize>>(&self, r: RB) -> Self::Iter<'_> {
        std::iter::once(&self.text.as_str()[(r.start_bound().cloned(), r.end_bound().cloned())])
    }

    fn try_get<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Self::Iter<'_>> {
        self.text
            .get((r.start_bound().cloned(), r.end_bound().cloned()))
            .map(std::iter::once)
    }

    fn get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Cow<'_, str> {
        let sb = r.start_bound().cloned();
        let eb = r.end_bound().cloned();
        Cow::Borrowed(&self.text.as_str()[(sb, eb)])
    }

    fn try_get_single<RB: RangeBounds<usize>>(&self, r: RB) -> Option<Cow<'_, str>> {
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
}
