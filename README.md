# MemoIter

A small Rust library implementing a Memoized Iterator, which pairs a normal `Iterator<Item=T>` with an owned `Vec<T>` to store the items it returns. Alternatively, it may be considered as a wrapped `Vec<T>` that is lazily populated with items from the Iterator.

This is useful for infinite Iterators where each value depends on the last, such as the factorial function: Calculating the factorial of `1000` is quite expensive, but it also includes, as byproducts, the factorials of `999`, `998`, and so on. If these are stored, they can be retrieved later, without needing to be recalculated for their own sake.

## Example

The following example shows a `MemoIter` being used to cache the results of calculating the Fibonacci Sequence.

```rust
use memoiter::MemoIter;
use std::iter::successors;


fn main() {
    let mut fibonacci: MemoIter<_, u32> = successors(
        Some((0, 1)),
        |&(a, b)| Some((b, b + a)),
    ).map(|p| p.0).into();

    assert_eq!(fibonacci.get(0), Some(&0));
    assert_eq!(fibonacci.get(1), Some(&1));
    assert_eq!(fibonacci.get(4), Some(&3));
    assert_eq!(fibonacci.get(9), Some(&34));

    //  This value was calculated as a byproduct of calculating 4, and is simply
    //      retrieved here.
    assert_eq!(fibonacci.get(3), Some(&2));

    let (seq, _) = fibonacci.take();
    assert_eq!(seq, [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
}
```
