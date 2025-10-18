use ltrait::{
    UI,
    color_eyre::eyre::Result,
    launcher::batcher::Batcher,
    ui::{Buffer, Position},
};

use std::marker::PhantomData;

pub struct DummyUI<T, F>
where
    F: Fn(&T),
{
    f: F,

    _marker: PhantomData<T>,
}

impl<T, F> DummyUI<T, F>
where
    T: Sync + Send,
    F: Fn(&T) + Sync,
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: PhantomData,
        }
    }
}

impl<T, F, Cushion> UI<Cushion> for DummyUI<T, F>
where
    T: Sync + Send,
    F: Fn(&T) + Sync,
    Cushion: Send + Sync + 'static,
{
    type Context = T;

    async fn run(&self, mut batcher: Batcher<Cushion, Self::Context>) -> Result<Option<Cushion>> {
        let mut more = true;
        let mut buf: Buffer<(T, usize)> = Buffer::default();

        while more {
            let from = batcher.prepare().await;
            more = batcher.merge(&mut buf, from)?;
        }

        let mut pos = Position::default();
        let mut least_one = false;
        while let Some(&(ref c, _)) = buf.next(&mut pos) {
            (self.f)(c);
            if !least_one {
                least_one = true;
            }
        }

        if least_one {
            Ok(Some(batcher.compute_cushion(0)?))
        } else {
            Ok(None)
        }
    }
}
