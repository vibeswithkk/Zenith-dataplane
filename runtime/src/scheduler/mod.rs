/// Task Scheduler for Runtime
/// Manages concurrent plugin execution and resource allocation
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Schedulable task
pub struct Task {
    pub id: u64,
    pub priority: Priority,
    pub payload: Vec<u8>,
}

/// Simple priority-based task scheduler
pub struct Scheduler {
    queues: Arc<Mutex<[VecDeque<Task>; 4]>>, // One queue per priority level
    concurrency_limit: Arc<Semaphore>,
    next_task_id: std::sync::atomic::AtomicU64,
}

impl Scheduler {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            queues: Arc::new(Mutex::new([
                VecDeque::new(), // Low
                VecDeque::new(), // Normal
                VecDeque::new(), // High
                VecDeque::new(), // Critical
            ])),
            concurrency_limit: Arc::new(Semaphore::new(max_concurrent)),
            next_task_id: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Submit a task for execution
    pub fn submit(&self, priority: Priority, payload: Vec<u8>) -> u64 {
        let task_id = self.next_task_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let task = Task {
            id: task_id,
            priority,
            payload,
        };

        let mut queues = self.queues.lock().unwrap();
        queues[priority as usize].push_back(task);
        
        task_id
    }

    /// Get next task (highest priority first)
    pub async fn next_task(&self) -> Option<Task> {
        let _permit = self.concurrency_limit.acquire().await.ok()?;
        
        let mut queues = self.queues.lock().unwrap();
        
        // Check queues from highest to lowest priority
        for i in (0..4).rev() {
            if let Some(task) = queues[i].pop_front() {
                return Some(task);
            }
        }
        
        None
    }

    /// Get pending task count
    pub fn pending_count(&self) -> usize {
        let queues = self.queues.lock().unwrap();
        queues.iter().map(|q| q.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_scheduling() {
        let scheduler = Scheduler::new(10);
        
        scheduler.submit(Priority::Low, vec![1]);
        scheduler.submit(Priority::Critical, vec![2]);
        scheduler.submit(Priority::Normal, vec![3]);
        
        assert_eq!(scheduler.pending_count(), 3);
    }
}
