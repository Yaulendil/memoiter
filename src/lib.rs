//! Implements a Memoized Iterator, which pairs a normal `Iterator<Item=T>` with
//!     an owned `Vec<T>` to store the items it returns. Alternatively, it may
//!     be considered as a wrapped `Vec<T>` that is lazily populated with items
//!     from the Iterator.
//!
//! This is useful for infinite Iterators where each value depends on the last,
//!     such as the factorial function: Calculating the factorial of 1000 is
//!     quite expensive, but it also includes, as byproducts, the factorials of
//!     999, 998, and so on. If these are stored, they can be retrieved later,
//!     without needing to be recalculated for their own sake.

use std::{
    collections::Bound,
    ops::{Deref, RangeBounds},
    slice::SliceIndex,
};


/// A Memoized Iterator. Wraps an Iterator, associating it with a Vector to
///     store its returns. Past returns can then be retrieved by index.
///
/// # Examples
///
/// The following example shows a `MemoIter` being used to cache the results of
///     calculating the Fibonacci Sequence.
///
/// ```
/// use memoiter::MemoIter;
/// use std::iter::successors;
///
/// let mut fibonacci: MemoIter<_, u32> = successors(
///     Some((0, 1)),
///     |&(a, b)| Some((b, b + a)),
/// ).map(|p| p.0).into();
///
/// assert_eq!(fibonacci.get(0), Some(&0));
/// assert_eq!(fibonacci.get(1), Some(&1));
/// assert_eq!(fibonacci.get(4), Some(&3));
/// assert_eq!(fibonacci.get(9), Some(&34));
///
/// //  This value was calculated as a byproduct of calculating 4, and is simply
/// //      retrieved here.
/// assert_eq!(fibonacci.get(3), Some(&2));
///
/// let (seq, _) = fibonacci.consume();
/// assert_eq!(seq, [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
/// ```
#[derive(Debug)]
pub struct MemoIter<I, T> where
    I: Iterator<Item=T>,
{
    exhausted: bool,
    iterator: I,
    sequence: Vec<T>,
}


impl<I, T> MemoIter<I, T> where
    I: Iterator<Item=T>,
{
    /// Create an empty `MemoIter` wrapping a given Iterator.
    pub fn new(iterator: I) -> Self {
        Self {
            exhausted: false,
            iterator,
            sequence: Vec::new(),
        }
    }

    /// Create an empty `MemoIter`, but with a specified capacity, wrapping a
    ///     given Iterator. This only affects the initial capacity, and does
    ///     **not** restrict the size of the internal vector.
    pub fn with_capacity(capacity: usize, iterator: I) -> Self {
        Self {
            exhausted: false,
            iterator,
            sequence: Vec::with_capacity(capacity),
        }
    }

    /// Create a `MemoIter` wrapping a given Iterator, using a provided Vector
    ///     for its storage.
    pub fn with_vec(iterator: I, sequence: Vec<T>) -> Self {
        Self {
            exhausted: false,
            iterator,
            sequence,
        }
    }
}


