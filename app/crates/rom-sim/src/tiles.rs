//! A per-tile layer stored in chunks, each kept only once a tile in it is written (task 99).
//! System space is open space everywhere and mostly never visited, so its layers hold one
//! value and the few chunks around what was seen or built there; a layer reads the same
//! however much of it is stored.

use std::ops::{Index, IndexMut, Range};

/// Tiles per chunk, as a shift.
const SHIFT: usize = 10;
/// Tiles per chunk.
const CHUNK: usize = 1 << SHIFT;

/// One value per tile: `value` everywhere except in the chunks that are stored.
#[derive(Debug, Clone)]
pub(crate) struct Tiles<T> {
    value: T,
    len: usize,
    chunks: Vec<Option<Box<[T]>>>,
}

impl<T: Copy + PartialEq> Tiles<T> {
    /// `len` tiles holding `value`, stored as one value.
    pub fn uniform(value: T, len: usize) -> Self {
        Self {
            value,
            len,
            chunks: Vec::new(),
        }
    }

    /// The values given; chunks holding only the first value are not stored.
    pub fn from_vec(values: Vec<T>) -> Self
    where
        T: Default,
    {
        let value = values.first().copied().unwrap_or_default();
        let mut layer = Self::uniform(value, values.len());
        for (chunk, part) in values.chunks(CHUNK).enumerate() {
            if part.iter().any(|v| *v != value) {
                layer.chunk_mut(chunk).copy_from_slice(part);
            }
        }
        layer
    }

    /// Number of tiles.
    pub fn len(&self) -> usize {
        self.len
    }

    /// The value of every tile while no chunk is stored.
    pub fn uniform_value(&self) -> Option<T> {
        self.chunks
            .iter()
            .all(Option::is_none)
            .then_some(self.value)
    }

    /// Whether every tile holds `value`, stored or not.
    pub fn holds_only(&self, value: T) -> bool {
        self.value == value
            && self
                .chunks
                .iter()
                .flatten()
                .all(|chunk| chunk.iter().all(|v| *v == value))
    }

    /// Tiles in chunk `chunk`; the last chunk may be short.
    fn chunk_len(&self, chunk: usize) -> usize {
        (self.len - chunk * CHUNK).min(CHUNK)
    }

    /// Every chunk in tile order: its length, and its tiles when stored. Unstored chunks hold
    /// the value returned first.
    pub fn chunks(&self) -> (T, impl Iterator<Item = (usize, Option<&[T]>)> + '_) {
        let count = self.len.div_ceil(CHUNK);
        let parts = (0..count).map(|chunk| {
            let stored = self.chunks.get(chunk).and_then(|c| c.as_deref());
            (self.chunk_len(chunk), stored)
        });
        (self.value, parts)
    }

    /// Every tile's value in tile order.
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        let (value, chunks) = self.chunks();
        chunks.flat_map(move |(len, stored)| {
            let (run, fill) = match stored {
                Some(run) => (run, 0),
                None => (&[][..], len),
            };
            run.iter()
                .copied()
                .chain(std::iter::repeat_n(value, fill))
        })
    }

    /// One chunk's tiles to write, stored from now on.
    fn chunk_mut(&mut self, chunk: usize) -> &mut [T] {
        if self.chunks.len() <= chunk {
            self.chunks.resize_with(chunk + 1, || None);
        }
        let len = self.chunk_len(chunk);
        let value = self.value;
        self.chunks[chunk].get_or_insert_with(|| vec![value; len].into_boxed_slice())
    }

    /// Calls `f` with the index and value of every tile of `range`, to change it; the
    /// chunks touched are stored from now on.
    pub fn update(&mut self, range: Range<usize>, mut f: impl FnMut(usize, &mut T)) {
        assert!(
            range.start <= range.end && range.end <= self.len,
            "tiles {range:?} outside a layer of {}",
            self.len
        );
        let mut index = range.start;
        while index < range.end {
            let chunk = index >> SHIFT;
            let end = range.end.min((chunk + 1) * CHUNK);
            let base = chunk * CHUNK;
            for (offset, value) in self.chunk_mut(chunk)[index - base..end - base]
                .iter_mut()
                .enumerate()
            {
                f(index + offset, value);
            }
            index = end;
        }
    }

    /// Sets every tile to `value`, stored as one value again.
    pub fn set_all(&mut self, value: T) {
        self.value = value;
        self.chunks.clear();
    }

    /// Every tile's value as a vector.
    pub fn to_vec(&self) -> Vec<T> {
        self.iter().collect()
    }

    /// Stores every chunk, as the layers were stored before task 99.
    #[cfg(test)]
    pub fn store_all(&mut self) {
        for chunk in 0..self.len.div_ceil(CHUNK) {
            self.chunk_mut(chunk);
        }
    }
}

