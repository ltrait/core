use async_trait::async_trait;
use std::marker::PhantomData;

/// Generator is a kind of Source but it takes input.
/// It is not envisaged that Generator will return a large number of items.
///
/// Generator can be used to implement a calculator (that needs a user input).
#[async_trait]
pub trait Generator: Send + Sync {
    type Item;

    // 本当は短くて比較的意味が伝わりやすいgenが良かったんだけど予約語
    async fn generate(&self, input: &str) -> Vec<Self::Item>;
}

pub struct ClosureGenerator<Item, F>(F, PhantomData<Item>)
where
    F: Fn(&str) -> Vec<Item>;

impl<Item, F> ClosureGenerator<Item, F>
where
    F: Fn(&str) -> Vec<Item>,
{
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
}

#[async_trait]
impl<Item, F> Generator for ClosureGenerator<Item, F>
where
    F: Fn(&str) -> Vec<Item> + Sync + Send,
    Item: Sync + Send,
{
    type Item = Item;

    async fn generate(&self, input: &str) -> Vec<Self::Item> {
        (self.0)(input)
    }
}

pub struct GenWrapper<Item, GenT, F, Cushion>
where
    F: Fn(Item) -> Cushion + Sync + Send,
    GenT: Generator<Item = Item> + Sync,
    Cushion: Sync + Send,
{
    f: F,
    generator: GenT,

    _marker: PhantomData<Cushion>,
}

#[async_trait]
impl<Item, GenT, F, Cushion> Generator for GenWrapper<Item, GenT, F, Cushion>
where
    F: Fn(Item) -> Cushion + Sync + Send,
    GenT: Generator<Item = Item> + Sync,
    Cushion: Sync + Send,
{
    type Item = Cushion;

    async fn generate(&self, input: &str) -> Vec<Self::Item> {
        self.generator
            .generate(input)
            .await
            .into_iter()
            .map(|item| (self.f)(item)) // 直接self.fを渡すと参照のエラー
            .collect()
    }
}

impl<Item, GenT, F, Cushion> GenWrapper<Item, GenT, F, Cushion>
where
    F: Fn(Item) -> Cushion + Sync + Send,
    GenT: Generator<Item = Item> + Sync,
    Cushion: Sync + Send,
{
    pub fn new(generator: GenT, transformer: F) -> Self {
        Self {
            f: transformer,
            generator,
            _marker: PhantomData,
        }
    }
}
