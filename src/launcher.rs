use crate::action::Action;
use crate::filter::Filter;
use crate::sorter::Sorter;
use crate::source::Source;

use std::marker::PhantomData;

pub struct Launcher<'a, Cusion> {
    actions: Vec<Box<dyn Action<'a, Context = Cusion> + 'a>>,
    filters: Vec<Box<dyn Filter<'a, Context = Cusion> + 'a>>,
    sorters: Vec<Box<dyn Sorter<'a, Context = Cusion> + 'a>>,
    sources: Vec<Source<'a, Cusion>>,

    /// if `filter_and` and number of filters is greater than 1,
    /// the launcher will show you entries where all of the filter predicates are true.
    /// the default is false.
    filter_and: bool,

    // TODO: impl
    #[cfg(feature = "parallel")]
    par_sort: bool,
    #[cfg(feature = "parallel")]
    par_filter: bool,
}

impl<Cusion> Default for Launcher<'_, Cusion> {
    fn default() -> Self {
        Self {
            sources: Vec::default(),
            filters: Vec::default(),
            sorters: Vec::default(),
            actions: Vec::default(),

            filter_and: false,

            #[cfg(feature = "parallel")]
            par_sort: false,
            #[cfg(feature = "parallel")]
            par_filter: false,
        }
    }
}

