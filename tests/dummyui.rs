use ltrait::{
    UI,
    color_eyre::eyre::Result,
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

    _marker: PhantomData<&'a T>,
}

impl<'a, T, F> DummyUI<'a, T, F>
where
    T: 'a + Sync + Send,
    F: Fn(&T) + Sync,
{
    pub fn new(f: F) -> Self {
        Self {
            f,
            _marker: PhantomData,
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
    ) -> Result<Option<Cusion>> {
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

        if least_one {
            Ok(Some(batcher.compute_cusion(0)?))
        } else {
            Ok(None)
        }
    }
}
