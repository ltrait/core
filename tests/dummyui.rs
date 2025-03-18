use ltrait::{
    UI,
    color_eyre::eyre::{Result, ensure},
    launcher::batcher::Batcher,
    ui::{Buffer, Position},
};

use std::marker::PhantomData;

pub struct DummyUI<'a, T, F>
where
    T: 'a,
    F: Fn(&T),
{
    f: F,

    _cusion: PhantomData<&'a T>,
}

impl<'a, T, F> DummyUI<'a, T, F>
where
    T: 'a + Sync + Send,
    F: Fn(&T) + Sync,
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            _cusion: PhantomData,
        }
    }
}

impl<'a, T, F> UI<'a> for DummyUI<'a, T, F>
where
    T: 'a + Sync + Send,
    F: Fn(&T) + Sync,
{
    type Context = T;

    async fn run<Cusion: 'a + Send>(
        &self,
        mut batcher: Batcher<'a, Cusion, Self::Context>,
    ) -> Result<Cusion> {
        let mut more = true;
        let mut buf: Buffer<(T, usize)> = Buffer::default();

        while more {
            let from = batcher.prepare().await;
            more = batcher.merge(&mut buf, from).await?;
        }

        let mut pos = Position::default();
        let mut least_one = false;
        while let Some(&(ref c, _)) = buf.next(&mut pos) {
            (self.f)(c);
            if !least_one {
                least_one = true;
            }
        }

        ensure!(least_one, "No items were processed by the UI"); // Ensure at least one item was processed

        batcher.compute_cusion(0)
    }
}
