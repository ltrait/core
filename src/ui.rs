use color_eyre::Result;

pub trait UI<'a> {
    type Context: 'a;

    fn run<Cusion: 'a + Send>(
        &self,
        batcher: crate::launcher::batcher::Batcher<'a, Cusion, Self::Context>,
    ) -> impl std::future::Future<Output = Result<Cusion>> + Send;
}

#[derive(Debug)]
pub struct Buffer<T> {
    // あとからactionをじっこうするためにhashmapてきに
    vec: Vec<T>,
    pos: usize,
}

impl<T> Default for Buffer<T> {
    fn default() -> Self {
        Self {
            vec: vec![],
            pos: 0,
        }
    }
}

impl<T> Buffer<T> {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn reset_pos(&mut self) {
        self.pos = 0;
    }

    pub(crate) fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.vec
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn push(&mut self, value: T) {
        self.vec.push(value);
    }

    /// Iterator can't return a type that borrows itself
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<&T> {
        self.pos += 1;
        self.vec.get(self.pos - 1)
    }

    pub fn has_next(&self) -> bool {
        self.pos + 1 < self.len()
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
        buf.reset_pos();
        assert_eq!(buf.next(), Some((1u32, 1)).as_ref());
        assert_eq!(buf.next(), Some((2u32, 2)).as_ref());
        assert_eq!(buf.next(), None);

        Ok(())
    }
}