/// note: use functions such as
///
/// * `std::convert::identity`
/// * `std::convert::Into::into`
///
/// as the transformer function
impl<'a, Cusion> Launcher<'a, Cusion> {
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
        fn transform_source<'a, Cusion, SourceContext, F>(
            source: Source<'a, SourceContext>,
            f: F,
        ) -> Source<'a, Cusion>
        where
            SourceContext: 'a,
            F: Fn(SourceContext) -> Cusion + Send + 'a,
        {
            use tokio_stream::StreamExt as _;

            Box::pin(source.map(f))
        }

        self.sources.push(transform_source(source, transformer));
        self
    }

    pub fn add_filter<FilterContext, FilterT, F>(mut self, filter: FilterT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> FilterContext + Send + 'a,
        FilterContext: 'a,
        FilterT: Filter<'a, Context = FilterContext> + 'a,
        Cusion: 'a,
    {
        struct FilterWrapper<'a, FilterContext, FilterT, F, Cusion>
        where
            F: Fn(&Cusion) -> FilterContext + Send + 'a,
            FilterT: Filter<'a, Context = FilterContext>,
            FilterContext: 'a,
        {
            f: F,
            filter: FilterT,

            _filter_context: PhantomData<&'a FilterContext>,
            _cusion: PhantomData<Cusion>,
        }

        impl<'a, FilterContext, FilterT, F, Cusion> FilterWrapper<'a, FilterContext, FilterT, F, Cusion>
        where
            F: Fn(&Cusion) -> FilterContext + Send + 'a,
            FilterT: Filter<'a, Context = FilterContext>,
            FilterContext: 'a,
            Cusion: 'a,
        {
            fn new(filter: FilterT, transformer: F) -> Self {
                Self {
                    f: transformer,
                    filter,

                    _filter_context: PhantomData,
                    _cusion: PhantomData,
                }
            }
        }

        impl<'a, FilterContext, FilterT, F, Cusion> Filter<'a>
            for FilterWrapper<'a, FilterContext, FilterT, F, Cusion>
        where
            F: Fn(&Cusion) -> FilterContext + Send + 'a,
            FilterT: Filter<'a, Context = FilterContext>,
            FilterContext: 'a,
            Cusion: 'a,
        {
            type Context = Cusion;

            fn predicate(&self, ctx: &Self::Context, input: &str) -> bool {
                self.filter.predicate(&(self.f)(ctx), input)
            }
        }

        self.filters
            .push(Box::new(FilterWrapper::new(filter, transformer)));

        self
    }

    pub fn add_sorter<SorterContext, SorterT, F>(mut self, sorter: SorterT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> SorterContext + Send + 'a,
        SorterContext: 'a,
        SorterT: Sorter<'a, Context = SorterContext> + 'a,
        Cusion: 'a,
    {
        struct SorterWrapper<'a, SorterContext, SorterT, F, Cusion>
        where
            F: Fn(&Cusion) -> SorterContext + Send + 'a,
            SorterT: Sorter<'a, Context = SorterContext>,
            SorterContext: 'a,
        {
            f: F,
            sorter: SorterT,

            _sorter_context: PhantomData<&'a SorterContext>,
            _cusion: PhantomData<Cusion>,
        }

        impl<'a, SorterContext, SorterT, F, Cusion> SorterWrapper<'a, SorterContext, SorterT, F, Cusion>
        where
            F: Fn(&Cusion) -> SorterContext + Send + 'a,
            SorterT: Sorter<'a, Context = SorterContext>,
            SorterContext: 'a,
            Cusion: 'a,
        {
            fn new(sorter: SorterT, transformer: F) -> Self {
                Self {
                    f: transformer,
                    sorter,

                    _sorter_context: PhantomData,
                    _cusion: PhantomData,
                }
            }
        }

        impl<'a, SorterContext, SorterT, F, Cusion> Sorter<'a>
            for SorterWrapper<'a, SorterContext, SorterT, F, Cusion>
        where
            F: Fn(&Cusion) -> SorterContext + Send + 'a,
            SorterT: Sorter<'a, Context = SorterContext>,
            SorterContext: 'a,
            Cusion: 'a,
        {
            type Context = Cusion;

            fn compare(
                &self,
                lhs: &Self::Context,
                rhs: &Self::Context,
                input: &str,
            ) -> std::cmp::Ordering {
                (self.sorter).compare(&(self.f)(lhs), &(self.f)(rhs), input)
            }
        }

        self.sorters
            .push(Box::new(SorterWrapper::new(sorter, transformer)));

        self
    }

    pub fn add_action<ActionContext, ActionT, F>(mut self, action: ActionT, transformer: F) -> Self
    where
        F: Fn(Cusion) -> ActionContext + Send + 'a,
        ActionT: Action<'a, Context = ActionContext> + 'a,
        ActionContext: 'a,
        Cusion: 'a,
    {
        struct ActionWrapper<'a, ActionContext, ActionT, F, Cusion>
        where
            F: Fn(Cusion) -> ActionContext + Send + 'a,
            ActionT: Action<'a, Context = ActionContext>,
            ActionContext: 'a,
            Cusion: 'a,
        {
            f: F,
            action: ActionT,

            _cusion: PhantomData<&'a Cusion>,
        }

        impl<'a, ActionContext, ActionT, F, Cusion> ActionWrapper<'a, ActionContext, ActionT, F, Cusion>
        where
            F: Fn(Cusion) -> ActionContext + Send + 'a,
            ActionT: Action<'a, Context = ActionContext>,
            ActionContext: 'a,
            Cusion: 'a,
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
            F: Fn(Cusion) -> ActionContext + Send + 'a,
            ActionT: Action<'a, Context = ActionContext>,
            ActionContext: 'a,
            Cusion: 'a,
        {
            type Context = Cusion;

            fn act(&self, ctx: Self::Context) {
                self.action.act((self.f)(ctx));
            }
        }
        self.actions
            .push(Box::new(ActionWrapper::new(action, transformer)));

        self
    }

    pub fn filter_and(mut self, flag: bool) -> Self {
        self.filter_and = flag;
        self
    }

    #[cfg(feature = "parallel")]
    pub fn par_sort(mut self, flag: bool) -> Self {
        self.par_sort = flag;
        self
    }

    #[cfg(feature = "parallel")]
    pub fn par_filter(mut self, flag: bool) -> Self {
        self.par_filter = flag;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_source() -> Result<(), Box<dyn std::error::Error>> {
        let _launcher =
            Launcher::default().add_source(Box::pin(tokio_stream::iter(vec![1, 2])), |x| x);

        assert_eq!(_launcher.sources.len(), 1);

        Ok(())
    }

    #[test]
    fn add_filter() -> Result<(), Box<dyn std::error::Error>> {
        let _launcher = Launcher::default().add_filter(
            crate::filter::ClosureFilter::new(|&x: &u8, input| x == 0u8 && input == ""),
            |x| *x,
        );

        assert_eq!(_launcher.filters.len(), 1);

        Ok(())
    }

    #[test]
    fn add_sorter() -> Result<(), Box<dyn std::error::Error>> {
        let _launcher = Launcher::default().add_sorter(
            crate::sorter::ClosureSorter::new(|lhs: &u8, rhs, _| lhs.cmp(rhs)),
            |x| *x,
        );

        assert_eq!(_launcher.sorters.len(), 1);

        Ok(())
    }

    #[test]
    fn add_action() -> Result<(), Box<dyn std::error::Error>> {
        let _launcher = Launcher::default().add_action(
            crate::action::ClosureAction::new(|x: u8| println!("{x}")),
            std::convert::identity,
        );

        assert_eq!(_launcher.actions.len(), 1);

        Ok(())
    }
}
