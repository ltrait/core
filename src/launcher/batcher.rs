use color_eyre::eyre::{Result, ensure};

use crate::filter::Filter;
use crate::generator::Generator;
use crate::sorter::Sorter;
use crate::source::Source;

use crate::ui::Buffer;

use std::marker::PhantomData;

use tokio_stream::StreamExt as _;

type CusionToUIF<'a, Cusion, UIContext> = Option<Box<dyn Fn(&Cusion) -> UIContext + Send + 'a>>;

type FilterT<'a, Cusion> = Box<dyn Filter<'a, Context = Cusion> + Send + 'a>;
type SorterT<'a, Cusion> = Box<dyn Sorter<'a, Context = Cusion> + Send + 'a>;
type GenT<'a, Cusion> = Box<dyn Generator<Item = Cusion> + Send + 'a>;

pub struct Batcher<'a, Cusion, UIContext> {
    filters: Vec<FilterT<'a, Cusion>>,
    sorters: Vec<SorterT<'a, Cusion>>,
    generators: Vec<GenT<'a, Cusion>>,
    sources: Vec<Source<'a, Cusion>>,

    pub(super) filter_and: bool,

    pub(super) cusion_to_ui: CusionToUIF<'a, Cusion, UIContext>,

    pub(super) batch_size: usize,

    state: BatcherState<Cusion>,
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

            state: BatcherState::default(),
        }
    }
}

struct BatcherState<Cusion> {
    input: String,

    /// Items sourced from Source and generators when first batch
    /// The cache of the second and subsequent times is used.
    ///
    /// And Buffer's usize is `sourced_items`'s index
    items: Vec<Cusion>,

    // index of items
    items_from_sources_i: Buffer<usize>,

    peeked_item: Option<Cusion>,
    // 本当に最初にsourceからitemを取得するときにまだ取得してpeeked_itemに入っていないだけなのに
    // source_indexを一つ上げてしまって最初のsourceがsourceされなくなるからひつよう
    first_source: bool,

    gen_index: usize,
    source_index: usize,
}

impl<Cusion> Default for BatcherState<Cusion> {
    fn default() -> Self {
        Self {
            input: "".into(),
            gen_index: 0,
            source_index: 0,
            first_source: true,
            peeked_item: None,
            items: vec![],
            items_from_sources_i: Buffer::default(),
        }
    }
}

