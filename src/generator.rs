use async_trait::async_trait;

/// Generator is a kind of Source but it takes input.
/// It is not envisaged that Generator will return a large number of items.
///
/// Generator can be used to implement a calculator (that needs a user input).
#[async_trait]
pub trait Generator {
    type Item;

    // 本当は短くて比較的意味が伝わりやすいgenが良かったんだけど予約語
    async fn generate(&self, input: &str) -> Vec<Self::Item>;
}

pub struct ClosureGenerator<Item, F>(F, std::marker::PhantomData<Item>)
where
    F: Fn(&str) -> Vec<Item>;

impl<Item, F> ClosureGenerator<Item, F>
where
    F: Fn(&str) -> Vec<Item>,
{
    #[allow(dead_code)]
    pub fn new(f: F) -> Self {
        Self(f, std::marker::PhantomData)
    }
}

#[async_trait]
impl<Item, F> Generator for ClosureGenerator<Item, F>
where
    F: Fn(&str) -> Vec<Item> + Sync,
    Item: Sync,
{
    type Item = Item;

    async fn generate(&self, input: &str) -> Vec<Self::Item> {
        (self.0)(input)
    }
}
