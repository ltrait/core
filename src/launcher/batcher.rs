use color_eyre::eyre::{Result, ensure, eyre};

use tracing::{debug, info};

use crate::filter::Filter;
use crate::generator::Generator;
use crate::sorter::Sorter;
use crate::source::Source;

use crate::ui::{Buffer, Position};

use tokio_stream::StreamExt as _;

type CushionToUIF<Cushion, UIContext> = Option<Box<dyn Fn(&Cushion) -> UIContext + Send>>;

type FilterT<Cushion> = Box<dyn Filter<Context = Cushion>>;
type SorterT<Cushion> = Box<dyn Sorter<Context = Cushion>>;
type GenT<Cushion> = Box<dyn Generator<Item = Cushion>>;

pub struct Batcher<Cushion, UIContext> {
    filters: Vec<FilterT<Cushion>>,
    sorters: Vec<SorterT<Cushion>>,
    generators: Vec<GenT<Cushion>>,
    sources: Vec<Source<Cushion>>,

    pub(super) cushion_to_ui: CushionToUIF<Cushion, UIContext>,

    pub(super) batch_size: usize,
    pub(super) filter_and: bool,

    state: BatcherState<Cushion>,
}

impl<Cushion, UIContext> Default for Batcher<Cushion, UIContext>
where
    UIContext: Send,
{
    fn default() -> Self {
        Self {
            filters: vec![],
            sorters: vec![],
            sources: vec![],
            generators: vec![],

            batch_size: 0,
            filter_and: true,

            cushion_to_ui: None,

            state: BatcherState::default(),
        }
    }
}

struct BatcherState<Cushion> {
    input: String,

    /// Items sourced from Source and generators when first batch
    /// The cache of the second and subsequent times is used.
    ///
    /// And Buffer's usize is `sourced_items`'s index
    items: Vec<Cushion>,

    // index of items
    items_from_sources_i: (Buffer<usize>, Position),

    peeked_item: Option<Cushion>,
    // 本当に最初にsourceからitemを取得するときにまだ取得してpeeked_itemに入っていないだけなのに
    // source_indexを一つ上げてしまって最初のsourceがsourceされなくなるからひつよう
    first_source: bool,

    gen_index: usize,
    source_index: usize,
}

mod debug_state {
    use super::BatcherState;
    use std::fmt;

    impl<Cushion> fmt::Debug for BatcherState<Cushion> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // itemsの件数を表示
            let items_info = format!("{} item(s)", self.items.len());
            // peeked_itemはSomeかNoneかを表示
            let peeked_info = if self.peeked_item.is_some() {
                "Some"
            } else {
                "None"
            };

            f.debug_struct("BatcherState")
                .field("input", &self.input)
                .field("items", &items_info)
                .field("items_from_sources_i", &self.items_from_sources_i)
                .field("peeked_item", &peeked_info)
                .field("first_source", &self.first_source)
                .field("gen_index", &self.gen_index)
                .field("source_index", &self.source_index)
                .finish()
        }
    }
}

impl<Cushion> Default for BatcherState<Cushion> {
    fn default() -> Self {
        Self {
            input: "".into(),
            gen_index: 0,
            source_index: 0,
            first_source: true,
            peeked_item: None,
            items: vec![],
            items_from_sources_i: (Buffer::default(), Position::default()),
        }
    }
}

pub struct Prepared<T>(Buffer<(T, usize)>);

impl<T> Prepared<T> {
    pub(crate) fn into_inner(self) -> Buffer<(T, usize)> {
        self.0
    }

    pub(crate) fn new(value: Buffer<(T, usize)>) -> Self {
        Self(value)
    }
}

