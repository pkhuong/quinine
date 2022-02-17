Quinine
=======

Quinine implements atomic, lock-free, but write-once versions of
containers like `Option<Box<T>>` (`MonoBox`) and `Option<Arc<T>>`
(`MonoArc`).

These write-once container can be read with mere `Ordering::Acquire`
loads, and otherwise perform like `Box` and `Arc`.  On the write-side,
atomic updates happen with a compare-and-swap, only the first of which
will succeed.
