extern crate alloc;

use alloc::boxed::Box;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering;

/// A [`MonoBox<T>`] is an atomic, lock-free, write-once
/// [`Option<Box<T>>`].  Write-once means that a [`MonoBox`] can only
/// transition from [`None`] to [`Some<Box<T>>`] once, and is then
/// frozen in that state until destruction.
///
/// As a special case, when one has exclusive ownership over the
/// [`MonoBox`] (evidenced by a `&mut` reference), it is possible to
/// [`MonoBox::swap`] its contents with an arbitrary
/// [`Option<Box<T>>`].  This non-monotonic operation is safe because
/// the mutable references guarantees no other thread can observe the
/// transition.
pub struct MonoBox<T> {
    ptr_or_null: AtomicPtr<T>,
}

impl<T> MonoBox<T> {
    /// Returns a fresh [`MonoBox`] that holds `inner`.
    ///
    /// Use [`Default::default()`] or [`MonoBox::empty()`] for a
    /// [`None`] initial value.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn new(inner: Option<Box<T>>) -> Self {
        let ptr = inner.map(Box::into_raw).unwrap_or_else(core::ptr::null_mut);

        Self {
            ptr_or_null: AtomicPtr::new(ptr),
        }
    }

    /// Returns a fresh [`MonoBox`] that holds [`None`].
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn empty() -> Self {
        Self::new(None)
    }

    /// Returns whether the [`MonoBox`]'s value is [`None`].
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn is_none(&self) -> bool {
        self.ptr_or_null.load(Ordering::Relaxed).is_null()
    }

    /// Returns whether the [`MonoBox`]'s value is [`Some`].
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Returns the value previously stored in this [`MonoBox`] and
    /// replaces it with `value`.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn swap(&mut self, value: Option<Box<T>>) -> Option<Box<T>> {
        let new = value.map(Box::into_raw).unwrap_or_else(core::ptr::null_mut);
        // We should be able to use `Relaxed` loads and store here,
        // and rely on the ordering that guarantees `self` is `&mut`.
        // However, it's more obviously safe when every load and store
        // can be matched as acquires and releases.
        let old = self.ptr_or_null.load(Ordering::Acquire);

        // We don't need or want an atomic swap here: `&mut`
        // guarantees exclusive ownership.
        self.ptr_or_null.store(new, Ordering::Release);
        if old.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(old) })
        }
    }

    /// Attempts to store `value` in this [`MonoBox`].  The operation
    /// succeeds iff it upgrades the [`MonoBox`] from [`None`] to
    /// [`Some`].
    ///
    /// Returns [`Ok`] when the store succeeds, and passes back `value`
    /// as [`Err`] otherwise.
    pub fn store(&self, value: Box<T>) -> Result<(), Box<T>> {
        let ptr = Box::into_raw(value);

        match self.ptr_or_null.compare_exchange(
            core::ptr::null_mut(),
            ptr,
            Ordering::Release,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(unsafe { Box::from_raw(ptr) }),
        }
    }

    /// Attempts to store `value` in this [`MonoBox`].
    ///
    /// Returns true on success and false if there was already some
    /// value in the [`MonoBox`].
    pub fn store_value(&self, value: T) -> bool {
        self.store(Box::new(value)).is_ok()
    }

    /// Gets the value stored in this [`MonoBox`], if any.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn as_ref(&self) -> Option<&T> {
        let ptr = self.ptr_or_null.load(Ordering::Acquire);
        unsafe { ptr.as_ref() }
    }

    /// Gets the value stored in this [`MonoBox`], if any.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        let ptr = self.ptr_or_null.load(Ordering::Acquire);
        unsafe { ptr.as_mut() }
    }

    /// Takes the value out of this [`MonoBox`], leaving a [`None`] in
    /// its place.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn take(&mut self) -> Option<Box<T>> {
        self.swap(None)
    }

    /// Consumes this [`MonoBox`], returning the wrapped value, if
    /// any.
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn into_inner(mut self) -> Option<Box<T>> {
        self.take()
    }
}

