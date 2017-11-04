use mcu::{CpuSregFlags, CPU};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

pub struct CriticalSection {
    sreg: CpuSregFlags,
}

impl CriticalSection {
    pub fn new() -> CriticalSection {
        unsafe {
            let sreg = (*CPU.get()).sreg.read();
            asm!("CLI");
            asm!("" ::: "memory");

            CriticalSection { sreg }
        }
    }
}

impl Drop for CriticalSection {
    fn drop(&mut self) {
        // Restore prior status register.  This has the effect of
        // enabling interrupts again if they were enabled prior
        // to our CLI, or leaving them disabled if we were nested.
        unsafe {
            (*CPU.get()).sreg.write(self.sreg);
            asm!("" ::: "memory");
        }
    }
}

pub fn interrupt_free<F: Fn(&CriticalSection)>(f: F) {
    let cs = CriticalSection::new();
    f(&cs)
}

/// Mutex operates similarly to its namesake in std::sync::Mutex,
/// but with two important differences:
/// 1. uses a CriticalSection object as the MutexGuard
///    which means that interrupts are disabled for the duration of
///    the locked section.
/// 2. There is no concept of lock poisoning here.
pub struct Mutex<T: ?Sized> {
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

#[must_use]
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    // funny underscores due to how Deref/DerefMut currently work (they
    // disregard field privacy).
    __lock: &'a Mutex<T>,
    __cs: CriticalSection,
}

impl<'a, T: ?Sized> !Send for MutexGuard<'a, T> {}
unsafe impl<'a, T: ?Sized + Sync> Sync for MutexGuard<'a, T> {}

impl<T> Mutex<T> {
    pub fn new(t: T) -> Mutex<T> {
        Self {
            data: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<T> {
        unsafe { MutexGuard::new(&self, CriticalSection::new()) }
    }
}

impl<T: ?Sized + Default> Default for Mutex<T> {
    /// Creates a `Mutex<T>`, with the `Default` value for T.
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

impl<'mutex, T: ?Sized> MutexGuard<'mutex, T> {
    unsafe fn new(lock: &'mutex Mutex<T>, cs: CriticalSection) -> MutexGuard<'mutex, T> {
        MutexGuard {
            __lock: lock,
            __cs: cs,
        }
    }
}

impl<'mutex, T: ?Sized> Deref for MutexGuard<'mutex, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.__lock.data.get() }
    }
}

impl<'mutex, T: ?Sized> DerefMut for MutexGuard<'mutex, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.__lock.data.get() }
    }
}
