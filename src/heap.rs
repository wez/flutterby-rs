//! A slimmed down version of some heap functions.
//! It's generally frowned upon to use dynamic memory in
//! the context of AVR/embedded systems, but limiting ourselves to
//! static allocations forces us to use a lot of unsafe
//! rust code.  This flavor of allocator allows us to adopt
//! some of the higher level rust stuff that makes rust
//! so appealing.
use core::ptr::{self, Unique};
use core::mem;
use core::ops::{Deref, DerefMut};
use core::hash::{self, Hash};
use core::cmp::Ordering;
use core::fmt;
use core::ops::CoerceUnsized;
use core::marker::Unsize;

extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

pub struct Box<T: ?Sized>(Unique<T>);

impl<T> Box<T> {
    /// Attempt to allocate heap storage for x and move the value
    /// into it.
    pub fn try_new(x: T) -> Result<Self, ()> {
        unsafe {
            let ptr = malloc(mem::size_of_val(&x)) as *mut T;
            if ptr.is_null() {
                Err(())
            } else {
                ptr::write_volatile(ptr, x);
                Ok(Self {
                    0: Unique::new_unchecked(ptr),
                })
            }
        }
    }

    /// Allocate heap storage for x and move the value into it.
    /// ## Panics
    /// Will panic if there is insufficient heap available.
    /// Consider using try_new() if possible.
    #[inline(always)]
    pub fn new(x: T) -> Self {
        Box::try_new(x).expect("malloc failed")
    }
}

impl<T: Clone> Box<T> {
    /// Try to clone a new boxed version
    pub fn try_clone(&self) -> Result<Box<T>, ()> {
        unsafe { Self::try_new((*self.0.as_ref()).clone()) }
    }
}

impl<T: ?Sized> Box<T> {
    #[inline]
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        mem::transmute(raw)
    }

    #[inline]
    pub fn into_raw(b: Box<T>) -> *mut T {
        unsafe { mem::transmute(b) }
    }
}

impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            free(self.0.as_ptr() as *mut u8);
        }
    }
}

impl<T: Default> Default for Box<T> {
    /// Creates a `Box<T>`, with the `Default` value for T.
    fn default() -> Box<T> {
        Box::new(Default::default())
    }
}

impl<T: Clone> Clone for Box<T> {
    /// Returns a new box with a `clone()` of this box's contents.
    ///
    /// # Examples
    ///
    /// ```
    /// let x = Box::new(5);
    /// let y = x.clone();
    /// ```
    #[inline]
    fn clone(&self) -> Box<T> {
        Box::new({ (**self).clone() })
    }

    #[inline]
    fn clone_from(&mut self, source: &Box<T>) {
        (**self).clone_from(&(**source));
    }
}

impl<T: ?Sized + PartialEq> PartialEq for Box<T> {
    #[inline]
    fn eq(&self, other: &Box<T>) -> bool {
        PartialEq::eq(&**self, &**other)
    }
    #[inline]
    fn ne(&self, other: &Box<T>) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}

impl<T: ?Sized + PartialOrd> PartialOrd for Box<T> {
    #[inline]
    fn partial_cmp(&self, other: &Box<T>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
    #[inline]
    fn lt(&self, other: &Box<T>) -> bool {
        PartialOrd::lt(&**self, &**other)
    }
    #[inline]
    fn le(&self, other: &Box<T>) -> bool {
        PartialOrd::le(&**self, &**other)
    }
    #[inline]
    fn ge(&self, other: &Box<T>) -> bool {
        PartialOrd::ge(&**self, &**other)
    }
    #[inline]
    fn gt(&self, other: &Box<T>) -> bool {
        PartialOrd::gt(&**self, &**other)
    }
}

impl<T: ?Sized + Ord> Ord for Box<T> {
    #[inline]
    fn cmp(&self, other: &Box<T>) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

impl<T: ?Sized + Eq> Eq for Box<T> {}

impl<T: ?Sized + Hash> Hash for Box<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T> From<T> for Box<T> {
    fn from(t: T) -> Self {
        Box::new(t)
    }
}

impl<T: fmt::Debug + ?Sized> fmt::Debug for Box<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

impl<I: Iterator + ?Sized> Iterator for Box<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
    fn nth(&mut self, n: usize) -> Option<I::Item> {
        (**self).nth(n)
    }
}

impl<T: ?Sized> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> {}
