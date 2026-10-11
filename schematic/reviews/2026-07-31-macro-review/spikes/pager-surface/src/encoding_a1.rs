//! Encoding A1 (compared, not chosen): the same capability traits as the
//! real pager, but each knob lives in an `impl` block bounded by its trait.
//! A missing knob is then E0599 ("method exists but its trait bounds were not
//! satisfied"), which ignores `#[diagnostic::on_unimplemented]`. Surface only.

use std::marker::PhantomData;

use crate::spec::{HasPrev, KeepAliveKnob, PageNumber, PageSizeKnob, Paginated, ResumeAfter, Scroll};

pub struct Pager1<E>(PhantomData<E>);
pub struct Pages1<E>(PhantomData<E>);

impl<E: Paginated> Pager1<E> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
    pub fn pages(self) -> Pages1<E> {
        Pages1(PhantomData)
    }
}

impl<E: Paginated> Default for Pager1<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: PageSizeKnob> Pager1<E> {
    pub fn page_size(self, _n: u32) -> Self {
        self
    }
}

impl<E: Paginated<Kind = PageNumber>> Pager1<E> {
    pub fn page(self, _n: u64) -> Self {
        self
    }
}

impl<E: ResumeAfter> Pager1<E> {
    pub fn after(self, _c: &str) -> Self {
        self
    }
}

impl<E: KeepAliveKnob> Pager1<E> {
    pub fn keep_alive(self, _d: &str) -> Self {
        self
    }
}

impl<E: HasPrev> Pages1<E> {
    pub fn prev(&mut self) {}
}

impl<E: Paginated<Kind = Scroll>> Pages1<E> {
    pub fn close(&mut self) {}
}
