use color_eyre::Result;
use std::marker::PhantomData;

pub trait Action: Send {
    type Context;

    fn act(&self, ctx: &Self::Context) -> Result<()>;
}

pub struct ClosureAction<Context, F>(F, PhantomData<Context>)
where
    F: Fn(&Context) -> Result<()> + Send,
    Context: Sync;

impl<Context, F> ClosureAction<Context, F>
where
    F: Fn(&Context) -> Result<()> + Send,
    Context: Sync,
{
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

impl<Context, F> Action for ClosureAction<Context, F>
where
    F: Fn(&Context) -> Result<()> + Send,
    Context: Sync + Send,
{
    type Context = Context;

    fn act(&self, ctx: &Self::Context) -> Result<()> {
        (self.0)(ctx)
    }
}

pub struct ActionWrapper<ActionContext, ActionT, F, Cushion>
where
    F: Fn(&Cushion) -> ActionContext + Send,
    ActionT: Action<Context = ActionContext>,
    Cushion: Sync,
{
    f: F,
    action: ActionT,

    _marker: PhantomData<Cushion>,
}

impl<ActionContext, ActionT, F, Cushion> Action
    for ActionWrapper<ActionContext, ActionT, F, Cushion>
where
    F: Fn(&Cushion) -> ActionContext + Send,
    ActionT: Action<Context = ActionContext>,
    Cushion: Sync + Send,
{
    type Context = Cushion;

    fn act(&self, ctx: &Self::Context) -> Result<()> {
        self.action.act(&(self.f)(ctx))
    }
}

impl<ActionContext, ActionT, F, Cushion> ActionWrapper<ActionContext, ActionT, F, Cushion>
where
    F: Fn(&Cushion) -> ActionContext + Send,
    ActionT: Action<Context = ActionContext>,
    Cushion: Sync,
{
    pub fn new(action: ActionT, transformer: F) -> Self {
        Self {
            f: transformer,
            action,

            _marker: PhantomData,
        }
    }
}
