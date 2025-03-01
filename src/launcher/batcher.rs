use crate::filter::Filter;
use crate::generator::Generator;
use crate::sorter::Sorter;
use crate::source::Source;

use std::marker::PhantomData;

type CusionToUIF<'a, Cusion, UIContext> = Option<Box<dyn Fn(&Cusion) -> UIContext + 'a>>;

pub struct Batcher<'a, Cusion, UIContext> {
    filters: Vec<Box<dyn Filter<'a, Context = Cusion> + 'a>>,
    sorters: Vec<Box<dyn Sorter<'a, Context = Cusion> + 'a>>,
    generators: Vec<Box<dyn Generator<Item = Cusion> + 'a>>,
    sources: Vec<Source<'a, Cusion>>,

    pub(super) filter_and: bool,

    pub(super) cusion_to_ui: CusionToUIF<'a, Cusion, UIContext>,

    pub(super) batch_size: usize,

    #[cfg(feature = "parallel")]
    pub(super) par_sort: bool,
    #[cfg(feature = "parallel")]
    pub(super) par_filter: bool,
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
            generators: vec![],

            batch_size: 0,
            cusion_to_ui: None,
            filter_and: true,

            #[cfg(feature = "parallel")]
            par_sort: false,
            #[cfg(feature = "parallel")]
            par_filter: false,
        }
    }
}

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

        self.filters.push(Box::new(FilterWrapper {
            f: transformer,
            filter,

            _filter_context: PhantomData,
            _cusion: PhantomData,
        }));
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

        self.sorters.push(Box::new(SorterWrapper {
            f: transformer,
            sorter,

            _sorter_context: PhantomData,
            _cusion: PhantomData,
        }));
    }

    pub(super) fn add_generator<Item, GenT, F>(&mut self, generator: GenT, transformer: F)
    where
        Item: 'a,
        F: Fn(Item) -> Cusion + Sync + 'a,
        GenT: Generator<Item = Item> + Sync + 'a,
        Cusion: Sync + 'a,
    {
        struct GenWrapper<Item, GenT, F, Cusion>
        where
            F: Fn(Item) -> Cusion,
            GenT: Generator<Item = Item>,
        {
            f: F,
            generator: GenT,

            _cusion: PhantomData<Cusion>,
        }

        #[async_trait::async_trait]
        impl<Item, GenT, F, Cusion> Generator for GenWrapper<Item, GenT, F, Cusion>
        where
            F: Fn(Item) -> Cusion + Sync,
            GenT: Generator<Item = Item> + Sync,
            Cusion: Sync,
        {
            type Item = Cusion;

            async fn generate(&self, input: &str) -> Vec<Self::Item> {
                self.generator
                    .generate(input)
                    .await
                    .into_iter()
                    .map(|item| (self.f)(item)) // 直接self.fを渡すと参照のエラー
                    .collect()
            }
        }

        self.generators.push(Box::new(GenWrapper {
            f: transformer,
            generator,
            _cusion: PhantomData,
        }));
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
