use crate::filter::Filter;
use crate::source::Source;

pub struct Launcher<'a, Cusion> {
    sources: Vec<Source<'a, Cusion>>,
    filters: Vec<Box<dyn Filter<'a, Context = Cusion> + 'a>>,

    /// if `filter_all` and number of filters is greater than 1,
    /// the launcher will show you entries where all of the filter predicates are true.
    /// the default is false.
    filter_all: bool,

    // TODO: impl
    #[cfg(feature = "parallel")]
    par_sort: bool,
    #[cfg(feature = "parallel")]
    par_filter: bool,
}

impl<'a, Cusion> Default for Launcher<'a, Cusion> {
    fn default() -> Self {
        Self {
            sources: Vec::default(),
            filters: Vec::default(),
            filter_all: false,

            #[cfg(feature = "parallel")]
            par_sort: false,
            #[cfg(feature = "parallel")]
            par_filter: false,
        }
    }
}

impl<'a, Cusion> Launcher<'a, Cusion> {
    /// Add a source to `self`, builder
    fn add_source<SourceContext, F>(
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

            Box::pin(source.map(move |ctx| f(ctx)))
        }

        self.sources.push(transform_source(source, transformer));
        self
    }

    fn add_filter<FilterContext, FilterT, F>(mut self, filter: FilterT, transformer: F) -> Self
    where
        F: Fn(&Cusion) -> FilterContext + Send + 'a,
        FilterContext: 'a,
        FilterT: Filter<'a, Context = FilterContext> + 'a,
        Cusion: 'a,
    {
        use std::marker::PhantomData;

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

    #[cfg(feature = "parallel")]
    fn par_sort(mut self, flag: bool) -> self {
        self.par_sort = flag;
        self
    }

    #[cfg(feature = "parallel")]
    fn par_filter(mut self, flag: bool) -> self {
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
}
