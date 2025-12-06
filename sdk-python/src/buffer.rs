//! Lock-free Ring Buffer for high-performance data streaming
//!
//! This module implements a SPSC (Single Producer Single Consumer)
//! ring buffer optimized for low-latency data transfer.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::UnsafeCell;

/// A lock-free ring buffer for streaming data
pub struct RingBuffer {
    buffer: Vec<UnsafeCell<Option<Vec<u8>>>>,
    capacity: usize,
    head: AtomicUsize,  // Writer position
    tail: AtomicUsize,  // Reader position
}

// Safety: RingBuffer is designed for SPSC use
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

impl RingBuffer {
    /// Create a new ring buffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(UnsafeCell::new(None));
        }
        
        Self {
            buffer,
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }
    
    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    
    /// Get the number of items in the buffer
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }
    
    /// Try to push data into the buffer
    ///
    /// Returns `Ok(())` if successful, `Err(data)` if buffer is full
    pub fn try_push(&self, data: Vec<u8>) -> Result<(), Vec<u8>> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        
        if head.wrapping_sub(tail) >= self.capacity {
            return Err(data);
        }
        
        let index = head & (self.capacity - 1);
        
        // Safety: We have exclusive access to this slot
        unsafe {
            *self.buffer[index].get() = Some(data);
        }
        
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }
    
    /// Try to pop data from the buffer
    ///
    /// Returns `Some(data)` if available, `None` if buffer is empty
    pub fn try_pop(&self) -> Option<Vec<u8>> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        
        if tail == head {
            return None;
        }
        
        let index = tail & (self.capacity - 1);
        
        // Safety: We have exclusive access to this slot
        let data = unsafe {
            (*self.buffer[index].get()).take()
        };
        
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_operations() {
        let buffer = RingBuffer::new(4);
        
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
        
        buffer.try_push(vec![1, 2, 3]).unwrap();
        assert_eq!(buffer.len(), 1);
        
        let data = buffer.try_pop().unwrap();
        assert_eq!(data, vec![1, 2, 3]);
        assert!(buffer.is_empty());
    }
    
    #[test]
    fn test_full_buffer() {
        let buffer = RingBuffer::new(2);
        
        buffer.try_push(vec![1]).unwrap();
        buffer.try_push(vec![2]).unwrap();
        
        // Buffer should be full now (capacity is rounded to 2)
        assert!(buffer.is_full());
        
        let result = buffer.try_push(vec![3]);
        assert!(result.is_err());
    }
}