impl<Cushion, UIContext> Batcher<Cushion, UIContext>
where
    Cushion: Send,
{
    /// Consumes (and destroys) the current instance, returning ownership of the `Cushion`.
    ///
    /// Call this function as the final step to retrieve the `Cushion`.
    #[inline(always)]
    pub fn compute_cushion(mut self, id: usize) -> Result<Cushion> {
        ensure!(
            self.state.items.len() > id,
            "Failed to get Cushion, index is over the length. Maybe the ui is not using the usize obtained from Buffer"
        );

        Ok(self.state.items.swap_remove(id))
    }

    #[inline(always)]
    fn create_sorter(&self) -> impl Fn(&usize, &usize) -> std::cmp::Ordering {
        |lhs, rhs| {
            use std::cmp::Ordering;

            let lhs = &self.state.items[*lhs];
            let rhs = &self.state.items[*rhs];
            for si in &self.sorters {
                match si.compare(lhs, rhs, &self.state.input) {
                    Ordering::Equal => {
                        continue;
                    }
                    ord => return ord,
                }
            }

            Ordering::Equal
        }
    }

    // ここのusizeはself.state.itemsのindex
    // あとからmergeで比較しつつmergeして、最後にcushion_to_uiで(UIContext, usize)に変換される

    /// Prepares the next batch of indices for rendering.
    ///
    /// This asynchronous function generates and returns a `Prepared<UIContext>` containing indices
    /// that correspond to UI elements needing rendering. The resulting buffer is intended to be used
    /// in conjunction with a rendering buffer of type `Buffer<(UIContext, usize)>` during the merge process.
    ///
    /// For optimal performance, it is recommended that this function runs concurrently with the rendering process.
    #[must_use]
    #[inline]
    pub async fn prepare(&mut self) -> Prepared<UIContext> {
        info!("Preparing");
        debug!("state on prepare {:?}", self.state);

        if self.cushion_to_ui.is_none() {
            panic!(
                "{}",
                eyre!(
                    "Cushion to UIContext did not set. Did you set UI?(This error is probably not called because of the way Rust works!)"
                )
            );
        }

        let mut batch_count = if self.batch_size == 0 {
            usize::MAX
        } else {
            self.batch_size
        };

        // Vec<usize(id self.state.items)>
        // 最後に(UIContext, usize)に変換してmargeする
        let mut v = {
            let estimated_capacity = if self.batch_size == 0 {
                256
            } else {
                batch_count
            };
            self.state.items.reserve(estimated_capacity);
            Vec::with_capacity(estimated_capacity)
        };

        let gen_len = self.generators.len();
        if self.state.gen_index < gen_len && batch_count > 0 {
            use std::sync::atomic::{AtomicUsize, Ordering};

            let len = AtomicUsize::new(0);

            let gen_count_to_run = batch_count.min(gen_len - self.state.gen_index);

            // Iterator<Item = impl Future<Output = Vec<Cushion>>>
            // でjoin_allでFutureを解決して
            let cushions_from_gen = self.generators
                [self.state.gen_index..(self.state.gen_index + gen_count_to_run)]
                .iter()
                .map(|r#gen| async {
                    let cushions = r#gen.generate(&self.state.input).await.into_iter();
                    // 最終結果で計算が終わったあとの長さにしか興味がないからRelaxedで問題ない
                    len.fetch_add(cushions.len(), Ordering::Relaxed);
                    cushions
                });

            let cushions_from_gen = futures::future::join_all(cushions_from_gen)
                .await
                .into_iter()
                .flatten();

            v.reserve(len.load(Ordering::SeqCst));
            for c in cushions_from_gen {
                let index = self.state.items.len();
                self.state.items.push(c);
                v.push(index);
            }

            if batch_count < gen_count_to_run {
                batch_count = 0;
            } else {
                // カウントして減らすこともできるけど流石にドキュメントするのも大変
                batch_count -= gen_count_to_run
            }

            self.state.gen_index += gen_count_to_run;
        }

        while batch_count != 0 {
            if let Some(ci) = self
                .state
                .items_from_sources_i
                .0
                .next(&mut self.state.items_from_sources_i.1)
            {
                v.push(*ci);
            } else if self.state.source_index < self.sources.len() {
                if let Some(cushion) = self.state.peeked_item.take() {
                    batch_count -= 1;
                    self.state.items.push(cushion);
                    v.push(self.state.items.len() - 1);
                    self.state
                        .items_from_sources_i
                        .0
                        .push(self.state.items.len() - 1);
                } else if !self.state.first_source {
                    self.state.source_index += 1;
                    if self.state.source_index == self.sources.len() {
                        break;
                    }
                } else {
                    // 初めての場合はpeeked_itemにいれるけど
                    self.state.first_source = false;
                }

                // dbg!(&self.state);

                self.state.peeked_item = self.sources[self.state.source_index].next().await;
            } else {
                break;
            }
        }

        let ctuf = self.cushion_to_ui.as_ref().unwrap();

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
                        .any(|filter| filter.predicate(&self.state.items[*ci], &self.state.input))
                }
            })
            .map(|ci| (ctuf(&self.state.items[ci]), ci))
            .collect();

        let sorterf = self.create_sorter();

        v.sort_by(|(_, lhs), (_, rhs)| sorterf(lhs, rhs));

        Prepared::new(v.into())
    }

    /// Merges UI context data into the rendering buffer.
    ///
    /// This synchronous function accepts two buffers:
    /// - `buf`: A mutable reference to a `Buffer<(UIContext, usize)>` used for UI rendering.
    /// - `from`: A `Buffer<usize>` produced by `prepare` containing corresponding indices.
    ///
    /// The function associates each index from `from` with its respective UI context and inserts
    /// the resulting pairs into `buf`. It returns a `Result<bool>`, where the boolean indicates whether
    /// there remain items that have not been fully processed.
    ///
    /// Note:
    /// Both the preparation and merge operations are relatively time-consuming. To minimize rendering delays,
    /// it is recommended that the preparation and rendering processes are executed concurrently (for example, in separate
    /// threads or processes), while the merge operation should be performed in a synchronized manner.
    pub fn merge(
        &mut self,
        buf: &mut Buffer<(UIContext, usize)>,
        from: Prepared<UIContext>,
    ) -> Result<bool> {
        debug!("state on merge: {:?}", self.state);

        // sorterは順番に適用していくのと、逆にしてstd::Ordering::Equalが出たら次のやつを参照するっていうのが同義っぽいきがする
        // どっちにするかだけど、std::Ordering::Equalが出たら戻るほうが(ここでは逆にしたりしない)計算量が少なそう

        let v = from.into_inner().into_inner();

        let sorterf = self.create_sorter();

        {
            let dst = buf.as_mut();

            let dst_owned = std::mem::take(dst);
            let mut merged = Vec::with_capacity(dst_owned.len() + v.len());

            let mut iter_dst = dst_owned.into_iter();
            let mut iter_src = v.into_iter();

            let mut next_dst = iter_dst.next();
            let mut next_src = iter_src.next();

            while next_src.is_some() && next_dst.is_some() {
                let a = next_dst.take().unwrap();
                let b = next_src.take().unwrap();

                if sorterf(&a.1, &b.1) != std::cmp::Ordering::Greater {
                    merged.push(a);
                    next_dst = iter_dst.next();
                } else {
                    merged.push(b);
                    next_src = iter_src.next();
                }
            }

            if let Some(val) = next_dst {
                merged.push(val);
                merged.extend(iter_dst);
            }
            if let Some(val) = next_src {
                merged.push(val);
                merged.extend(iter_src);
            }

            *dst = merged;
        }

        Ok(self.state.peeked_item.is_some())
    }

    /// Accepts user input, resets the internal state, and initiates processing of a new batch.
    pub fn input(&mut self, buf: &mut Buffer<(UIContext, usize)>, input: &str) {
        self.state.input = input.into();
        self.state.gen_index = 0;
        buf.reset();

        // Positionだけリセット。元(Positionを分けるまえ)のコードにはバグがあって(多分)全部払い出したあとにinputすると変になってた
        self.state.items_from_sources_i.1.reset();
    }

    // そういえばSourceだけもともとBoxを求めてる(まあいいや)
    /// Add a source to `self`, builder
    pub(super) fn add_raw_source(&mut self, source: Source<Cushion>) {
        self.sources.push(source);
    }

    pub(super) fn add_raw_filter<FilterT>(&mut self, filter: FilterT)
    where
        FilterT: Filter<Context = Cushion> + 'static,
    {
        self.filters.push(Box::new(filter));
    }

    pub(super) fn add_raw_sorter<SorterT>(&mut self, sorter: SorterT)
    where
        SorterT: Sorter<Context = Cushion> + 'static,
    {
        self.sorters.push(Box::new(sorter));
    }

    pub(super) fn add_raw_generator<GenT>(&mut self, generator: GenT)
    where
        GenT: Generator<Item = Cushion> + Sync + Send + 'static,
    {
        self.generators.push(Box::new(generator));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_source() -> Result<(), Box<dyn std::error::Error>> {
        let mut batcher: Batcher<i32, ()> = Batcher::default();

        batcher.add_raw_source(Box::pin(tokio_stream::iter(vec![1, 2])));

        assert_eq!(batcher.sources.len(), 1);

        Ok(())
    }

    #[test]
    fn add_filter() -> Result<(), Box<dyn std::error::Error>> {
        let mut batcher: Batcher<i32, ()> = Batcher::default();

        batcher.add_raw_filter(crate::filter::ClosureFilter::new(|&x: &i32, input| {
            x == 0i32 && input == ""
        }));

        assert_eq!(batcher.filters.len(), 1);

        Ok(())
    }

    #[test]
    fn add_sorter() -> Result<(), Box<dyn std::error::Error>> {
        let mut batcher: Batcher<i32, ()> = Batcher::default();

        batcher.add_raw_sorter(crate::sorter::ClosureSorter::new(|lhs: &i32, rhs, _| {
            lhs.cmp(rhs)
        }));

        assert_eq!(batcher.sorters.len(), 1);

        Ok(())
    }

    #[test]
    fn add_generator() -> Result<(), Box<dyn std::error::Error>> {
        let mut batcher: Batcher<(), ()> = Batcher::default();
        batcher.add_raw_generator(crate::generator::ClosureGenerator::new(|input| {
            println!("{input}");
            vec![]
        }));

        assert_eq!(batcher.generators.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_prepare() -> Result<(), Box<dyn std::error::Error>> {
        let mut batcher: Batcher<i32, ()> = Batcher::default();
        batcher.cushion_to_ui = Some(Box::new(|_: &i32| ()));

        batcher.add_raw_source(Box::pin(tokio_stream::iter(vec![1, 2])));

        let buf = batcher.prepare().await;
        assert_eq!(buf.0.len(), 2);
        Ok(())
    }
}