impl<'a, Cusion, UIContext> Batcher<'a, Cusion, UIContext>
where
    Cusion: std::marker::Send,
{
    /// Consumes (and destroys) the current instance, returning ownership of the `Cusion`.
    ///
    /// Call this function as the final step to retrieve the `Cusion`.
    pub fn compute_cusion(mut self, id: usize) -> Result<Cusion> {
        ensure!(
            self.state.items.len() > id,
            "Failed to get Cusion, index is over the length. Maybe the ui is not using the usize obtained from Buffer"
        );

        Ok(self.state.items.swap_remove(id))
    }

    /// Take Buffer and merge Items(UIContext) into the Buffer
    /// Returns that whether there are items that have not yet been fully acquired(bool)
    ///
    /// This reset the position of buffer
    pub async fn marge(&mut self, buf: &mut Buffer<(UIContext, usize)>) -> Result<bool>
    where
        Cusion: 'a,
    {
        ensure!(
            self.cusion_to_ui.is_some(),
            "Cusion to UIContext did not set. Did you set UI?(This error is probably not called because of the way Rust works!)"
        );

        buf.reset_pos();

        let mut batch_count = if self.batch_size == 0 {
            usize::MAX
        } else {
            self.batch_size
        };

        // Vec<Cusion>
        // 最後に(UIContext, usize)に変換してmargeする
        let mut v = vec![];

        while batch_count != 0 {
            if self.state.gen_index < self.generators.len() {
                let cusions_from_gen: Vec<_> = self.generators[self.state.gen_index]
                    .generate(&self.state.input)
                    .await
                    .into_iter()
                    .map(|c| {
                        self.state.items.push(c);
                        self.state.items.len() - 1
                    })
                    .collect();

                let len = cusions_from_gen.len();

                v.extend(cusions_from_gen);

                if batch_count < len {
                    batch_count = 0;
                } else {
                    batch_count -= len
                }

                self.state.gen_index += 1;
            } else if self.state.source_index < self.sources.len() {
                if let Some(cusion) = self.state.peeked_item.take() {
                    batch_count -= 1;
                    self.state.items.push(cusion);
                    v.push(self.state.items.len() - 1);
                    self.state
                        .items_from_sources_i
                        .push(self.state.items.len() - 1);
                } else if !self.state.first_source {
                    self.state.source_index += 1;
                } else {
                    self.state.first_source = false;
                }

                if let Some(cusion) = self.sources[self.state.source_index].next().await {
                    self.state.peeked_item = Some(cusion);
                } else {
                    self.state.peeked_item = None;
                }
            } else if let Some(ci) = self.state.items_from_sources_i.next() {
                v.push(*ci);
            } else {
                break;
            }
        }

        let mut v: Vec<_> = v
            .into_iter()
            .filter(|ci| {
                if self.filter_and {
                    self.filters
                        .iter()
                        .all(|filter| filter.predicate(&self.state.items[*ci], &self.state.input))
                } else {
                    self.filters
                        .iter()
                        .all(|filter| filter.predicate(&self.state.items[*ci], &self.state.input))
                }
            })
            .collect();

        // sorterは順番に適用していくのと、逆にしてstd::Ordering::Equalが出たら次のやつを参照するっていうのが同義っぽいきがする
        // どっちにするかだけど、std::Ordering::Equalが出たら戻るほうが(ここでは逆にしたりしない)計算量が少なそう

        let sorterf = |lhs: &Cusion, rhs: &Cusion| {
            use std::cmp::Ordering::*;
            for si in &self.sorters {
                match si.compare(lhs, rhs, &self.state.input) {
                    Equal => {
                        continue;
                    }
                    ord => {
                        return ord;
                    }
                }
            }

            Equal
        };

        v.sort_by(|lhs, rhs| sorterf(&self.state.items[*lhs], &self.state.items[*rhs]));

        {
            let dst = buf.as_mut();

            let dst_owned = std::mem::take(dst);
            let mut merged = Vec::with_capacity(dst_owned.len() + v.len());

            let mut iter_dst = dst_owned.into_iter();
            let mut iter_src = v.into_iter();

            let mut next_dst = iter_dst.next();
            let mut next_src = iter_src.next();

            while let (Some(a), Some(b)) = (next_dst.as_ref(), next_src.as_ref()) {
                if sorterf(&self.state.items[a.1], &self.state.items[*b])
                    != std::cmp::Ordering::Greater
                {
                    merged.push(next_dst.take().unwrap());
                    next_dst = iter_dst.next();
                } else {
                    merged.push({
                        // まず、self.state.itemsにpushしてそれのindexとtupleにする
                        let ui_ctx = (self.cusion_to_ui.as_ref().unwrap())(
                            &self.state.items[*next_src.as_ref().unwrap()],
                        );

                        (ui_ctx, *next_src.as_ref().unwrap())
                    });
                    next_src = iter_src.next();
                }
            }

            if let Some(val) = next_dst {
                merged.push(val);
                merged.extend(iter_dst);
            }
            if let Some(val) = next_src {
                merged.push({
                    let ui_ctx = (self.cusion_to_ui.as_ref().unwrap())(&self.state.items[val]);

                    (ui_ctx, val)
                });
                merged.extend(iter_src.map(|ci| {
                    let ui_ctx = (self.cusion_to_ui.as_ref().unwrap())(&self.state.items[ci]);

                    (ui_ctx, ci)
                }));
            }

            *dst = merged;
        }

        Ok(self.state.peeked_item.is_some())
    }

    /// Accepts user input, resets the internal state, and initiates processing of a new batch.
    pub fn input(&mut self, buf: &mut Buffer<(UIContext, usize)>, input: &str) {
        self.state.input = input.into();
        buf.reset();
        self.state.items_from_sources_i.reset();
    }

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
        F: Fn(Item) -> Cusion + Sync + Send + 'a,
        GenT: Generator<Item = Item> + Sync + Send + 'a,
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
