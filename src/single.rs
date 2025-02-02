//! An accessor to a single element

use {
    crate::{
        error::Error,
        mapper::Mapper,
        marker::{self, AccessorTypeSpecifier, Readable, Writable},
    },
    core::{fmt, hash::Hash, marker::PhantomData, mem, ptr},
};

/// An alias of [`ReadWrite`].
#[deprecated(since = "0.3.2", note = "Use `ReadWrite`.")]
pub type Single<T, M> = ReadWrite<T, M>;

/// A readable and writable accessor.
pub type ReadWrite<T, M> = Generic<T, M, marker::ReadWrite>;

/// A read-only accessor.
pub type ReadOnly<T, M> = Generic<T, M, marker::ReadOnly>;

/// A write-only accessor.
pub type WriteOnly<T, M> = Generic<T, M, marker::WriteOnly>;

/// Combined with proc-macro [`BoundedStructuralOf`], this trait converts array accessors of field struct types into a struct of accessors with same field names.
///
/// This trait is intended to be implemented automatically by [`BoundedStructuralOf`] macro expansion. Users should not implement this manually.
///
/// # Examples
///
/// ```no_run
/// use accessor::mapper::Identity;
/// use accessor::BoundedStructuralOf;
/// use accessor::single::{BoundedStructural, BoundedStructuralMut};
///
/// #[repr(C)]
/// #[derive(Clone, Copy, BoundedStructuralOf)]
/// struct Foo {
///     x: u32,
///     y: u32,
/// }
///
/// // The above derivation creates a struct-of-accessor type called `BoundedStructuralOfFoo` which is roughly equivalent to:
/// // ```
/// // struct BoundedStructuralOfFoo {
/// //     x: accessor::single::ReadWrite::<u32, Identity>,
/// //     y: accessor::single::ReadWrite::<u32, Identity>,
/// // }
/// // ```
/// // The derivation also implements `BoundedStructural<Foo, M, A>` and `BoundedStructuralMut<Foo, M, A>` so that an `accessor::single::ReadWrite::<Foo, M>` instance
/// // can be accessed with a `BoundedStructuralOfFoo` item, which has a lifetime bound to the base accessor.
///
/// let mut a = unsafe { accessor::single::ReadWrite::<Foo, M>::new(0x1000, Identity) };
///
/// // read `x` field of the accessor.
/// let x = a.structural().x.read_volatile();
///
/// // write 5 as the `y` field of the accessor.
/// a.structural_at_mut(2).y.write_volatile(5);
///
/// ```
///
pub trait BoundedStructural<T, M, A>
where
    M: Mapper,
    A: Readable,
{
    /// The concrete type of the struct of accessors which `.structural_at(i)` returns.
    type BoundedStructuralType<'a>
    where
        Self: 'a;

    /// Returns bounded struct of read-only accessors.
    fn structural(&self) -> Self::BoundedStructuralType<'_>;
}

/// The mutable counterpart for [`BoundedStructural`].
/// See [`BoundedStructural`] for details.
pub trait BoundedStructuralMut<T, M, A>
where
    M: Mapper,
    A: Writable,
{
    /// The concrete type of the struct of accessors which `.structural_at_mut(i)` returns.
    type BoundedStructuralType<'a>
    where
        Self: 'a;

    /// Returns bounded struct of writable accessors.
    fn structural_mut(&mut self) -> Self::BoundedStructuralType<'_>;
}