impl<I, T> MemoIter<I, T> where
    I: Iterator<Item=T>,
{
    /// Return the number of items evaluated. This value will be one more than
    ///     the highest index available via `MemoIter::recall()`.
    #[inline]
    pub fn evaluated(&self) -> usize {
        self.sequence.len()
    }

    fn expand_to_contain(&mut self, idx: usize) {
        if !self.exhausted {
            let len: usize = self.sequence.len();

            if idx >= len {
                self.sequence.reserve(idx - len + 1);

                for _i in len..=idx {
                    #[cfg(test)] println!("+ {}", _i);

                    match self.iterator.next() {
                        Some(next) => self.sequence.push(next),
                        None => {
                            self.exhausted = true;
                            self.sequence.shrink_to_fit();
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Retrieve, by its index, a value returned by the Iterator. If the value
    ///     at the index given has not yet been evaluated, it will be. Returns
    ///     `None` if the internal Iterator terminates before reaching the given
    ///     index.
    pub fn get(&mut self, idx: usize) -> Option<&T> {
        #[cfg(test)] println!("get({}):", idx);
        self.expand_to_contain(idx);
        self.sequence.get(idx)
    }

    /// Retrieve a slice of values returned by the Iterator. If the values in
    ///     the range in question have not yet been evaluated, they will be.
    ///
    /// Retrieving a slice whose end bound has not yet been evaluated will cause
    ///     all values up to that point to be evaluated. Because an Iterator may
    ///     be infinite, an *unbounded* slice will, out of caution, **not** do
    ///     any evaluations, instead being limited to the existing indices in
    ///     the stored sequence. A convenient side effect of this is that a full
    ///     range -- that is, `memiter.get_slice(..)` -- will return a full
    ///     slice of the *stored* sequence, doing no new evaluations.
    ///
    /// However, because the final index may not be knowable, this method also
    ///     includes a check to ensure that it will not panic if given a range
    ///     with indices outside the final sequence, instead returning an empty
    ///     slice.
    pub fn get_slice<R>(&mut self, range: R) -> &[T] where
        R: RangeBounds<usize> + SliceIndex<[T], Output=[T]>,
    {
        let first: usize = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i + 1,
        };

        match range.end_bound() {
            Bound::Unbounded => {
                let end: usize = self.sequence.len();

                &self.sequence[first.min(end)..end]
            }
            Bound::Included(&i) => {
                self.expand_to_contain(i);
                let last: usize = self.sequence.len().saturating_sub(1).min(i);

                if first <= last {
                    &self.sequence[first..=last]
                } else {
                    //  NOTE: Edge case. Prevents `&[0, 1, 2, 3][10..=20]` from
                    //      being evaluated as `&[3]`.
                    &[]
                }
            }
            Bound::Excluded(&i) => {
                self.expand_to_contain(i.saturating_sub(1));
                let end: usize = self.sequence.len().min(i);

                &self.sequence[first.min(end)..end]
            }
        }
    }

    /// Return `true` if the internal Iterator has been exhausted and is done
    ///     returning new values.
    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    /// Retrieve, by its index, a value returned by the Iterator. If the value
    ///     at the index given has not yet been evaluated, it will **NOT** be
    ///     evaluated now, and this method will return `None`.
    pub fn recall(&mut self, idx: usize) -> Option<&T> {
        #[cfg(test)] println!("recall({})", idx);
        self.sequence.get(idx)
    }

    /// Consume self, returning a Tuple containing the internal stored `Vec<T>`
    ///     and the original Iterator.
    pub fn consume(self) -> (Vec<T>, I) {
        let Self { sequence, iterator, .. } = self;
        (sequence, iterator)
    }
}


impl<I, T> AsRef<[T]> for MemoIter<I, T> where
    I: Iterator<Item=T>,
{
    fn as_ref(&self) -> &[T] {
        self.sequence.as_ref()
    }
}


impl<I, T> Deref for MemoIter<I, T> where
    I: Iterator<Item=T>,
{
    type Target = [T];

    /// A MemoIter dereferences to the slice of its stored values.
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.sequence[..]
    }
}


impl<I, T> ExactSizeIterator for MemoIter<I, T> where
    I: ExactSizeIterator + Iterator<Item=T>,
    T: Copy,
{
    #[inline]
    fn len(&self) -> usize {
        self.sequence.len() + self.iterator.len()
    }

    // #[cfg(exact_size_is_empty)]
    // fn is_empty(&self) -> bool {
    //     self.iterator.is_empty()
    // }
}


impl<F, I, T> From<F> for MemoIter<I, T> where
    F: IntoIterator<Item=T, IntoIter=I>,
    I: Iterator<Item=T>,
{
    fn from(into: F) -> Self {
        Self::new(into.into_iter())
    }
}


impl<I, T> Iterator for MemoIter<I, T> where
    I: Iterator<Item=T>,
    T: Copy,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.exhausted {
            match self.iterator.next() {
                Some(next) => {
                    self.sequence.push(next);
                    Some(next)
                }
                None => {
                    self.exhausted = true;
                    self.sequence.shrink_to_fit();
                    None
                }
            }
        } else { None }
    }
}


#[cfg(test)]
mod tests {
    use std::iter::successors;
    use super::*;

    #[test]
    fn test_factorial() {
        //  Instantiate a `MemoIter` that calculates factorials.
        let mut factorial: MemoIter<_, u32> = successors(
            Some((0, 1)),
            |&(idx0, acc)| {
                let idx1: u32 = idx0 + 1;
                Some((idx1, idx1 * acc))
            },
        ).map(|p| p.1).into();

        //  Ensure that it starts empty.
        assert_eq!(factorial.sequence, [], "MemoIter does not start empty.");
        assert_eq!(factorial.recall(3), None);

        //  Ensure that its specific values are calculated correctly.
        assert_eq!(factorial.get(0), Some(&1)); // 0!
        assert_eq!(factorial.get(1), Some(&1)); // 1!
        assert_eq!(factorial.get(4), Some(&24)); // 4!
        assert_eq!(factorial.get(6), Some(&720)); // 6!
        assert_eq!(factorial.get(4), Some(&24)); // 4!
        assert_eq!(factorial.get(2), Some(&2)); // 2!
        assert_eq!(factorial.get(0), Some(&1)); // 0!

        assert_eq!(factorial.recall(3), Some(&6));
        println!("{:?}", &factorial);

        assert_eq!(factorial.get_slice(..), [1, 1, 2, 6, 24, 120, 720]);
        assert_eq!(factorial.get_slice(0..4), [1, 1, 2, 6]);
        assert_eq!(factorial.get_slice(..=7), [1, 1, 2, 6, 24, 120, 720, 5040]);
        assert_eq!(factorial.get_slice(5..8), [120, 720, 5040]);

        //  Ensure that it maintains its returns, in order.
        let (seq, _) = factorial.consume();
        assert_eq!(
            seq, [1, 1, 2, 6, 24, 120, 720, 5040],
            "MemoIter does not correctly store its past values.",
        );
    }

    #[test]
    fn test_fibonacci() {
        let mut fibonacci: MemoIter<_, u32> = successors(
            Some((0, 1)),
            |&(a, b)| Some((b, b + a)),
        ).map(|p| p.0).into();

        assert_eq!(fibonacci.get(0), Some(&0));
        assert_eq!(fibonacci.get(1), Some(&1));
        assert_eq!(fibonacci.get(4), Some(&3));
        assert_eq!(fibonacci.get(9), Some(&34));

        println!("{:?}", &fibonacci);

        let (seq, _) = fibonacci.consume();
        assert_eq!(seq, [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
    }

    #[test]
    fn test_len() {
        let mut five = MemoIter::new(0..5);

        assert!(!five.is_exhausted());
        assert_eq!(five.evaluated(), 0);
        assert_eq!(five.len(), 5);
        assert_eq!(five.get(3), Some(&3));

        assert!(!five.is_exhausted());
        assert_eq!(five.evaluated(), 4);
        assert_eq!(five.len(), 5);
        assert_eq!(five.get(7), None);

        assert!(five.is_exhausted());
        assert_eq!(five.evaluated(), 5);
        assert_eq!(five.len(), 5);
    }

    #[test]
    fn test_prevec() {
        let mut five = MemoIter::with_vec(1..5, vec![0]);

        assert!(!five.is_exhausted());
        assert_eq!(five.len(), 5);
        assert_eq!(five.evaluated(), 1);
        assert_eq!(five.recall(0), Some(&0));
        assert_eq!(five.recall(1), None);
        assert_eq!(five.get_slice(..), [0]);

        assert_eq!(five.get(10), None);

        assert!(five.is_exhausted());
        assert_eq!(five.len(), 5);
        assert_eq!(five.evaluated(), 5);
        assert_eq!(five.recall(0), Some(&0));
        assert_eq!(five.recall(1), Some(&1));
        assert_eq!(five.get_slice(..), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_slice() {
        let mut five = MemoIter::new(0..5);

        assert!(!five.is_exhausted());
        assert_eq!(five.evaluated(), 0);

        assert_eq!(five.get_slice(..), []);
        assert_eq!(five.get_slice(..0), []);
        assert_eq!(five.get_slice(..=0), [0]);
        assert_eq!(five.get_slice(0..1), [0]);
        assert_eq!(five.get_slice(0..), [0]);
        assert_eq!(five.get_slice(..), [0]);

        assert!(!five.is_exhausted());
        assert_eq!(five.evaluated(), 1);

        assert_eq!(five.get_slice(10..20), []);
        assert_eq!(five.get_slice(4..=20), [4]);
        assert_eq!(five.get_slice(10..=20), []);
        assert_eq!(five.get_slice(..20), [0, 1, 2, 3, 4]);
        assert_eq!(five.get_slice(..=9), [0, 1, 2, 3, 4]);
        assert_eq!(five.get_slice(10..), []);
        assert_eq!(five.get_slice(..), [0, 1, 2, 3, 4]);

        assert_eq!(five.get_slice(..=usize::MAX), [0, 1, 2, 3, 4]);
        assert_eq!(five.get_slice(50..40), []);

        assert!(five.is_exhausted());
        assert_eq!(five.evaluated(), 5);
        assert_eq!(five.len(), 5);

        assert_eq!(*five, [0, 1, 2, 3, 4]);
        assert_eq!(five[..], [0, 1, 2, 3, 4]);
    }
}
