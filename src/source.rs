use std::pin::Pin;

pub type Source<'a, T> = Pin<Box<dyn tokio_stream::Stream<Item = T> + Send + 'a>>;

pub fn from_iter<'a, T, Iter>(
    iter: impl std::iter::IntoIterator<Item = T, IntoIter = Iter>,
) -> Source<'a, T>
where
    Iter: std::iter::Iterator<Item = T> + Send + 'a,
{
    Box::pin(tokio_stream::iter(iter))
}

pub fn transform_source<'a, Cushion, SourceContext, F>(
    source: Source<'a, SourceContext>,
    f: F,
) -> Source<'a, Cushion>
where
    SourceContext: 'a,
    F: Fn(SourceContext) -> Cushion + Send + 'a,
{
    use tokio_stream::StreamExt as _;

    Box::pin(source.map(f))
}
