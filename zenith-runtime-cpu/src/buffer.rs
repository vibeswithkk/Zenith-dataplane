//! Lock-free Ring Buffer Implementations
//!
//! This module provides high-performance, lock-free ring buffers for
//! producer/consumer patterns in low-latency applications.
//!
//! ## Implementations
//!
//! - `SpscRingBuffer`: Single Producer Single Consumer - highest performance
//! - `MpmcRingBuffer`: Multiple Producer Multiple Consumer - thread-safe
//!
//! ## Performance Characteristics
//!
//! - Zero memory allocation during operation
//! - Cache-line aligned to prevent false sharing
//! - Memory ordering optimized for x86_64 and ARM
//! - Batch operations for improved throughput

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Cache line size for padding to prevent false sharing
const CACHE_LINE_SIZE: usize = 64;

/// Trait for ring buffer operations
pub trait RingBuffer<T> {
    /// Try to push an item into the buffer
    ///
    /// Returns `Ok(())` if successful, `Err(item)` if buffer is full
    fn try_push(&self, item: T) -> Result<(), T>;
    
    /// Try to pop an item from the buffer
    ///
    /// Returns `Some(item)` if successful, `None` if buffer is empty
    fn try_pop(&self) -> Option<T>;
    
    /// Returns the current number of items in the buffer
    fn len(&self) -> usize;
    
    /// Returns true if the buffer is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Returns true if the buffer is full
    fn is_full(&self) -> bool;
    
    /// Returns the capacity of the buffer
    fn capacity(&self) -> usize;
}

/// Cache-line padded atomic counter
#[repr(align(64))]
struct PaddedAtomicUsize {
    value: AtomicUsize,
    _padding: [u8; CACHE_LINE_SIZE - std::mem::size_of::<AtomicUsize>()],
}

impl PaddedAtomicUsize {
    fn new(value: usize) -> Self {
        Self {
            value: AtomicUsize::new(value),
            _padding: [0; CACHE_LINE_SIZE - std::mem::size_of::<AtomicUsize>()],
        }
    }
    
    fn load(&self, ordering: Ordering) -> usize {
        self.value.load(ordering)
    }
    
    fn store(&self, value: usize, ordering: Ordering) {
        self.value.store(value, ordering)
    }
}

/// Single Producer Single Consumer Ring Buffer
///
/// Optimized for the case where exactly one thread pushes and one thread pops.
/// This is the highest-performance option when applicable.
///
/// # Example
///
/// ```
/// use zenith_runtime_cpu::buffer::{SpscRingBuffer, RingBuffer};
///
/// let buffer = SpscRingBuffer::<u64>::new(1024);
///
/// // Producer thread
/// buffer.try_push(42).unwrap();
///
/// // Consumer thread
/// let value = buffer.try_pop().unwrap();
/// assert_eq!(value, 42);
/// ```
pub struct SpscRingBuffer<T> {
    /// Buffer storage
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,
    /// Capacity (power of 2)
    capacity: usize,
    /// Mask for fast modulo (capacity - 1)
    mask: usize,
    /// Producer position (cache-line aligned)
    head: PaddedAtomicUsize,
    /// Consumer position (cache-line aligned)
    tail: PaddedAtomicUsize,
}

// Safety: SpscRingBuffer is designed for single-producer single-consumer
// The head is only written by producer, tail only by consumer
unsafe impl<T: Send> Send for SpscRingBuffer<T> {}
unsafe impl<T: Send> Sync for SpscRingBuffer<T> {}

impl<T> SpscRingBuffer<T> {
    /// Create a new SPSC ring buffer with the specified capacity
    ///
    /// Capacity will be rounded up to the next power of 2.
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than 0");
        
        // Round up to power of 2
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;
        
        // Allocate buffer with uninitialized memory
        let buffer: Vec<UnsafeCell<MaybeUninit<T>>> = (0..capacity)
            .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
            .collect();
        
        Self {
            buffer: buffer.into_boxed_slice(),
            capacity,
            mask,
            head: PaddedAtomicUsize::new(0),
            tail: PaddedAtomicUsize::new(0),
        }
    }
    
    /// Push multiple items in a batch
    ///
    /// Returns the number of items successfully pushed
    pub fn push_batch(&self, items: &mut Vec<T>) -> usize {
        let mut pushed = 0;
        while let Some(item) = items.pop() {
            if self.try_push(item).is_err() {
                break;
            }
            pushed += 1;
        }
        pushed
    }
    
    /// Pop multiple items in a batch
    ///
    /// Returns items up to `max_count`
    pub fn pop_batch(&self, max_count: usize) -> Vec<T> {
        let mut items = Vec::with_capacity(max_count);
        for _ in 0..max_count {
            match self.try_pop() {
                Some(item) => items.push(item),
                None => break,
            }
        }
        items
    }
}

