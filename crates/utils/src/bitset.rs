use std::{
    alloc::{Allocator, Global},
    iter,
};

/// A set of integers represented as a bitset.
///
/// Capacity is rounded up to the next multiple of 64.
#[derive(Debug, Clone)]
pub struct BitSet<A: Allocator = Global> {
    values: Vec<u64, A>,
}

impl BitSet<Global> {
    /// Creates a new bitset with the given capacity.
    ///
    /// Capacity is rounded up to the next multiple of 64.
    ///
    /// Uses the global allocator.
    pub fn new(capacity: usize) -> Self {
        Self::new_in(capacity, Global)
    }
}

impl<A> BitSet<A>
where
    A: Allocator,
{
    /// Creates a new bitset with the given capacity.
    ///
    /// Capacity is rounded up to the next multiply of 64.
    ///
    /// Uses the given allocator.
    pub fn new_in(capacity: usize, alloc: A) -> Self {
        let capacity = ceil_64(capacity);
        let mut values = Vec::with_capacity_in(capacity / 64, alloc);
        values.extend(iter::repeat(0).take(values.capacity()));
        Self { values }
    }

    /// Inserts a value into the bitset.
    ///
    /// Returns whether the set previously contained `x`.
    ///
    /// # Panics
    /// Panics if `x >= self.capacity()`.
    #[inline]
    pub fn insert(&mut self, x: usize) -> bool {
        let (u64, bit) = self.index(x).unwrap_or_else(|| {
            panic!(
                "index out of bounds: x = {}, capacity = {}",
                x,
                self.capacity()
            )
        });
        let value = &mut self.values[u64];
        let was_set = (*value & (1 << bit)) != 0;
        *value |= 1 << bit;
        was_set
    }

    /// Returns whether the bitset contains the given value.
    ///
    /// Returns `false` of `x >= self.capacity()`.
    #[inline]
    pub fn contains(&self, x: usize) -> bool {
        let (u64, bit) = match self.index(x) {
            Some(index) => index,
            None => return false, // out of bounds
        };

        ((self.values[u64] >> bit) & 1) == 1
    }

    /// Removes the given value from the bitset.
    ///
    /// Returns whether the bitset previously contained `x`.
    ///
    /// Does nothing and returns `false` if `x >= self.capacity()`.
    #[inline]
    pub fn remove(&mut self, x: usize) -> bool {
        let (u64, bit) = match self.index(x) {
            Some(index) => index,
            None => return false, // out of bounds
        };

        let value = &mut self.values[u64];
        let was_set = (*value & (1 << bit)) != 0;
        *value &= !(1 << bit);
        was_set
    }

    /// Iterates over the values contained in this bitset.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        self.values.iter().enumerate().flat_map(|(i, &value)| {
            let i = i * 64;
            IterSetBits { value }.map(move |x| x + i)
        })
    }

    /// Gets the next element whose value is at least `min`.
    #[inline]
    pub fn next(&self, min: usize) -> Option<usize> {
        let (u64, mut min_bit) = self.index(min)?;

        for (i, &value) in self.values[u64..].iter().enumerate() {
            let mut value = value;
            let mask = (1 << min_bit as u64) - 1;
            value &= !mask;

            let n = value.trailing_zeros();
            if n != 64 {
                return Some(i * 64 + u64 * 64 + n as usize);
            }

            min_bit = 0;
        }

        None
    }

    /// Sets all bits in the bitset.
    pub fn fill(&mut self) {
        self.values.fill(u64::MAX);
    }

    /// Clears all bits in the btset.
    pub fn clear(&mut self) {
        self.values.fill(0);
    }

    /// Returns the  capacity of this bitset, which is the
    /// greatest possible value plus one.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.values.len() * 64
    }

    fn index(&self, x: usize) -> Option<(usize, usize)> {
        if x >= self.capacity() {
            None
        } else {
            Some((x / 64, x % 64))
        }
    }
}

struct IterSetBits {
    value: u64,
}

impl Iterator for IterSetBits {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let n = self.value.trailing_zeros();
        if n == 64 {
            None
        } else {
            self.value &= !(1 << n as u64);
            Some(n as usize)
        }
    }
}

fn ceil_64(x: usize) -> usize {
    (x + 63) / 64 * 64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitset_insert() {
        let mut set = BitSet::new(1000);
        set.insert(50);
        set.insert(150);
        set.insert(49);
        set.insert(1);
        assert!(set.contains(50));
        assert!(set.contains(150));
        assert!(set.contains(49));
        assert!(set.contains(1));
        assert!(!set.contains(51));
        assert!(!set.contains(0));
    }

    #[test]
    fn bitset_remove() {
        let mut set = BitSet::new(1000);
        assert!(!set.remove(0));
        set.insert(1);
        assert!(set.remove(1));
        assert!(!set.contains(1));
    }

    #[test]
    fn bitset_iter() {
        let mut set = BitSet::new(1000);

        (0..100).for_each(|x| assert!(!set.insert(x)));
        (200..250).for_each(|x| assert!(!set.insert(x)));
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            (0..100).chain(200..250).collect::<Vec<_>>(),
        )
    }

    #[test]
    #[should_panic]
    fn bitset_insert_out_of_bounds() {
        let mut set = BitSet::new(128);
        set.insert(128);
    }

    #[test]
    fn bitset_remove_out_of_bounds() {
        let mut set = BitSet::new(128);
        assert!(!set.remove(128));
    }

    #[test]
    fn bitset_contains_out_of_bounds() {
        let mut set = BitSet::new(128);
        set.insert(127);
        assert!(!set.contains(128));
    }

    #[test]
    fn bitset_next() {
        let mut set = BitSet::new(1000);
        assert_eq!(set.next(0), None);
        set.insert(1);
        assert_eq!(set.next(0), Some(1));
        assert_eq!(set.next(1), Some(1));
        assert_eq!(set.next(2), None);

        set.insert(500);
        assert_eq!(set.next(2), Some(500));
        assert_eq!(set.next(499), Some(500));
        assert_eq!(set.next(500), Some(500));
        assert_eq!(set.next(501), None);
    }

    #[test]
    fn test_ceil_64() {
        assert_eq!(ceil_64(63), 64);
        assert_eq!(ceil_64(1), 64);
        assert_eq!(ceil_64(0), 0);
        assert_eq!(ceil_64(65), 128);
    }
}
