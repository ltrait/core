use std::marker::PhantomData;

pub trait Sorter<'a>: Send + 'a {
    type Context: 'a;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering;
}

pub struct ClosureSorter<'a, Context, F>(F, PhantomData<&'a Context>)
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering,
    Context: 'a;

impl<'a, Context, F> ClosureSorter<'a, Context, F>
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering,
    Context: 'a,
{
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

impl<'a, Context, F> Sorter<'a> for ClosureSorter<'a, Context, F>
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering + Send + 'a,
    Context: 'a + Sync,
{
    type Context = Context;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering {
        (self.0)(lhs, rhs, input)
    }
}

pub struct SorterWrapper<'a, SorterContext, SorterT, F, Cushion>
where
    F: Fn(&Cushion) -> SorterContext + Send + 'a,
    SorterT: Sorter<'a, Context = SorterContext>,
    SorterContext: 'a + Sync,
{
    f: F,
    sorter: SorterT,

    _marker: PhantomData<(&'a SorterContext, Cushion)>,
}

impl<'a, SorterContext, SorterT, F, Cushion> Sorter<'a>
    for SorterWrapper<'a, SorterContext, SorterT, F, Cushion>
where
    F: Fn(&Cushion) -> SorterContext + Send + 'a,
    SorterT: Sorter<'a, Context = SorterContext>,
    SorterContext: 'a + Sync,
    Cushion: 'a + Send,
{
    type Context = Cushion;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering {
        (self.sorter).compare(&(self.f)(lhs), &(self.f)(rhs), input)
    }
}

impl<'a, SorterContext, SorterT, F, Cushion> SorterWrapper<'a, SorterContext, SorterT, F, Cushion>
where
    F: Fn(&Cushion) -> SorterContext + Send + 'a,
    SorterT: Sorter<'a, Context = SorterContext>,
    SorterContext: 'a + Sync,
    Cushion: 'a + Send,
{
    pub fn new(sorter: SorterT, transformer: F) -> Self {
        Self {
            f: transformer,
            sorter,
            _marker: PhantomData,
        }
    }
}
