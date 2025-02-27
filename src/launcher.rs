use crate::action::Action;
use crate::filter::Filter;
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
    /// Add a source to `self`, builder
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
        self.batcher.add_action(action, transformer);

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

    pub async fn run(self) {
        self.ui.unwrap().run(self.batcher).await;
    }

    // settings

    pub fn filter_and(mut self, flag: bool) -> Self {
        self.batcher.filter_and(flag);
        self
    }

    #[cfg(feature = "parallel")]
    pub fn par_sort(mut self, flag: bool) -> Self {
        self.batcher.par_sort(flag);
        self
    }

    #[cfg(feature = "parallel")]
    pub fn par_filter(mut self, flag: bool) -> Self {
        self.batcher.par_filter(flag);
        self
    }
}
