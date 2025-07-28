use color_eyre::Result;
use std::marker::PhantomData;

pub trait Action<'a>: Send + 'a {
    type Context: 'a;

    fn act(&self, ctx: &Self::Context) -> Result<()>;
}

pub struct ClosureAction<'a, Context, F>(F, PhantomData<&'a Context>)
where
    F: Fn(&Context) -> Result<()> + Send + 'a,
    Context: 'a + Sync;

impl<'a, Context, F> ClosureAction<'a, Context, F>
where
    F: Fn(&Context) -> Result<()> + Send + 'a,
    Context: 'a + Sync,
{
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

impl<'a, Context, F> Action<'a> for ClosureAction<'a, Context, F>
where
    F: Fn(&Context) -> Result<()> + Send + 'a,
    Context: 'a + Sync,
{
    type Context = Context;

    fn act(&self, ctx: &Self::Context) -> Result<()> {
        (self.0)(ctx)
    }
}

pub struct ActionWrapper<'a, ActionContext, ActionT, F, Cushion>
where
    F: Fn(&Cushion) -> ActionContext + Send + 'a,
    ActionT: Action<'a, Context = ActionContext>,
    ActionContext: 'a,
    Cushion: 'a + Sync,
{
    f: F,
    action: ActionT,

    _marker: PhantomData<&'a Cushion>,
}

impl<'a, ActionContext, ActionT, F, Cushion> Action<'a>
    for ActionWrapper<'a, ActionContext, ActionT, F, Cushion>
where
    F: Fn(&Cushion) -> ActionContext + Send + 'a,
    ActionT: Action<'a, Context = ActionContext>,
    ActionContext: 'a,
    Cushion: 'a + Sync,
{
    type Context = Cushion;

    fn act(&self, ctx: &Self::Context) -> Result<()> {
        self.action.act(&(self.f)(ctx))
    }
}

impl<'a, ActionContext, ActionT, F, Cushion> ActionWrapper<'a, ActionContext, ActionT, F, Cushion>
where
    F: Fn(&Cushion) -> ActionContext + Send + 'a,
    ActionT: Action<'a, Context = ActionContext>,
    ActionContext: 'a,
    Cushion: 'a + Sync,
{
    pub fn new(action: ActionT, transformer: F) -> Self {
        Self {
            f: transformer,
            action,

            _marker: PhantomData,
        }
    }
}
