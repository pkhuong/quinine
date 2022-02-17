//! Quinine implements atomic, lock-free, but write-once versions of
//! containers like [`Option<Box<T>>`] ([`MonoBox`]) and
//! [`Option<Arc<T>>`] ([`MonoArc`]).  Write-once means that the value
//! transitions from [`None`] to [`Some`] at most once during the
//! atomic object's lifetime, and the object is then frozen as is.
//! This monotonicity makes it easy to access monotonic containers
//! from multiple threads without any special coordination between
//! readers and writers.
//!
//! Crates like [ArcSwap](https://crates.io/crates/arc-swap) offer
//! optimised versions of [`RwLock<Arc<T>>`](std::sync::RwLock), for
//! read-mostly workloads.  Quinine's containers are even more heavily
//! biased away from writes ([`MonoBox`] and [`MonoArc`] can only be
//! mutated once), and offer even lower overhead in return: stores
//! require only a
//! [`AtomicPtr::compare_exchange`](std::sync::atomic::AtomicPtr::compare_exchange),
//! and reads are plain
//! [`Ordering::Acquire`](std::sync::atomic::Ordering) loads.  Of
//! course, obtaining a full-blown [`Arc`](std::sync::Arc) incurs
//! reference counting overhead, just like a regular
//! [`Arc::clone`](std::sync::Arc::clone).
//!
//! When containers are updated without locking, but only so long as
//! the set of resources (e.g., memory allocations) owned by that
//! container grows monotonically, we can implement simple update
//! algorithms based on compare-and-swap, without having to worry
//! about object lifetimes and concurrent readers. All references and
//! other shared capabilities readers might have obtained via a
//! monotonic container will remain valid as long as the container
//! itself is valid.
//!
//! For example, once we've observed a [`MonoBox`] with [`Some`]
//! value, we can safely use its pointee for however long we have a
//! reference to that [`MonoBox`] (something that Rust's ownership
//! system enforces for us): the [`MonoBox`]'s value is now
//! frozen, so the pointee's lifetime exactly matches the
//! [`MonoBox`]'s lifetime.
//!
//! Monotonic containers may only release resources or otherwise
//! change non-monotonically when a mutable reference (`&mut`) serves
//! as a witness of single ownership.  For example, that's how
//! containers can implement [`Drop::drop`].
mod arc;
mod r#box;

pub use arc::MonoArc;
pub use r#box::MonoBox;
