// これはwrapはされるけどstreamの状態でfilterするわけではない
// 本当はasyncのほうがいいかも?
use std::marker::PhantomData;

pub trait Filter: Send {
    type Context;

    fn predicate(&self, ctx: &Self::Context, input: &str) -> bool;
}

pub struct ClosureFilter<Context, F>(F, PhantomData<Context>)
where
    F: Fn(&Context, &str) -> bool;

impl<Context, F> ClosureFilter<Context, F>
where
    F: Fn(&Context, &str) -> bool,
{
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

impl<Context, F> Filter for ClosureFilter<Context, F>
where
    F: Fn(&Context, &str) -> bool + Send,
    Context: Sync + Send,
{
    type Context = Context;

    fn predicate(&self, ctx: &Self::Context, input: &str) -> bool {
        (self.0)(ctx, input)
    }
}

pub struct FilterWrapper<FilterContext, FilterT, F, Cushion>
where
    F: Fn(&Cushion) -> FilterContext + Send,
    FilterT: Filter<Context = FilterContext>,
{
    f: F,
    filter: FilterT,

    _marker: PhantomData<(FilterContext, Cushion)>,
}

impl<FilterContext, FilterT, F, Cushion> Filter
    for FilterWrapper<FilterContext, FilterT, F, Cushion>
where
    F: Fn(&Cushion) -> FilterContext + Send,
    FilterT: Filter<Context = FilterContext>,
    FilterContext: Sync + Send,
    Cushion: Send,
{
    type Context = Cushion;

    fn predicate(&self, ctx: &Self::Context, input: &str) -> bool {
        self.filter.predicate(&(self.f)(ctx), input)
    }
}

impl<FilterContext, FilterT, F, Cushion> FilterWrapper<FilterContext, FilterT, F, Cushion>
where
    F: Fn(&Cushion) -> FilterContext + Send,
    FilterT: Filter<Context = FilterContext>,
    FilterContext: Sync,
    Cushion: Send,
{
    pub fn new(filter: FilterT, transformer: F) -> Self {
        Self {
            f: transformer,
            filter,

            _marker: PhantomData,
        }
    }
}
