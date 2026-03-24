use std::array;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

/*
    Use MaybeUninit<T> to allow uninitialized memory
    Option<T> is not used because it will create extra overhead

    UnsafeCell<T> allow mutation through &self (shared reference)
    mut &self is not used because it is exclusive
    which means push() and pop() cannot happen concurrently


*/
pub struct SpscQueue<T, const N: usize> {
    buffer: [UnsafeCell<MaybeUninit<T>>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
}

/*
    Sync trait for SpscQueue

    Rust doesn't mark the struct as Sync automatically
    Sync means it is safe to be shared between threads

    T: Send is used because value move from producer to consumer
*/
unsafe impl<T: Send, const N: usize> Sync for SpscQueue<T, N> {}

/*
    array:from_fn to init array with N slots which are uninitialized,
    so no default initialization of T, no heap, no runtime growth
*/
impl<T, const N: usize> SpscQueue<T, N> {
    pub fn new() -> Self {
        assert!(N > 1, "queue capacity must be > 1");
        Self {
            buffer: array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    #[inline]
    fn next(index: usize) -> usize {
        (index + 1) % N
    }

    #[inline]
    pub fn push(&self, value: T) -> Result<(), T> {
        // Relaxed because it is SPSC, so no need to sync itself
        let tail = self.tail.load(Ordering::Relaxed);
        let next_tail = Self::next(tail);

        // Acquire because it need to get the latest value after pop()
        let head = self.head.load(Ordering::Acquire);
        if next_tail == head {
            return Err(value);
        }

        unsafe {
            (*self.buffer[tail].get()).write(value);
        }

        // Release to update it globally to other thread that Acquire it
        self.tail.store(next_tail, Ordering::Release);
        Ok(())
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        // Move value out from buffer
        let value = unsafe {
            (*self.buffer[head].get()).assume_init_read()
        };

        self.head.store(Self::next(head), Ordering::Release);
        Some(value)
    }

    #[inline]
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        if tail >= head {
            tail - head
        }
        else {
            N - head + tail
        }
    }

    #[inline]
    pub fn free_len(&self) -> usize {
        (N - 1) - self.len()
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let next_tail = Self::next(tail);
        let head = self.head.load(Ordering::Acquire);
        next_tail == head
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }
}

// Drop trait for SpscQueue since is exclusive access, no need atomic ordering
impl<T, const N: usize> Drop for SpscQueue<T, N> {
    fn drop(&mut self) {
        while self.head.load(Ordering::Relaxed) != self.tail.load(Ordering::Relaxed) {
            let mut head = *self.head.get_mut();
            let tail = *self.tail.get_mut();

            while head != tail {
                unsafe {
                    (*self.buffer[head].get()).assume_init_drop();
                }
                head = Self::next(head);
            }

            *self.head.get_mut() = head;
        }
    }
}