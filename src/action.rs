pub trait Action<'a>: std::marker::Send {
    type Context: 'a;

    fn act(&self, ctx: Self::Context);
}

pub struct ClosureAction<'a, Context, F>(F, std::marker::PhantomData<&'a Context>)
where
    F: Fn(Context),
    Context: 'a;

impl<'a, Context, F> ClosureAction<'a, Context, F>
where
    F: Fn(Context),
    Context: 'a,
{
    #[allow(dead_code)]
    pub fn new(f: F) -> Self {
        Self(f, std::marker::PhantomData)
    }
}

impl<'a, Context, F> Action<'a> for ClosureAction<'a, Context, F>
where
    F: Fn(Context) + Send,
    Context: 'a + Sync,
{
    type Context = Context;

    fn act(&self, ctx: Self::Context) {
        (self.0)(ctx);
    }
}
