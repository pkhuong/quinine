extern crate alloc;

use alloc::sync::Arc;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering;

use crate::MonoBox;

/// A [`MonoArc<T>`] is an atomic, lock-free, write-once
/// [`Option<Arc<T>>`].  Write-once means that a [`MonoArc`] can only
/// transition from [`None`] to [`Some<Arc<T>>`] once, and is then
/// frozen in that state until destruction.
///
/// As a special case, when one has exclusive ownership over the
/// [`MonoArc`] (evidenced by a `&mut` reference), it is possible to
/// [`MonoArc::swap`] its contents with an arbitrary
/// [`Option<Arc<T>>`].  This non-monotonic operation is safe because
/// the mutable references guarantees no other thread can observe the
/// transition.
pub struct MonoArc<T> {
    ptr_or_null: AtomicPtr<T>,
}

impl<T> MonoArc<T> {
    /// Returns a fresh [`MonoArc`] that holds `inner`.
    ///
    /// Use [`Default::default()`] or [`MonoArc::empty()`] for a
    /// [`None`] initial value.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn new(inner: Option<Arc<T>>) -> Self {
        let ptr = inner.map(Arc::into_raw).unwrap_or_else(core::ptr::null);

        Self {
            ptr_or_null: AtomicPtr::new(ptr as *mut _),
        }
    }

    /// Returns a fresh [`MonoArc`] that holds [`None`].
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn empty() -> Self {
        Self::new(None)
    }

    /// Returns whether the [`MonoArc`]'s value is [`None`].
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn is_none(&self) -> bool {
        self.ptr_or_null.load(Ordering::Relaxed).is_null()
    }

    /// Returns whether the [`MonoArc`]'s value is [`Some`].
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Returns the value previously stored in this [`MonoArc`] and
    /// replaces it with `value`.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn swap(&mut self, value: Option<Arc<T>>) -> Option<Arc<T>> {
        let new = value.map(Arc::into_raw).unwrap_or_else(core::ptr::null);
        // We should be able to use `Relaxed` loads and store here,
        // and rely on the ordering that guarantees `self` is `&mut`.
        // However, it's more obviously safe when every load and store
        // can be matched as acquires and releases.
        let old = self.ptr_or_null.load(Ordering::Acquire);

        // We don't need or want an atomic swap here: `&mut`
        // guarantees exclusive ownership.
        self.ptr_or_null.store(new as *mut T, Ordering::Release);
        if old.is_null() {
            None
        } else {
            Some(unsafe { Arc::from_raw(old as *const T) })
        }
    }

    /// Attempts to store `value` in this [`MonoArc`].  The operation
    /// succeeds iff it upgrades the [`MonoArc`] from [`None`] to
    /// [`Some`].
    ///
    /// Returns [`Ok`] when the store succeeds, and passes back
    /// `value` as [`Err`] otherwise.
    pub fn store(&self, value: Arc<T>) -> Result<(), Arc<T>> {
        let ptr = Arc::into_raw(value);

        match self.ptr_or_null.compare_exchange(
            core::ptr::null_mut(),
            ptr as *mut _,
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(unsafe { Arc::from_raw(ptr) }),
        }
    }

    /// Attempts to store `value` in this [`MonoArc`].
    ///
    /// Returns true on success and false if there already was some
    /// value in the [`MonoArc`].
    pub fn store_value(&self, value: T) -> bool {
        self.store(Arc::new(value)).is_ok()
    }

    /// Gets the value stored in this [`MonoArc`], if any.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn as_ref(&self) -> Option<&T> {
        let ptr = self.ptr_or_null.load(Ordering::Acquire);
        unsafe { ptr.as_ref() }
    }

    /// Gets a clone of the [`Arc`] stored in this [`MonoArc`], if any.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn get(&self) -> Option<Arc<T>> {
        let ptr = self.ptr_or_null.load(Ordering::Acquire) as *const T;

        if ptr.is_null() {
            None
        } else {
            Some(unsafe {
                Arc::increment_strong_count(ptr);
                Arc::from_raw(ptr)
            })
        }
    }

    /// Takes the value out of this [`MonoArc`], leaving a [`None`] in
    /// its place.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn take(&mut self) -> Option<Arc<T>> {
        self.swap(None)
    }

    /// Consumes this [`MonoArc`], returning the wrapped value, if any.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn into_inner(mut self) -> Option<Arc<T>> {
        self.take()
    }
}

