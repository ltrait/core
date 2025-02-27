pub trait Sorter<'a> {
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
    #[allow(dead_code)]
    pub fn new(f: F) -> Self {
        Self(f, std::marker::PhantomData)
    }
}

impl<'a, Context, F> Sorter<'a> for ClosureSorter<'a, Context, F>
where
    F: Fn(&Context, &Context, &str) -> std::cmp::Ordering,
    Context: 'a,
{
    type Context = Context;

    fn compare(&self, lhs: &Self::Context, rhs: &Self::Context, input: &str) -> std::cmp::Ordering {
        (self.0)(lhs, rhs, input)
    }
}
