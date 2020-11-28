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
/// let (seq, _) = fibonacci.take();
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

    fn expand_to_contain(&mut self, idx: usize) {
        if !self.exhausted {
            let len: usize = self.sequence.len();

            if idx >= len {
                self.sequence.reserve(idx - len + 1);

                for i in len..=idx {
                    #[cfg(test)] println!("+ {}", i);

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
    ///     at the index given has not yet been evaluated, it will be.
    pub fn get(&mut self, idx: usize) -> Option<&T> {
        #[cfg(test)] println!("get({}):", idx);
        self.expand_to_contain(idx);
        self.sequence.get(idx)
    }

    /// Consume self, returning a Tuple containing the internal stored `Vec<T>`
    ///     and the original Iterator.
    pub fn take(self) -> (Vec<T>, I) {
        let Self { sequence, iterator, .. } = self;
        (sequence, iterator)
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

        //  Ensure that its specific values are calculated correctly.
        assert_eq!(factorial.get(0), Some(&1)); // 0!
        assert_eq!(factorial.get(1), Some(&1)); // 1!
        assert_eq!(factorial.get(4), Some(&24)); // 4!
        assert_eq!(factorial.get(6), Some(&720)); // 6!
        assert_eq!(factorial.get(4), Some(&24)); // 4!
        assert_eq!(factorial.get(2), Some(&2)); // 2!
        assert_eq!(factorial.get(0), Some(&1)); // 0!

        println!("{:?}", &factorial);

        //  Ensure that it maintains its returns, in order.
        let (seq, _) = factorial.take();
        assert_eq!(
            seq, [1, 1, 2, 6, 24, 120, 720],
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

        let (seq, _) = fibonacci.take();
        assert_eq!(seq, [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
    }

    #[test]
    fn test_len() {
        assert_eq!(MemoIter::new(0..5).len(), 5);
        assert_eq!(*MemoIter::new(0..=5).get(5).unwrap(), 5);

        assert!(MemoIter::new(0..5).get(7).is_none());
    }
}
