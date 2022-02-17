use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;

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
#[derive(Default)]
pub struct MonoBox<T> {
    ptr_or_null: AtomicPtr<T>,
}

impl<T> MonoBox<T> {
    /// Returns a fresh [`MonoBox`] that holds `inner`.
    ///
    /// Use [`Default::default()`] or [`MonoBox::empty()`] for a
    /// [`None`] initial value.
    #[inline(always)]
    pub fn new(inner: Option<Box<T>>) -> Self {
        let ptr = inner.map(Box::into_raw).unwrap_or_else(std::ptr::null_mut);

        Self {
            ptr_or_null: AtomicPtr::new(ptr),
        }
    }

    /// Returns a fresh [`MonoBox`] that holds [`None`].
    #[inline(always)]
    pub fn empty() -> Self {
        Self::new(None)
    }

    /// Returns whether the [`MonoBox`]'s value is [`None`].
    #[inline(always)]
    pub fn is_none(&self) -> bool {
        self.ptr_or_null.load(Ordering::Relaxed).is_null()
    }

    /// Returns whether the [`MonoBox`]'s value is [`Some`].
    #[inline(always)]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Returns the value previously stored in this [`MonoBox`] and
    /// replaces it with `value`.
    #[inline(always)]
    pub fn swap(&mut self, value: Option<Box<T>>) -> Option<Box<T>> {
        let new = value.map(Box::into_raw).unwrap_or_else(std::ptr::null_mut);
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
            std::ptr::null_mut(),
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
    #[inline(always)]
    pub fn as_ref(&self) -> Option<&T> {
        let ptr = self.ptr_or_null.load(Ordering::Acquire);
        unsafe { ptr.as_ref() }
    }

    /// Gets the value stored in this [`MonoBox`], if any.
    #[inline(always)]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        let ptr = self.ptr_or_null.load(Ordering::Acquire);
        unsafe { ptr.as_mut() }
    }

    /// Takes the value out of this [`MonoBox`], leaving a [`None`] in
    /// its place.
    #[inline(always)]
    pub fn take(&mut self) -> Option<Box<T>> {
        self.swap(None)
    }

    /// Consumes this [`MonoBox`], returning the wrapped value, if
    /// any.
    #[inline(always)]
    pub fn into_inner(mut self) -> Option<Box<T>> {
        self.take()
    }
}

impl<T> Drop for MonoBox<T> {
    fn drop(&mut self) {
        std::mem::drop(self.take())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for MonoBox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.as_ref(), f)
    }
}

impl<T> std::fmt::Pointer for MonoBox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&(self.ptr_or_null.load(Ordering::Relaxed) as *const T), f)
    }
}

impl<T: std::ops::Deref> MonoBox<T> {
    #[inline(always)]
    pub fn as_deref(&self) -> Option<&T::Target> {
        self.as_ref().map(|t| t.deref())
    }
}

impl<T: std::ops::DerefMut> MonoBox<T> {
    #[inline(always)]
    pub fn as_deref_mut(&mut self) -> Option<&mut T::Target> {
        self.as_mut().map(|t| t.deref_mut())
    }
}

impl<'a, T> From<&'a MonoBox<T>> for Option<&'a T> {
    #[inline(always)]
    fn from(mono: &'a MonoBox<T>) -> Option<&T> {
        mono.as_ref()
    }
}

impl<'a, T> From<&'a mut MonoBox<T>> for Option<&'a mut T> {
    #[inline(always)]
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

impl<T> From<MonoBox<T>> for Option<T> {
    fn from(mono: MonoBox<T>) -> Option<T> {
        mono.into_inner().map(|b| *b)
    }
}

impl<T> From<Box<T>> for MonoBox<T> {
    #[inline(always)]
    fn from(value: Box<T>) -> MonoBox<T> {
        MonoBox::new(Some(value))
    }
}

impl<T> From<Option<Box<T>>> for MonoBox<T> {
    #[inline(always)]
    fn from(value: Option<Box<T>>) -> MonoBox<T> {
        MonoBox::new(value)
    }
}

impl<T> From<MonoBox<T>> for Option<Box<T>> {
    #[inline(always)]
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
}

#[test]
fn test_default() {
    let mono: MonoBox<()> = Default::default();

    assert!(mono.is_none());
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
fn test_swap() {
    let mut mono: MonoBox<Vec<usize>> = Default::default();

    assert_eq!(mono.store(Box::new(vec![1])), Ok(()));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert_eq!(mono.store(Box::new(vec![2])), Err(Box::new(vec![2])));
    assert_eq!(mono.as_ref().unwrap(), &[1]);

    assert_eq!(mono.swap(Some(Box::new(vec![2]))), Some(Box::new(vec![1])));

    assert_eq!(mono.take(), Some(Box::new(vec![2])));

    assert!(mono.is_none());
}

#[test]
fn test_fmt() {
    let mono = MonoBox::<()>::empty();

    assert_eq!(format!("{:?}", &mono), "None");
    eprintln!("as a pointer: {:p}", &mono);
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

    let mono: MonoBox<String> = Box::new("bar".to_string()).into();
    assert_eq!(mono.as_deref(), Some("bar"));

    let mono: MonoBox<String> = Option::<Box<String>>::None.into();
    let val: Option<Box<String>> = mono.into();
    assert_eq!(val, None);

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
}
