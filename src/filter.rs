// これはwrapはされるけどstreamの状態でfilterするわけではない
// 本当はasyncのほうがいいかも?
pub trait Filter<'a>: Send {
    type Context: 'a;

    fn predicate(&self, ctx: &Self::Context, input: &str) -> bool;
}

pub struct ClosureFilter<'a, Context, F>(F, std::marker::PhantomData<&'a Context>)
where
    F: Fn(&Context, &str) -> bool,
    Context: 'a;

impl<'a, Context, F> ClosureFilter<'a, Context, F>
where
    F: Fn(&Context, &str) -> bool,
    Context: 'a,
{
    #[allow(dead_code)]
    pub fn new(f: F) -> Self {
        Self(f, std::marker::PhantomData)
    }
}

impl<'a, Context, F> Filter<'a> for ClosureFilter<'a, Context, F>
where
    F: Fn(&Context, &str) -> bool + Send,
    Context: 'a + Sync,
{
    type Context = Context;

    fn predicate(&self, ctx: &Self::Context, input: &str) -> bool {
        (self.0)(ctx, input)
    }
}
