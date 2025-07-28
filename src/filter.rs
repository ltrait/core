// これはwrapはされるけどstreamの状態でfilterするわけではない
// 本当はasyncのほうがいいかも?
use std::marker::PhantomData;

pub trait Filter<'a>: Send + 'a {
    type Context: 'a;

    fn predicate(&self, ctx: &Self::Context, input: &str) -> bool;
}

pub struct ClosureFilter<'a, Context, F>(F, PhantomData<&'a Context>)
where
    F: Fn(&Context, &str) -> bool,
    Context: 'a;

impl<'a, Context, F> ClosureFilter<'a, Context, F>
where
    F: Fn(&Context, &str) -> bool,
    Context: 'a,
{
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

impl<'a, Context, F> Filter<'a> for ClosureFilter<'a, Context, F>
where
    F: Fn(&Context, &str) -> bool + Send + 'a,
    Context: 'a + Sync,
{
    type Context = Context;

    fn predicate(&self, ctx: &Self::Context, input: &str) -> bool {
        (self.0)(ctx, input)
    }
}

pub struct FilterWrapper<'a, FilterContext, FilterT, F, Cushion>
where
    F: Fn(&Cushion) -> FilterContext + Send + 'a,
    FilterT: Filter<'a, Context = FilterContext>,
    FilterContext: 'a,
{
    f: F,
    filter: FilterT,

    _marker: PhantomData<(&'a FilterContext, Cushion)>,
}

impl<'a, FilterContext, FilterT, F, Cushion> Filter<'a>
    for FilterWrapper<'a, FilterContext, FilterT, F, Cushion>
where
    F: Fn(&Cushion) -> FilterContext + Send + 'a,
    FilterT: Filter<'a, Context = FilterContext>,
    FilterContext: 'a + Sync,
    Cushion: 'a + Send,
{
    type Context = Cushion;

    fn predicate(&self, ctx: &Self::Context, input: &str) -> bool {
        self.filter.predicate(&(self.f)(ctx), input)
    }
}

impl<'a, FilterContext, FilterT, F, Cushion> FilterWrapper<'a, FilterContext, FilterT, F, Cushion>
where
    F: Fn(&Cushion) -> FilterContext + Send + 'a,
    FilterT: Filter<'a, Context = FilterContext>,
    FilterContext: 'a + Sync,
    Cushion: 'a + Send,
{
    pub fn new(filter: FilterT, transformer: F) -> Self {
        Self {
            f: transformer,
            filter,

            _marker: PhantomData,
        }
    }
}
