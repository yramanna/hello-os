//! Interrupt-safe Mutex implementation
//! 
//! This mutex disables interrupts while holding the lock to prevent deadlocks
//! with interrupt handlers that might try to acquire the same lock.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A mutual exclusion primitive that disables interrupts while held
pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    /// Acquires the mutex, blocking until it becomes available
    /// Disables interrupts before acquiring the lock
    pub fn lock(&self) -> MutexGuard<T> {
        // Disable interrupts
        let interrupts_enabled = are_interrupts_enabled();
        disable_interrupts();

        // Spin until we acquire the lock
        while self.locked.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Acquire
        ).is_err() {
            // Hint to CPU that we're spinning
            core::hint::spin_loop();
        }

        MutexGuard {
            mutex: self,
            interrupts_were_enabled: interrupts_enabled,
        }
    }

    /// Tries to acquire the mutex without blocking
    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        let interrupts_enabled = are_interrupts_enabled();
        disable_interrupts();

        if self.locked.compare_exchange(
            false,
            true,
            Ordering::Acquire,
            Ordering::Acquire
        ).is_ok() {
            Some(MutexGuard {
                mutex: self,
                interrupts_were_enabled: interrupts_enabled,
            })
        } else {
            // Re-enable interrupts if we didn't acquire the lock
            if interrupts_enabled {
                enable_interrupts();
            }
            None
        }
    }
}

/// RAII guard for the mutex
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    interrupts_were_enabled: bool,
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        // Release the lock
        self.mutex.locked.store(false, Ordering::Release);

        // Re-enable interrupts if they were enabled before
        if self.interrupts_were_enabled {
            enable_interrupts();
        }
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

/// Check if interrupts are enabled
fn are_interrupts_enabled() -> bool {
    let rflags: u64;
    unsafe {
        core::arch::asm!("pushfq; pop {}", out(reg) rflags, options(nomem, preserves_flags));
    }
    (rflags & (1 << 9)) != 0
}

/// Disable interrupts
fn disable_interrupts() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

/// Enable interrupts
fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}