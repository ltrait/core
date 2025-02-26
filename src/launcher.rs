use crate::source::Source;

pub struct Launcher<'a, Cusion> {
    sources: Vec<Source<'a, Cusion>>,
}

impl<'a, Cusion> Default for Launcher<'a, Cusion> {
    fn default() -> Self {
        Self {
            sources: Vec::default(),
        }
    }
}

impl<'a, Cusion> Launcher<'a, Cusion> {
    /// Add a source to `self`, builder
    fn add_source<SourceContext, F>(
        mut self,
        source: Source<'a, SourceContext>,
        transformer: F,
    ) -> Self
    where
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

            Box::pin(source.map(move |ctx| f(ctx)))
        }

        self.sources.push(transform_source(source, transformer));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_source() -> Result<(), Box<dyn std::error::Error>> {
        let _launcher =
            Launcher::default().add_source(Box::pin(tokio_stream::iter(vec![1, 2])), |x| x);

        assert_eq!(_launcher.sources.len(), 1);

        Ok(())
    }
}