impl<T> Drop for MonoArc<T> {
    fn drop(&mut self) {
        core::mem::drop(self.take());
    }
}

impl<T> Default for MonoArc<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> Clone for MonoArc<T> {
    fn clone(&self) -> MonoArc<T> {
        let ptr = self.ptr_or_null.load(Ordering::Acquire);

        if !ptr.is_null() {
            unsafe { Arc::increment_strong_count(ptr as *const T) };
        }

        MonoArc {
            ptr_or_null: AtomicPtr::new(ptr),
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for MonoArc<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_ref(), f)
    }
}

impl<T> core::fmt::Pointer for MonoArc<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&(self.ptr_or_null.load(Ordering::Relaxed) as *const T), f)
    }
}

impl<T: core::ops::Deref> MonoArc<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn as_deref(&self) -> Option<&T::Target> {
        self.as_ref().map(|t| t.deref())
    }
}

impl<'a, T> From<&'a MonoArc<T>> for Option<&'a T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(mono: &'a MonoArc<T>) -> Option<&T> {
        mono.as_ref()
    }
}

impl<T> From<T> for MonoArc<T> {
    fn from(value: T) -> MonoArc<T> {
        MonoArc::new(Some(Arc::new(value)))
    }
}

impl<T> From<Option<T>> for MonoArc<T> {
    fn from(value: Option<T>) -> MonoArc<T> {
        MonoArc::new(value.map(Arc::new))
    }
}

impl<T> From<Arc<T>> for MonoArc<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(value: Arc<T>) -> MonoArc<T> {
        MonoArc::new(Some(value))
    }
}

impl<T> From<Option<Arc<T>>> for MonoArc<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(value: Option<Arc<T>>) -> MonoArc<T> {
        MonoArc::new(value)
    }
}

impl<T> From<MonoArc<T>> for Option<Arc<T>> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(mono: MonoArc<T>) -> Option<Arc<T>> {
        mono.into_inner()
    }
}

impl<T> From<MonoBox<T>> for MonoArc<T> {
    fn from(mono: MonoBox<T>) -> MonoArc<T> {
        MonoArc::new(mono.into_inner().map(Into::into))
    }
}

#[test]
fn test_none() {
    let mono = MonoArc::<()>::empty();

    assert!(mono.is_none());
    assert!(!mono.is_some());

    assert!(mono.as_ref().is_none());
    assert!(mono.get().is_none());
    assert_eq!(mono.into_inner(), None);
}

#[test]
fn test_some() {
    let mono = MonoArc::new(Some(Arc::new(vec![1])));

    assert!(!mono.is_none());
    assert!(mono.is_some());

    assert_eq!(mono.as_ref().unwrap(), &[1]);
    assert_eq!(&*mono.get().unwrap(), &[1]);
    assert_eq!(mono.clone().as_ref().unwrap(), &[1]);

    assert_eq!(mono.into_inner(), Some(Arc::new(vec![1])));
}

#[test]
fn test_default() {
    let mono: MonoArc<()> = Default::default();

    assert!(mono.is_none());
}

