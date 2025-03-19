use std::marker::PhantomData;

pub trait Sorter<'a>: Send + 'a {
    type Context: 'a;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering;
}

pub struct ClosureSorter<'a, Context, F>(F, std::marker::PhantomData<&'a Context>)
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering,
    Context: 'a;

impl<'a, Context, F> ClosureSorter<'a, Context, F>
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering,
    Context: 'a,
{
    pub fn new(f: F) -> Self {
        Self(f, std::marker::PhantomData)
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

pub struct SorterWrapper<'a, SorterContext, SorterT, F, Cusion>
where
    F: Fn(&Cusion) -> SorterContext + Send + 'a,
    SorterT: Sorter<'a, Context = SorterContext>,
    SorterContext: 'a + Sync,
{
    f: F,
    sorter: SorterT,

    _sorter_context: PhantomData<&'a SorterContext>,
    _cusion: PhantomData<Cusion>,
}

impl<'a, SorterContext, SorterT, F, Cusion> Sorter<'a>
    for SorterWrapper<'a, SorterContext, SorterT, F, Cusion>
where
    F: Fn(&Cusion) -> SorterContext + Send + 'a,
    SorterT: Sorter<'a, Context = SorterContext>,
    SorterContext: 'a + Sync,
    Cusion: 'a + Send,
{
    type Context = Cusion;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering {
        (self.sorter).compare(&(self.f)(lhs), &(self.f)(rhs), input)
    }
}

impl<'a, SorterContext, SorterT, F, Cusion> SorterWrapper<'a, SorterContext, SorterT, F, Cusion>
where
    F: Fn(&Cusion) -> SorterContext + Send + 'a,
    SorterT: Sorter<'a, Context = SorterContext>,
    SorterContext: 'a + Sync,
    Cusion: 'a + Send,
{
    pub fn new(sorter: SorterT, transformer: F) -> Self {
        Self {
            f: transformer,
            sorter,
            _sorter_context: PhantomData,
            _cusion: PhantomData,
        }
    }
}
