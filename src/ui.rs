use color_eyre::Result;

pub trait UI<'a> {
    type Context: 'a;

    // TODO: クッションを返したい
    fn run<Cusion: 'a + Sync>(
        &self,
        batcher: crate::launcher::batcher::Batcher<Cusion, Self::Context>,
    ) -> impl std::future::Future<Output = Result<Cusion>> + Send;
}

#[derive(Debug, Default)]
pub struct Buffer<T> {
    // あとからactionをじっこうするためにhashmapてきに
    vec: Vec<(T, usize)>,
    pos: usize,
}

impl<T> Buffer<T> {
    pub(crate) fn reset(&mut self) {
        self.pos = 0;
    }

    fn as_mut(&mut self) -> &mut Vec<(T, usize)> {
        &mut self.vec
    }

    /// not iterator
    pub fn next(&mut self) -> Option<&(T, usize)> {
        self.pos += 1;
        self.vec.get(self.pos - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = {
            let mut buf = Buffer::default();
            let v = buf.as_mut();
            v.push((1, 1));
            v.push((2, 2));
            dbg!(v);
            buf
        };

        assert_eq!(buf.next(), Some((1u32, 1)).as_ref());
        assert_eq!(buf.next(), Some((2u32, 2)).as_ref());
        assert_eq!(buf.next(), None);
        buf.reset();
        assert_eq!(buf.next(), Some((1u32, 1)).as_ref());
        assert_eq!(buf.next(), Some((2u32, 2)).as_ref());
        assert_eq!(buf.next(), None);

        Ok(())
    }
}
