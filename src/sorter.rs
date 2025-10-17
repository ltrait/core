use std::marker::PhantomData;

pub trait Sorter: Send {
    type Context;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering;
}

pub struct ClosureSorter<Context, F>(F, PhantomData<Context>)
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering;

impl<Context, F> ClosureSorter<Context, F>
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering,
{
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

impl<Context, F> Sorter for ClosureSorter<Context, F>
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering + Send,
    Context: Sync + Send,
{
    type Context = Context;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering {
        (self.0)(lhs, rhs, input)
    }
}

pub struct SorterWrapper<SorterContext, SorterT, F, Cushion>
where
    F: Fn(&Cushion) -> SorterContext + Send,
    SorterT: Sorter<Context = SorterContext>,
    SorterContext: Sync,
{
    f: F,
    sorter: SorterT,

    _marker: PhantomData<(SorterContext, Cushion)>,
}

impl<SorterContext, SorterT, F, Cushion> Sorter
    for SorterWrapper<SorterContext, SorterT, F, Cushion>
where
    F: Fn(&Cushion) -> SorterContext + Send,
    SorterT: Sorter<Context = SorterContext>,
    SorterContext: Sync + Send,
    Cushion: Send,
{
    type Context = Cushion;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering {
        (self.sorter).compare(&(self.f)(lhs), &(self.f)(rhs), input)
    }
}

impl<SorterContext, SorterT, F, Cushion> SorterWrapper<SorterContext, SorterT, F, Cushion>
where
    F: Fn(&Cushion) -> SorterContext + Send,
    SorterT: Sorter<Context = SorterContext>,
    SorterContext: Sync,
    Cushion: Send,
{
    pub fn new(sorter: SorterT, transformer: F) -> Self {
        Self {
            f: transformer,
            sorter,
            _marker: PhantomData,
        }
    }
}
