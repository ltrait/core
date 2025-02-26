use std::pin::Pin;

pub type Source<'a, T> = Pin<Box<dyn tokio_stream::Stream<Item = T> + Send + 'a>>;
