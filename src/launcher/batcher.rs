use crate::action::Action;
use crate::filter::Filter;
use crate::sorter::Sorter;
use crate::source::Source;

use std::marker::PhantomData;

pub struct Batcher<'a, Cusion, UIContext> {
    filters: Vec<Box<dyn Filter<'a, Context = Cusion> + 'a>>,
    sorters: Vec<Box<dyn Sorter<'a, Context = Cusion> + 'a>>,
    sources: Vec<Source<'a, Cusion>>,

    /// if `filter_and` and number of filters is greater than 1,
    /// the launcher will show you entries where all of the filter predicates are true.
    /// the default is false.
    filter_and: bool,

    pub(super) cusion_to_ui: Option<Box<dyn Fn(&Cusion) -> UIContext + 'a>>,

    // TODO: impl
    #[cfg(feature = "parallel")]
    par_sort: bool,
    #[cfg(feature = "parallel")]
    par_filter: bool,
}

impl<'a, Cusion, UIContext> Default for Batcher<'a, Cusion, UIContext>
where
    UIContext: 'a + Send,
{
    fn default() -> Self {
        Self {
            filters: vec![],
            sorters: vec![],
            sources: vec![],
            filter_and: true,
            cusion_to_ui: None,

            #[cfg(feature = "parallel")]
            par_sort: false,
            #[cfg(feature = "parallel")]
            par_filter: false,
        }
    }
}

// TODO: marge(pub)(&mut Buffer)

impl<'a, Cusion, UIContext> Batcher<'a, Cusion, UIContext>
where
    Cusion: std::marker::Send,
{
    /// Add a source to `self`, builder
    pub(super) fn add_source<SourceContext, F>(
        &mut self,
        source: Source<'a, SourceContext>,
        transformer: F,
    ) where
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
    }

    pub(super) fn add_filter<FilterContext, FilterT, F>(&mut self, filter: FilterT, transformer: F)
    where
        F: Fn(&Cusion) -> FilterContext + Send + 'a,
        FilterContext: 'a + Sync,
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
            FilterContext: 'a + Sync,
            Cusion: 'a + Send,
        {
            type Context = Cusion;

            fn predicate(&self, ctx: &Self::Context, input: &str) -> bool {
                self.filter.predicate(&(self.f)(ctx), input)
            }
        }

        self.filters
            .push(Box::new(FilterWrapper::new(filter, transformer)));
    }

    pub(super) fn add_sorter<SorterContext, SorterT, F>(&mut self, sorter: SorterT, transformer: F)
    where
        F: Fn(&Cusion) -> SorterContext + Send + 'a,
        SorterContext: 'a + Sync,
        SorterT: Sorter<'a, Context = SorterContext> + 'a,
        Cusion: 'a + Send,
    {
        struct SorterWrapper<'a, SorterContext, SorterT, F, Cusion>
        where
            F: Fn(&Cusion) -> SorterContext + Send + 'a,
            SorterT: Sorter<'a, Context = SorterContext>,
            SorterContext: 'a + Sync,
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
            SorterContext: 'a + Sync,
            Cusion: 'a + Send,
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
            SorterContext: 'a + Sync,
            Cusion: 'a + Send,
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
    }

    pub(super) fn filter_and(&mut self, flag: bool) {
        self.filter_and = flag;
    }

    #[cfg(feature = "parallel")]
    pub(super) fn par_sort(&mut self, flag: bool) {
        self.par_sort = flag;
    }

    #[cfg(feature = "parallel")]
    pub(super) fn par_filter(&mut self, flag: bool) {
        self.par_filter = flag;
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn add_source() -> Result<(), Box<dyn std::error::Error>> {
    //     let mut batcher = Batcher::default();
    //
    //     batcher.add_source(Box::pin(tokio_stream::iter(vec![1, 2])), |x| x);
    //
    //     assert_eq!(batcher.sources.len(), 1);
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn add_filter() -> Result<(), Box<dyn std::error::Error>> {
    //     let mut batcher = Batcher::default();
    //
    //     batcher.add_filter(
    //         crate::filter::ClosureFilter::new(|&x: &i32, input| x == 0i32 && input == ""),
    //         |x| *x,
    //     );
    //
    //     assert_eq!(batcher.filters.len(), 1);
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn add_sorter() -> Result<(), Box<dyn std::error::Error>> {
    //     let mut batcher = Batcher::default();
    //     batcher.add_sorter(
    //         crate::sorter::ClosureSorter::new(|lhs: &i32, rhs, _| lhs.cmp(rhs)),
    //         |x| *x,
    //     );
    //
    //     assert_eq!(batcher.sorters.len(), 1);
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn add_action() -> Result<(), Box<dyn std::error::Error>> {
    //     let mut batcher = Batcher::default();
    //     batcher.add_action(
    //         crate::action::ClosureAction::new(|x: i32| println!("{x}")),
    //         |x: &i32| *x,
    //     );
    //
    //     assert_eq!(batcher.actions.len(), 1);
    //
    //     Ok(())
    // }
}