impl<T: Copy + PartialEq> PartialEq for Tiles<T> {
    /// Equal when every tile holds the same value, however each side is stored.
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}

impl<T: Copy + PartialEq> Eq for Tiles<T> {}

impl<T: Copy + PartialEq> PartialEq<Vec<T>> for Tiles<T> {
    /// Equal when the layer holds exactly these values, however it is stored.
    fn eq(&self, other: &Vec<T>) -> bool {
        self.len == other.len() && self.iter().eq(other.iter().copied())
    }
}

impl<T> Index<usize> for Tiles<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        assert!(
            index < self.len,
            "tile {index} outside a layer of {}",
            self.len
        );
        match self.chunks.get(index >> SHIFT) {
            Some(Some(chunk)) => &chunk[index & (CHUNK - 1)],
            _ => &self.value,
        }
    }
}

impl<T: Copy + PartialEq> IndexMut<usize> for Tiles<T> {
    /// Writing a tile stores its chunk.
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(
            index < self.len,
            "tile {index} outside a layer of {}",
            self.len
        );
        &mut self.chunk_mut(index >> SHIFT)[index & (CHUNK - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layer_reads_the_same_however_much_of_it_is_stored() {
        let len = 3 * CHUNK + 5;
        let mut layer = Tiles::uniform(0_u32, len);
        assert_eq!(layer.len(), len);
        assert_eq!(layer[len - 1], 0);
        assert_eq!(layer, Tiles::from_vec(vec![0; len]));
        assert_eq!(layer, vec![0; len]);
        let mut expected = vec![0; len];
        layer[50] += 3;
        expected[50] = 3;
        assert_eq!(layer.uniform_value(), None);
        layer.update(CHUNK - 2..CHUNK + 2, |i, v| *v = i as u32);
        for i in CHUNK - 2..CHUNK + 2 {
            expected[i] = i as u32;
        }
        layer[len - 1] = 9;
        expected[len - 1] = 9;
        assert_eq!(layer.to_vec(), expected);
        assert_eq!(layer, expected);
        let (value, chunks) = layer.chunks();
        assert_eq!(value, 0);
        let stored: Vec<_> = chunks.map(|(n, c)| (n, c.is_some())).collect();
        assert_eq!(
            stored,
            vec![(CHUNK, true), (CHUNK, true), (CHUNK, false), (5, true)]
        );
        assert_eq!(Tiles::from_vec(expected.clone()), layer);
        let mut all = layer.clone();
        all.store_all();
        assert_eq!(all, layer);
        assert!(!layer.holds_only(0));
        assert!(Tiles::uniform(0, 3).holds_only(0));
        let mut zeros = Tiles::uniform(0, 3);
        zeros[1] = 0;
        assert!(zeros.holds_only(0) && zeros.uniform_value().is_none());
        layer.set_all(4);
        assert_eq!(layer.uniform_value(), Some(4));
        assert_eq!(Tiles::from_vec(vec![7, 7]).uniform_value(), Some(7));
        assert_eq!(Tiles::from_vec(vec![7, 8]).uniform_value(), None);
        assert_ne!(Tiles::uniform(0, 2), Tiles::uniform(0, 3));
        assert_ne!(Tiles::uniform(0, 2), Tiles::from_vec(vec![0, 1]));
    }

    #[test]
    #[should_panic(expected = "outside a layer")]
    fn reading_past_a_layer_panics_like_a_vector() {
        let layer = Tiles::uniform(1_u8, 2);
        let _ = layer[2];
    }

    #[test]
    #[should_panic(expected = "outside a layer")]
    fn writing_past_a_layer_panics_like_a_vector() {
        let mut layer = Tiles::uniform(1_u8, 2);
        layer[2] = 0;
    }
}