impl<T> RingBuffer<T> for SpscRingBuffer<T> {
    fn try_push(&self, item: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        
        // Check if full
        if head.wrapping_sub(tail) >= self.capacity {
            return Err(item);
        }
        
        let index = head & self.mask;
        
        // Safety: We have exclusive write access to this slot
        unsafe {
            (*self.buffer[index].get()).write(item);
        }
        
        // Make the item visible to consumer
        self.head.store(head.wrapping_add(1), Ordering::Release);
        
        Ok(())
    }
    
    fn try_pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        
        // Check if empty
        if tail == head {
            return None;
        }
        
        let index = tail & self.mask;
        
        // Safety: We have exclusive read access to this slot
        let item = unsafe {
            (*self.buffer[index].get()).assume_init_read()
        };
        
        // Mark slot as consumed
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        
        Some(item)
    }
    
    fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }
    
    fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }
    
    fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T> Drop for SpscRingBuffer<T> {
    fn drop(&mut self) {
        // Drop any remaining items
        while self.try_pop().is_some() {}
    }
}

/// Multiple Producer Multiple Consumer Ring Buffer
///
/// Thread-safe ring buffer supporting multiple producers and consumers.
/// Uses crossbeam's high-performance bounded queue internally.
pub struct MpmcRingBuffer<T> {
    inner: crossbeam_queue::ArrayQueue<T>,
}

impl<T> MpmcRingBuffer<T> {
    /// Create a new MPMC ring buffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: crossbeam_queue::ArrayQueue::new(capacity),
        }
    }
}

impl<T> RingBuffer<T> for MpmcRingBuffer<T> {
    fn try_push(&self, item: T) -> Result<(), T> {
        self.inner.push(item)
    }
    
    fn try_pop(&self) -> Option<T> {
        self.inner.pop()
    }
    
    fn len(&self) -> usize {
        self.inner.len()
    }
    
    fn is_full(&self) -> bool {
        self.inner.is_full()
    }
    
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    
    #[test]
    fn test_spsc_basic() {
        let buffer = SpscRingBuffer::<u64>::new(4);
        
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.capacity(), 4);
        
        buffer.try_push(1).unwrap();
        buffer.try_push(2).unwrap();
        buffer.try_push(3).unwrap();
        
        assert_eq!(buffer.len(), 3);
        
        assert_eq!(buffer.try_pop(), Some(1));
        assert_eq!(buffer.try_pop(), Some(2));
        assert_eq!(buffer.try_pop(), Some(3));
        assert_eq!(buffer.try_pop(), None);
    }
    
    #[test]
    fn test_spsc_full() {
        let buffer = SpscRingBuffer::<u64>::new(2);
        
        buffer.try_push(1).unwrap();
        buffer.try_push(2).unwrap();
        
        assert!(buffer.is_full());
        
        let result = buffer.try_push(3);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), 3);
    }
    
    #[test]
    fn test_spsc_concurrent() {
        let buffer = Arc::new(SpscRingBuffer::<u64>::new(1024));
        let buffer_producer = Arc::clone(&buffer);
        let buffer_consumer = Arc::clone(&buffer);
        
        const COUNT: u64 = 10_000;
        
        let producer = thread::spawn(move || {
            for i in 0..COUNT {
                while buffer_producer.try_push(i).is_err() {
                    std::hint::spin_loop();
                }
            }
        });
        
        let consumer = thread::spawn(move || {
            let mut received = 0u64;
            let mut sum = 0u64;
            
            while received < COUNT {
                if let Some(value) = buffer_consumer.try_pop() {
                    sum += value;
                    received += 1;
                } else {
                    std::hint::spin_loop();
                }
            }
            
            sum
        });
        
        producer.join().unwrap();
        let sum = consumer.join().unwrap();
        
        // Sum of 0..COUNT = COUNT * (COUNT - 1) / 2
        let expected = COUNT * (COUNT - 1) / 2;
        assert_eq!(sum, expected);
    }
    
    #[test]
    fn test_mpmc_basic() {
        let buffer = MpmcRingBuffer::<u64>::new(4);
        
        buffer.try_push(1).unwrap();
        buffer.try_push(2).unwrap();
        
        assert_eq!(buffer.try_pop(), Some(1));
        assert_eq!(buffer.try_pop(), Some(2));
        assert_eq!(buffer.try_pop(), None);
    }
}
