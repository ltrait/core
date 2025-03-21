use color_eyre::Result;

pub trait UI<'a> {
    type Context: 'a;

    fn run<Cusion: 'a + Send>(
        &self,
        batcher: crate::launcher::batcher::Batcher<'a, Cusion, Self::Context>,
    ) -> impl std::future::Future<Output = Result<Cusion>> + Send;
}

#[derive(Debug, Clone)]
pub struct Buffer<T> {
    vec: Vec<T>,
}

impl<T> From<Vec<T>> for Buffer<T> {
    fn from(vec: Vec<T>) -> Self {
        Self { vec }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Position(pub usize);

impl Position {
    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

impl<T> Default for Buffer<T> {
    fn default() -> Self {
        Self { vec: vec![] }
    }
}

impl<T> Buffer<T> {
    #[inline]
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    #[inline]
    pub(crate) fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.vec
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        self.vec.push(value);
    }

    /// Iterator can't return a type that borrows itself
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self, pos: &mut Position) -> Option<&T> {
        pos.0 += 1;
        self.vec.get(pos.0 - 1)
    }

    #[inline]
    pub fn has_next(&self, pos: &Position) -> bool {
        pos.0 + 1 < self.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer() -> Result<(), Box<dyn std::error::Error>> {
        let mut pos = Position::default();
        let mut buf = {
            let mut buf = Buffer::default();
            let v = buf.as_mut();
            v.push((1, 1));
            v.push((2, 2));
            dbg!(v);
            buf
        };

        assert_eq!(buf.next(&mut pos), Some((1u32, 1)).as_ref());
        assert_eq!(buf.next(&mut pos), Some((2u32, 2)).as_ref());
        assert_eq!(buf.next(&mut pos), None);
        pos.reset();
        assert_eq!(buf.next(&mut pos), Some((1u32, 1)).as_ref());
        assert_eq!(buf.next(&mut pos), Some((2u32, 2)).as_ref());
        assert_eq!(buf.next(&mut pos), None);

        Ok(())
    }
}
