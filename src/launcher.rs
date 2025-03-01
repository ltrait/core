use std::marker::PhantomData;

use crate::action::Action;
use crate::filter::Filter;
use crate::generator::Generator;
use crate::launcher::batcher::Batcher;
use crate::sorter::Sorter;
use crate::source::Source;
use crate::ui::UI;

pub mod batcher;

pub struct Launcher<'a, Cusion, UIT, UIContext>
where
    UIT: UI<'a, Context = UIContext> + 'a,
    UIContext: 'a + Send,
    Cusion: 'a + Sync,
{
    batcher: Batcher<'a, Cusion, UIContext>,

    actions: Vec<Box<dyn Action<'a, Context = Cusion> + 'a>>,
    ui: Option<UIT>,
}

impl<'a, Cusion, UIT, UIContext> Default for Launcher<'a, Cusion, UIT, UIContext>
where
    UIT: UI<'a, Context = UIContext> + 'a,
    UIContext: 'a + Send,
    Cusion: 'a + Sync,
{
    fn default() -> Self {
        Self {
            batcher: batcher::Batcher::default(),
            actions: vec![],
            ui: None,
        }
    }
}
/// note: use functions such as
///
/// * `std::convert::identity`
/// * `std::convert::Into::into`
///
/// as the transformer function
impl<'a, Cusion, UIT, UIContext> Launcher<'a, Cusion, UIT, UIContext>
where
    UIT: UI<'a, Context = UIContext> + Sync + 'a,
    UIContext: 'a + Send,
    Cusion: 'a + Send + Sync,
{
    pub fn add_source<SourceContext, F>(
        mut self,
        source: Source<'a, SourceContext>,
        transformer: F,
    ) -> Self
    where
        F: Fn(SourceContext) -> Cusion + Send + 'a,
        SourceContext: 'a,
    {
        self.batcher.add_source(source, transformer);
        self
    }

    pub fn add_filter<FilterContext, FilterT, F>(mut self, filter: FilterT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> FilterContext + Send + 'a,
        FilterContext: 'a + Sync,
        FilterT: Filter<'a, Context = FilterContext> + 'a,
    {
        self.batcher.add_filter(filter, transformer);
        self
    }

    pub fn add_sorter<SorterContext, SorterT, F>(mut self, sorter: SorterT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> SorterContext + Send + 'a,
        SorterContext: 'a + Sync,
        SorterT: Sorter<'a, Context = SorterContext> + 'a,
        Cusion: 'a + Send,
    {
        self.batcher.add_sorter(sorter, transformer);

        self
    }

    pub fn add_action<ActionContext, ActionT, F>(mut self, action: ActionT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> ActionContext + Send + 'a,
        ActionT: Action<'a, Context = ActionContext> + 'a,
        ActionContext: 'a,
        Cusion: 'a + Sync,
    {
        struct ActionWrapper<'a, ActionContext, ActionT, F, Cusion>
        where
            F: Fn(&Cusion) -> ActionContext + Send + 'a,
            ActionT: Action<'a, Context = ActionContext>,
            ActionContext: 'a,
            Cusion: 'a + Sync,
        {
            f: F,
            action: ActionT,

            _cusion: PhantomData<&'a Cusion>,
        }

        impl<'a, ActionContext, ActionT, F, Cusion> ActionWrapper<'a, ActionContext, ActionT, F, Cusion>
        where
            F: Fn(&Cusion) -> ActionContext + Send + 'a,
            ActionT: Action<'a, Context = ActionContext>,
            ActionContext: 'a,
            Cusion: 'a + Sync,
        {
            fn new(action: ActionT, transformer: F) -> Self {
                Self {
                    f: transformer,
                    action,
                    _cusion: PhantomData,
                }
            }
        }

        impl<'a, ActionContext, ActionT, F, Cusion> Action<'a>
            for ActionWrapper<'a, ActionContext, ActionT, F, Cusion>
        where
            F: Fn(&Cusion) -> ActionContext + Send + 'a,
            ActionT: Action<'a, Context = ActionContext>,
            ActionContext: 'a,
            Cusion: 'a + std::marker::Sync,
        {
            type Context = Cusion;

            fn act(&self, ctx: Self::Context) {
                self.action.act((self.f)(&ctx));
            }
        }
        self.actions
            .push(Box::new(ActionWrapper::new(action, transformer)));

        self
    }

    pub fn set_ui<F>(mut self, ui: UIT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> UIContext + Send + Sync + 'a,
    {
        self.ui = Some(ui);
        self.batcher.cusion_to_ui = Some(Box::new(transformer));
        self
    }

    pub fn add_generator<Item, GenT, F>(mut self, generator: GenT, transformer: F) -> Self
    where
        Item: 'a,
        F: Fn(Item) -> Cusion + Sync + 'a,
        GenT: Generator<Item = Item> + Sync + 'a,
        Cusion: Sync + 'a,
    {
        self.batcher.add_generator(generator, transformer);
        self
    }

    pub async fn run(self) {
        self.ui.unwrap().run(self.batcher).await;
    }

    /// If `filter_and` is true and more than one filter is provided,
    /// the launcher will display only entries that satisfy all filter predicates.
    /// The default value is false.
    pub fn filter_and(mut self, flag: bool) -> Self {
        self.batcher.filter_and = flag;
        self
    }

    /// A batch refers to the process of retrieving items from a source (or any other source) and sorting the filtered items.
    /// For performance reasons, the number of items retrieved in a single batch may be limited based on the count of items before filtering.
    ///
    /// If `batch_size` is set to 0, all items are retrieved and displayed in one batch.
    /// The default value is 0.
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batcher.batch_size = batch_size;
        self
    }

    #[cfg(feature = "parallel")]
    pub fn par_sort(mut self, flag: bool) -> Self {
        self.batcher.par_sort = flag;
        self
    }

    #[cfg(feature = "parallel")]
    pub fn par_filter(mut self, flag: bool) -> Self {
        self.batcher.par_filter = flag;
        self
    }
}