impl<T> Drop for MonoBox<T> {
    fn drop(&mut self) {
        core::mem::drop(self.take())
    }
}

impl<T> Default for MonoBox<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for MonoBox<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.as_ref(), f)
    }
}

impl<T> core::fmt::Pointer for MonoBox<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Pointer::fmt(&(self.ptr_or_null.load(Ordering::Relaxed) as *const T), f)
    }
}

impl<T: core::ops::Deref> MonoBox<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn as_deref(&self) -> Option<&T::Target> {
        self.as_ref().map(|t| t.deref())
    }
}

impl<T: core::ops::DerefMut> MonoBox<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    pub fn as_deref_mut(&mut self) -> Option<&mut T::Target> {
        self.as_mut().map(|t| t.deref_mut())
    }
}

impl<'a, T> From<&'a MonoBox<T>> for Option<&'a T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(mono: &'a MonoBox<T>) -> Option<&T> {
        mono.as_ref()
    }
}

impl<'a, T> From<&'a mut MonoBox<T>> for Option<&'a mut T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(mono: &'a mut MonoBox<T>) -> Option<&mut T> {
        mono.as_mut()
    }
}

impl<T> From<T> for MonoBox<T> {
    fn from(value: T) -> MonoBox<T> {
        MonoBox::new(Some(Box::new(value)))
    }
}

impl<T> From<Option<T>> for MonoBox<T> {
    fn from(value: Option<T>) -> MonoBox<T> {
        MonoBox::new(value.map(Box::new))
    }
}

impl<T> From<Box<T>> for MonoBox<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(value: Box<T>) -> MonoBox<T> {
        MonoBox::new(Some(value))
    }
}

impl<T> From<Option<Box<T>>> for MonoBox<T> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(value: Option<Box<T>>) -> MonoBox<T> {
        MonoBox::new(value)
    }
}

impl<T> From<MonoBox<T>> for Option<T> {
    fn from(mono: MonoBox<T>) -> Option<T> {
        mono.into_inner().map(|b| *b)
    }
}

impl<T> From<MonoBox<T>> for Option<Box<T>> {
    #[cfg_attr(not(tarpaulin), inline(always))]
    fn from(mono: MonoBox<T>) -> Option<Box<T>> {
        mono.into_inner()
    }
}

#[test]
fn test_none() {
    let mut mono = MonoBox::<()>::empty();

    assert!(mono.is_none());
    assert!(!mono.is_some());

    assert!(mono.as_ref().is_none());
    assert!(mono.as_mut().is_none());
    assert_eq!(mono.into_inner(), None);
}

#[test]
fn test_some() {
    let mut mono = MonoBox::new(Some(Box::new(vec![1])));

    assert!(!mono.is_none());
    assert!(mono.is_some());

    assert_eq!(mono.as_ref().unwrap(), &[1]);

    mono.as_mut().unwrap().push(2);
    assert_eq!(mono.as_mut().unwrap(), &[1, 2]);

    assert_eq!(mono.into_inner(), Some(Box::new(vec![1, 2])));
}

#[test]
fn test_default() {
    let mono: MonoBox<()> = Default::default();

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
    let mono = MonoBox::new(Some(Box::new(DropTracker { counter: &counter })));

    assert_eq!(counter.load(Ordering::Relaxed), 0);
    std::mem::drop(mono);
    assert_eq!(counter.load(Ordering::Relaxed), 1);

    // Make sure `into_inner` doesn't double-drop.
    let mono = MonoBox::new(Some(Box::new(DropTracker { counter: &counter })));

    assert_eq!(counter.load(Ordering::Relaxed), 1);
    std::mem::drop(mono.into_inner());
    assert_eq!(counter.load(Ordering::Relaxed), 2);

    // Check that an empty `MonoBox` doesn't drop anything.
    let mono: MonoBox<DropTracker> = Default::default();
    assert_eq!(counter.load(Ordering::Relaxed), 2);
    std::mem::drop(mono);
    assert_eq!(counter.load(Ordering::Relaxed), 2);
}

