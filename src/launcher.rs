use color_eyre::eyre::{OptionExt, Result};

use crate::action::{Action, ActionWrapper};
use crate::filter::{Filter, FilterWrapper};
use crate::generator::{GenWrapper, Generator};
use crate::launcher::batcher::Batcher;
use crate::sorter::{Sorter, SorterWrapper};
use crate::source::{Source, transform_source};
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

/// The add_** functions essentially act as syntactic sugar; they call a Wrapper
/// first and then call the add_raw_ functions. For typical use cases, it is
/// preferable to use the add_** (the non-raw version). However, when using
/// types that further wrap the Wrapper (which are planned to be added in
/// ltrait-extra), it may be more appropriate to use add_raw_.
///
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
        self,
        source: Source<'a, SourceContext>,
        transformer: F,
    ) -> Self
    where
        F: Fn(SourceContext) -> Cusion + Send + 'a,
        SourceContext: 'a,
    {
        self.add_raw_source(transform_source(source, transformer))
    }

    pub fn add_raw_source(mut self, source: Source<'a, Cusion>) -> Self {
        self.batcher.add_raw_source(source);
        self
    }

    pub fn add_filter<FilterContext, FilterT, F>(self, filter: FilterT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> FilterContext + Send + 'a,
        FilterContext: 'a + Sync,
        FilterT: Filter<'a, Context = FilterContext> + 'a,
    {
        self.add_raw_filter(FilterWrapper::new(filter, transformer))
    }

    pub fn add_raw_filter<FilterT>(mut self, filter: FilterT) -> Self
    where
        FilterT: Filter<'a, Context = Cusion> + 'a,
    {
        self.batcher.add_raw_filter(filter);
        self
    }

    pub fn add_sorter<SorterContext, SorterT, F>(self, sorter: SorterT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> SorterContext + Send + 'a,
        SorterContext: 'a + Sync,
        SorterT: Sorter<'a, Context = SorterContext> + 'a,
        Cusion: 'a + Send,
    {
        self.add_raw_sorter(SorterWrapper::new(sorter, transformer))
    }

    pub fn add_raw_sorter<SorterT>(mut self, sorter: SorterT) -> Self
    where
        SorterT: Sorter<'a, Context = Cusion> + 'a,
    {
        self.batcher.add_raw_sorter(sorter);
        self
    }

    pub fn add_action<ActionContext, ActionT, F>(self, action: ActionT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> ActionContext + Send + 'a,
        ActionT: Action<'a, Context = ActionContext> + 'a,
        ActionContext: 'a,
        Cusion: 'a + Sync,
    {
        self.add_raw_action(ActionWrapper::new(action, transformer))
    }

    pub fn add_raw_action<ActionT>(mut self, action: ActionT) -> Self
    where
        ActionT: Action<'a, Context = Cusion> + 'a,
    {
        self.actions.push(Box::new(action));

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

    pub fn add_generator<Item, GenT, F>(self, generator: GenT, transformer: F) -> Self
    where
        Item: 'a,
        F: Fn(Item) -> Cusion + Sync + Send + 'a,
        GenT: Generator<Item = Item> + Sync + Send + 'a,
        Cusion: Sync + 'a,
    {
        self.add_raw_generator(GenWrapper::new(generator, transformer))
    }

    pub(super) fn add_raw_generator<GenT>(mut self, generator: GenT) -> Self
    where
        GenT: Generator<Item = Cusion> + Sync + Send + 'a,
    {
        self.batcher.add_raw_generator(generator);
        self
    }

    pub async fn run(self) -> Result<()> {
        let cusion: Cusion = self
            .ui
            .ok_or_eyre("UI must be set before calling run")?
            .run(self.batcher)
            .await?;

        for ai in self.actions {
            ai.act(&cusion)?;
        }

        Ok(())
    }

    /// If `filter_and` is true and more than one filter is provided,
    /// the launcher will display only entries that satisfy all filter predicates.
    /// The default value is false.
    pub fn filter_and(mut self, flag: bool) -> Self {
        self.batcher.filter_and = flag;
        self
    }

    /// A batch represents the process of retrieving items from all available sources and sorting the filtered items
    /// according to user-specified sorters.
    ///
    /// For performance reasons, the number of items retrieved in a single batch is capped based on the pre-filter count.
    /// The `batch_size` parameter sets this upper limit.
    ///
    /// Note that `batch_size` is determined heuristically and may require tuning for optimal performance; it is recommended
    /// to experiment with different values.
    ///
    /// In most cases, both the source (which returns one item per evaluation) and the generator (which can return multiple items)
    /// contribute to the running count of items in a batch. However, if the final evaluation in a batch comes from a generator
    /// that returns more items than needed to reach the specified limit, all of those items are added to the batch,
    /// potentially exceeding the `batch_size`. The extra items are not saved for subsequent batches.
    ///
    /// When `batch_size` is set to 0, there is no upper limit, and all items are retrieved and processed in a single batch.
    /// The default value is 0.
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batcher.batch_size = batch_size;
        self
    }
}
