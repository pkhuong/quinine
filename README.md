Quinine
=======
[![Crates.io](https://img.shields.io/crates/v/quinine)](https://crates.io/crates/quinine) [![docs.rs](https://img.shields.io/docsrs/quinine)](https://docs.rs/quinine)

Quinine implements atomic, lock-free, but write-once versions of
containers like `Option<Box<T>>` (`MonoBox`) and `Option<Arc<T>>`
(`MonoArc`).

These write-once containers can be read with mere `Ordering::Acquire`
loads; reads otherwise perform like `Box` and `Arc`.  On the
write-side, atomic updates happen via compare-and-swaps, only the
first of which will succeed.

The code is simpler (and likely faster) than, e.g.,
[ArcSwap](https://crates.io/crates/arc-swap), because it exploits the
monotonic nature of all updates to these write-once containers.
That's particularly true for reads, but updates also avoid a lot of
coordination overhead with potential concurrent readers.

When atomic containers can only transition monotonically from `None`
to `Some`, and then stay there, we can implement simple update
algorithms, without having to worry about the lifetime of references
derived from the container: once we've observed `Some` value, the
container can be trusted to keep it alive for us (until the container
is dropped safely). The general form of this trick applies to any
container that owns a monotonically increasing set of resources, until
the container itself is destroyed.