#[test]
fn test_drop() {
    use std::sync::atomic::AtomicUsize;

    struct DropTracker<'a> {
        counter: &'a AtomicUsize,
    }

    impl Drop for DropTracker<'_> {
        fn drop(&mut self) {
            self.counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    let counter = AtomicUsize::new(0);
    let mono = MonoArc::new(Some(Arc::new(DropTracker { counter: &counter })));
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    // These refcount increments and decrements should not trigger
    // destruction.
    std::mem::drop(mono.get());
    assert_eq!(counter.load(Ordering::Relaxed), 0);
    std::mem::drop(mono.clone());
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    // Now we should destroy.
    std::mem::drop(mono);
    assert_eq!(counter.load(Ordering::Relaxed), 1);

    // Make sure `into_inner` doesn't double-drop.
    let mono = MonoArc::new(Some(Arc::new(DropTracker { counter: &counter })));
    assert_eq!(counter.load(Ordering::Relaxed), 1);
    std::mem::drop(mono.into_inner());
    assert_eq!(counter.load(Ordering::Relaxed), 2);

    // Check that an empty `MonoArc` doesn't drop anything.
    let mono: MonoArc<DropTracker> = Default::default();
    assert_eq!(counter.load(Ordering::Relaxed), 2);
    std::mem::drop(mono.get());
    assert_eq!(counter.load(Ordering::Relaxed), 2);
    std::mem::drop(mono.clone());
    assert_eq!(counter.load(Ordering::Relaxed), 2);
    std::mem::drop(mono);
    assert_eq!(counter.load(Ordering::Relaxed), 2);
}

#[test]
fn test_upgrade() {
    let mono: MonoArc<Vec<usize>> = Default::default();

    assert_eq!(mono.store(Arc::new(vec![1])), Ok(()));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert_eq!(mono.store(Arc::new(vec![2])), Err(Arc::new(vec![2])));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert!(!mono.store_value(vec![3]));
    assert_eq!(mono.as_ref().unwrap(), &[1]);
}

#[test]
fn test_store_value() {
    let mono: MonoArc<Vec<usize>> = Default::default();

    assert!(mono.store_value(vec![1]));
    assert_eq!(mono.as_ref().unwrap(), &[1]);
}

#[test]
fn test_swap() {
    let mut mono: MonoArc<Vec<usize>> = Default::default();

    assert_eq!(mono.store(Arc::new(vec![1])), Ok(()));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert_eq!(mono.store(Arc::new(vec![2])), Err(Arc::new(vec![2])));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert_eq!(mono.swap(Some(Arc::new(vec![2]))), Some(Arc::new(vec![1])));

    assert_eq!(mono.take(), Some(Arc::new(vec![2])));

    assert!(mono.is_none());

    assert_eq!(mono.take(), None);
}

#[test]
fn test_fmt() {
    let mono = MonoArc::<()>::empty();

    assert_eq!(format!("{:?}", &mono), "None");
    assert_eq!(format!("as a pointer: {:p}", mono), "as a pointer: 0x0");
}

#[test]
fn test_conversions() {
    let mono: MonoArc<_> = Option::<String>::None.into();
    let opt_ref: Option<&str> = None;
    let opt_val: Option<&String> = None;

    assert_eq!(mono.as_deref(), opt_ref);

    {
        let as_string: Option<&String> = (&mono).into();
        assert_eq!(as_string, opt_val);
    }

    let mono: MonoArc<_> = "foo".to_string().into();
    assert_eq!(mono.as_deref(), Some("foo"));
    let val: Option<&String> = (&mono).into();
    assert_eq!(val, Some(&"foo".to_string()));

    let mono: MonoArc<String> = Some("foo".to_string()).into();
    assert_eq!(mono.as_deref(), Some("foo"));

    let mono: MonoArc<String> = Arc::new("bar".to_string()).into();
    assert_eq!(mono.as_deref(), Some("bar"));

    let mono: MonoArc<String> = Some(Arc::new("bar".to_string())).into();
    assert_eq!(mono.as_deref(), Some("bar"));

    let mut mono: MonoArc<String> = Option::<Arc<String>>::None.into();
    mono.swap(Some(Arc::new("baz".to_string())));
    let val: Option<Arc<String>> = mono.into();
    assert_eq!(val, Some(Arc::new("baz".to_string())));

    let boxed = MonoBox::<String>::empty();
    let mono: MonoArc<String> = boxed.into();
    assert!(mono.is_none());

    let boxed: MonoBox<String> = "quux".to_string().into();
    let mono: MonoArc<String> = boxed.into();
    assert_eq!(mono.as_deref(), Some("quux"));
}
