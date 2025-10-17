use std::pin::Pin;

pub type Source<T> = Pin<Box<dyn tokio_stream::Stream<Item = T> + Send>>;

pub fn from_iter<T, Iter>(
    iter: impl std::iter::IntoIterator<Item = T, IntoIter = Iter>,
) -> Source<T>
where
    Iter: std::iter::Iterator<Item = T> + Send + 'static,
{
    Box::pin(tokio_stream::iter(iter))
}

pub fn transform_source<Cushion, SourceContext, F>(
    source: Source<SourceContext>,
    f: F,
) -> Source<Cushion>
where
    SourceContext: 'static,
    F: Fn(SourceContext) -> Cushion + Send + 'static,
{
    use tokio_stream::StreamExt as _;

    Box::pin(source.map(f))
}
