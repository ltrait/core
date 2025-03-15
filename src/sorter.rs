pub trait Sorter<'a>: Send {
    type Context: 'a;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering;
}

pub struct ClosureSorter<'a, Context, F>(F, std::marker::PhantomData<&'a Context>)
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering,
    Context: 'a;

impl<'a, Context, F> ClosureSorter<'a, Context, F>
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering,
    Context: 'a,
{
    pub fn new(f: F) -> Self {
        Self(f, std::marker::PhantomData)
    }
}

impl<'a, Context, F> Sorter<'a> for ClosureSorter<'a, Context, F>
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering + Send,
    Context: 'a + Sync,
{
    type Context = Context;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering {
        (self.0)(lhs, rhs, input)
    }
}