/// An accessor to read, modify, and write a single value of memory.
///
/// `T` does not need to implement [`Copy`]. However, be careful that [`Generic::read_volatile`]
/// creates and [`Generic::write_volatile`] writes a bitwise copy of a value.
///
/// # Examples
///
/// ```no_run
/// use accessor::mapper::Mapper;
/// use accessor::single;
/// use core::num::NonZeroUsize;
///
/// struct M;
/// impl Mapper for M {
///     unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> NonZeroUsize {
///         todo!()
///     }
///
///     fn unmap(&mut self, phys_start: usize, bytes: usize) {
///         todo!()
///     }
/// }
///
/// let mapper = M;
///
/// // Create an accessor to the i32 value at the physical address 0x1000.
/// let mut a = unsafe { single::ReadWrite::<i32, M>::new(0x1000, mapper) };
///
/// // Read a value.
/// a.read_volatile();
///
/// // Write 42.
/// a.write_volatile(42);
///
/// // Update the value.
/// a.update_volatile(|v| {
///     *v *= 2;
/// });
/// ```
pub struct Generic<T, M, A>
where
    M: Mapper,
    A: AccessorTypeSpecifier,
{
    virt: usize,
    _marker: PhantomData<T>,
    _readable_writable: PhantomData<A>,
    mapper: M,
}
impl<T, M, A> Generic<T, M, A>
where
    M: Mapper,
    A: AccessorTypeSpecifier,
{
    /// Creates a new accessor to an element of type `T` at the physical address `phys_base`.
    ///
    /// # Safety
    ///
    /// The caller must ensure the following conditions:
    /// - The value at the physical address `phys_base` is valid.
    /// - Any other accessors except the one returned by this method must not access the value
    /// while the returned one lives.
    ///
    /// # Panics
    ///
    /// This method panics if `phys_base` is not aligned as the type `T` requires.
    pub unsafe fn new(phys_base: usize, mut mapper: M) -> Self {
        assert!(super::is_aligned::<T>(phys_base));

        let bytes = mem::size_of::<T>();
        let virt = mapper.map(phys_base, bytes).get();

        Self {
            virt,
            _marker: PhantomData,
            _readable_writable: PhantomData,
            mapper,
        }
    }

    /// Creates a new accessor to an element of type `T` at the physical address `phys_base`.
    ///
    /// # Safety
    ///
    /// The caller must ensure the following conditions:
    /// - The value at the physical address `phys_base` is valid.
    /// - Any other accessors except the one returned by this method must not access the value
    /// while the returned one lives.
    ///
    /// # Errors
    ///
    /// This method may return a [`Error::NotAligned`] error if `phys_base` is not aligned as the
    /// type `T` requires.
    pub unsafe fn try_new(phys_base: usize, mapper: M) -> Result<Self, Error> {
        if super::is_aligned::<T>(phys_base) {
            Ok(Self::new(phys_base, mapper))
        } else {
            Err(Error::NotAligned {
                alignment: mem::align_of::<T>(),
                address: phys_base,
            })
        }
    }

    /// Returns the virtual address of the item.
    ///
    /// This is public but hidden, since this method should be called in `accessor_macros::BoundedStructuralOf` proc-macro expansion.
    /// Users of this crate are not intended to call this directly.
    #[doc(hidden)]
    pub unsafe fn addr(&self) -> usize {
        self.virt
    }
}
impl<T, M, A> Generic<T, M, A>
where
    M: Mapper,
    A: Readable,
{
    /// Reads a value from the address that the accessor points to.
    pub fn read_volatile(&self) -> T {
        // SAFETY: `Accessor::new` ensures that `self.virt` is aligned properly.
        unsafe { ptr::read_volatile(self.virt as *const _) }
    }

    /// Alias of [`Generic::read_volatile`].
    #[deprecated(since = "0.3.1", note = "use `read_volatile`")]
    pub fn read(&self) -> T {
        self.read_volatile()
    }
}
impl<T, M, A> Generic<T, M, A>
where
    M: Mapper,
    A: Writable,
{
    /// Writes a value to the address that the accessor points to.
    pub fn write_volatile(&mut self, v: T) {
        // SAFETY: `Accessor::new` ensures that `self.virt` is aligned properly.
        unsafe {
            ptr::write_volatile(self.virt as *mut _, v);
        }
    }

    /// Alias of [`Generic::write_volatile`].
    #[deprecated(since = "0.3.1", note = "use `write_volatile`")]
    pub fn write(&mut self, v: T) {
        self.write_volatile(v);
    }
}
impl<T, M, A> Generic<T, M, A>
where
    M: Mapper,
    A: Readable + Writable,
{
    /// Updates a value that the accessor points by reading it, modifying it, and writing it.
    ///
    /// Note that some MMIO regions (e.g. the Command Ring Pointer field of the Command
    /// Ring Control Register of the xHCI) may return 0 regardless of the actual values of the
    /// fields. For these regions, this operation should be called only once.
    pub fn update_volatile<U>(&mut self, f: U)
    where
        U: FnOnce(&mut T),
    {
        let mut v = self.read_volatile();
        f(&mut v);
        self.write_volatile(v);
    }

    /// Alias of [`Generic::update_volatile`].
    #[deprecated(since = "0.3.1", note = "use `update_volatile`")]
    pub fn update<U>(&mut self, f: U)
    where
        U: FnOnce(&mut T),
    {
        self.update_volatile(f);
    }
}
impl<T, M, A> fmt::Debug for Generic<T, M, A>
where
    T: fmt::Debug,
    M: Mapper,
    A: Readable,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.read_volatile())
    }
}
impl<T, M, A> PartialEq for Generic<T, M, A>
where
    T: PartialEq,
    M: Mapper,
    A: Readable,
{
    fn eq(&self, other: &Self) -> bool {
        self.read_volatile().eq(&other.read_volatile())
    }
}
impl<T, M, A> Eq for Generic<T, M, A>
where
    T: Eq,
    M: Mapper,
    A: Readable,
{
}
impl<T, M, A> PartialOrd for Generic<T, M, A>
where
    T: PartialOrd,
    M: Mapper,
    A: Readable,
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.read_volatile().partial_cmp(&other.read_volatile())
    }
}
impl<T, M, A> Ord for Generic<T, M, A>
where
    T: Ord,
    M: Mapper,
    A: Readable,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.read_volatile().cmp(&other.read_volatile())
    }
}
impl<T, M, A> Hash for Generic<T, M, A>
where
    T: Hash,
    M: Mapper,
    A: Readable,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.read_volatile().hash(state);
    }
}
impl<T, M, A> Drop for Generic<T, M, A>
where
    M: Mapper,
    A: AccessorTypeSpecifier,
{
    fn drop(&mut self) {
        let bytes = mem::size_of::<T>();
        self.mapper.unmap(self.virt, bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::num::NonZeroUsize;

    struct M;
    impl Mapper for M {
        unsafe fn map(&mut self, phys_start: usize, _: usize) -> NonZeroUsize {
            NonZeroUsize::new(phys_start).unwrap()
        }

        fn unmap(&mut self, _: usize, _: usize) {}
    }

    #[test]
    fn test_read_volatile() {
        let v: u32 = 42;
        let a = unsafe { ReadWrite::<u32, M>::new(addr(&v), M) };

        assert_eq!(a.read_volatile(), 42);
    }

    #[test]
    fn test_write_volatile() {
        let mut v: u32 = 0;
        let mut a = unsafe { ReadWrite::<u32, M>::new(addr(&mut v), M) };

        a.write_volatile(42);
        assert_eq!(v, 42);
    }

    #[test]
    fn test_update_volatile() {
        let mut v: u32 = 42;
        let mut a = unsafe { ReadWrite::<u32, M>::new(addr(&mut v), M) };

        a.update_volatile(|v| *v *= 2);
        assert_eq!(v, 84);
    }

    #[test]
    #[should_panic]
    fn test_not_aligned() {
        let v: u32 = 42;

        let _ = unsafe { ReadWrite::<u32, M>::new(addr(&v) + 1, M) };
    }

    #[test]
    fn test_err_not_aligned() {
        let v: u32 = 42;

        let r = unsafe { ReadWrite::<u32, M>::try_new(addr(&v) + 1, M) };
        assert_eq!(
            r,
            Err(Error::NotAligned {
                alignment: mem::align_of::<u32>(),
                address: addr(&v) + 1,
            })
        );
    }

    fn addr<T>(v: &T) -> usize {
        let v: *const T = v;

        v as usize
    }
}