#[test]
fn test_upgrade() {
    let mono: MonoBox<Vec<usize>> = Default::default();

    assert_eq!(mono.store(Box::new(vec![1])), Ok(()));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert_eq!(mono.store(Box::new(vec![2])), Err(Box::new(vec![2])));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert!(!mono.store_value(vec![3]));
    assert_eq!(mono.as_ref().unwrap(), &[1]);
}

#[test]
fn test_store_value() {
    let mono: MonoBox<Vec<usize>> = Default::default();

    assert!(mono.store_value(vec![1]));
    assert_eq!(mono.as_ref().unwrap(), &[1]);
}

#[test]
fn test_swap() {
    let mut mono: MonoBox<Vec<usize>> = Default::default();

    assert_eq!(mono.store(Box::new(vec![1])), Ok(()));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert_eq!(mono.store(Box::new(vec![2])), Err(Box::new(vec![2])));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert_eq!(mono.swap(Some(Box::new(vec![2]))), Some(Box::new(vec![1])));

    assert_eq!(mono.take(), Some(Box::new(vec![2])));

    assert!(mono.is_none());

    assert_eq!(mono.take(), None);
}

#[test]
fn test_fmt() {
    let mono = MonoBox::<()>::empty();

    assert_eq!(format!("{:?}", &mono), "None");
    assert_eq!(format!("as a pointer: {:p}", mono), "as a pointer: 0x0");
}

#[test]
fn test_conversions() {
    let mono: MonoBox<_> = Option::<String>::None.into();
    let opt_ref: Option<&str> = None;
    let opt_val: Option<&String> = None;

    assert_eq!(mono.as_deref(), opt_ref);

    {
        let as_string: Option<&String> = (&mono).into();
        assert_eq!(as_string, opt_val);
    }

    let mono: MonoBox<_> = "foo".to_string().into();
    assert_eq!(mono.as_deref(), Some("foo"));

    let mono: MonoBox<_> = Some("foo".to_string()).into();
    {
        let as_string: Option<&String> = (&mono).into();
        assert_eq!(as_string, Some(&"foo".to_string()));

        let to_string: Option<String> = mono.into();
        assert_eq!(to_string, Some("foo".to_string()));
    }

    let mono: MonoBox<String> = Box::new("bar".to_string()).into();
    assert_eq!(mono.as_deref(), Some("bar"));

    let mono: MonoBox<String> = Option::<Box<String>>::None.into();
    let val: Option<Box<String>> = mono.into();
    assert_eq!(val, None);

    let mono: MonoBox<String> = Some(Box::new("baz".to_string())).into();
    assert_eq!(mono.as_deref(), Some("baz"));
    let val: Option<Box<String>> = mono.into();
    assert_eq!(val, Some(Box::new("baz".to_string())));

    let boxed = MonoBox::<String>::empty();
    let mono: MonoBox<String> = boxed.into();
    assert!(mono.is_none());

    let _val: Option<String> = mono.into();
}

#[test]
fn test_mut_conversions() {
    let mut mono: MonoBox<Vec<u8>> = Default::default();
    let empty: Option<&mut [u8]> = None;

    let as_mut: Option<&mut [u8]> = mono.as_deref_mut();
    assert_eq!(as_mut, empty);

    let as_mut: Option<&mut Vec<u8>> = (&mut mono).into();
    assert_eq!(as_mut, None);

    let mut mono: MonoBox<Vec<u8>> = vec![1].into();
    let mut vec = vec![1];
    let some: Option<&mut [u8]> = Some(&mut vec);

    let as_mut: Option<&mut [u8]> = mono.as_deref_mut();
    assert_eq!(as_mut, some);

    let as_mut: Option<&mut Vec<u8>> = (&mut mono).into();
    assert_eq!(as_mut, Some(&mut vec));
}
