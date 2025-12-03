//! Parallel command buffers for concurrent system execution.
//!
//! When systems run in parallel, each needs its own command buffer to avoid
//! contention. This module provides a pool of buffers with atomic allocation.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::CommandQueue;
use crate::world::World;

/// A pool of command buffers for parallel system execution.
///
/// Each parallel system claims a unique buffer via atomic index, writes its
/// commands, and the pool flushes all buffers at the end of the parallel batch.
///
/// # Thread Safety
///
/// - `claim()` is thread-safe via atomic operations
/// - Each buffer is only accessed by one system at a time
/// - `flush()` must be called from a single thread after all systems complete
///
/// # Example
///
/// ```ignore
/// let buffers = ParallelCommandBuffers::new(thread_count + 1);
///
/// pool.scope(|s| {
///     for system in batch {
///         s.spawn(|_| {
///             let buffer = buffers.claim();
///             system.run_with_buffer(world_cell, buffer);
///         });
///     }
/// });
///
/// buffers.flush(world);
/// ```
pub struct ParallelCommandBuffers {
    /// Pre-allocated command buffers (one per potential thread)
    buffers: Vec<UnsafeCell<CommandQueue>>,
    /// Next buffer index to claim (atomic for thread-safety)
    next: AtomicUsize,
    /// Number of buffers (for bounds checking)
    capacity: usize,
}

// SAFETY: ParallelCommandBuffers is Send + Sync because:
// - Each buffer is only accessed by one thread at a time (via atomic claim)
// - The atomic counter ensures unique access
// - flush() is only called after all parallel work completes
unsafe impl Send for ParallelCommandBuffers {}
unsafe impl Sync for ParallelCommandBuffers {}

impl ParallelCommandBuffers {
    /// Creates a new buffer pool with the specified capacity.
    ///
    /// Typically set to `thread_pool.thread_count() + 1` to account for
    /// the main thread potentially participating in work-stealing.
    pub fn new(capacity: usize) -> Self {
        let mut buffers = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffers.push(UnsafeCell::new(CommandQueue::new()));
        }
        Self {
            buffers,
            next: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Creates a buffer pool sized for the given thread pool.
    ///
    /// Allocates `pool.thread_count() + 1` buffers.
    pub fn for_pool(pool: &crate::task::TaskPool) -> Self {
        Self::new(pool.thread_count() + 1)
    }

    /// Returns the capacity (number of buffers).
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Resets the pool for a new parallel batch.
    ///
    /// Must be called before starting a new parallel batch.
    /// Does NOT clear the individual buffers - they are cleared during flush.
    pub fn reset(&self) {
        self.next.store(0, Ordering::Release);
    }

    /// Claims a command buffer for exclusive use by the calling thread.
    ///
    /// # Panics
    ///
    /// Panics if more buffers are claimed than capacity allows.
    /// This indicates a bug - the pool should be sized for max parallelism.
    pub fn claim(&self) -> &mut CommandQueue {
        let idx = self.next.fetch_add(1, Ordering::AcqRel);
        assert!(
            idx < self.capacity,
            "ParallelCommandBuffers: claimed {} buffers but capacity is {}. \
             Increase pool size or reduce parallelism.",
            idx + 1,
            self.capacity
        );
        // SAFETY: Each index is claimed exactly once per batch due to atomic increment.
        // No two threads will access the same buffer simultaneously.
        unsafe { &mut *self.buffers[idx].get() }
    }

    /// Returns the number of buffers currently claimed.
    pub fn claimed_count(&self) -> usize {
        self.next.load(Ordering::Acquire)
    }

    /// Flushes all claimed buffers to the world and resets for next batch.
    ///
    /// # Safety Requirements
    ///
    /// Must only be called after all parallel systems have completed.
    /// This is naturally enforced by calling it after `pool.scope()` returns.
    pub fn flush(&self, world: &mut World) {
        let count = self.next.load(Ordering::Acquire);
        for i in 0..count {
            // SAFETY: We're the only thread accessing buffers now (scope completed)
            let buffer = unsafe { &mut *self.buffers[i].get() };
            buffer.apply(world);
        }
        // Reset for next batch
        self.next.store(0, Ordering::Release);
    }

    /// Clears all buffers without applying commands.
    ///
    /// Useful for error recovery or cancellation.
    pub fn clear(&self) {
        let count = self.next.load(Ordering::Acquire);
        for i in 0..count {
            let buffer = unsafe { &mut *self.buffers[i].get() };
            buffer.clear();
        }
        self.next.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicI32;

    #[test]
    fn parallel_buffers_basic() {
        let buffers = ParallelCommandBuffers::new(4);
        assert_eq!(buffers.capacity(), 4);
        assert_eq!(buffers.claimed_count(), 0);

        // Claim some buffers
        let _b1 = buffers.claim();
        let _b2 = buffers.claim();
        assert_eq!(buffers.claimed_count(), 2);

        // Reset
        buffers.reset();
        assert_eq!(buffers.claimed_count(), 0);
    }

    #[test]
    fn parallel_buffers_flush() {
        use crate::commands::Command;
        
        // Simple command that increments a counter
        struct AddCommand(Arc<AtomicI32>, i32);
        
        impl Command for AddCommand {
            fn apply_owned(self, _world: &mut World) {
                self.0.fetch_add(self.1, Ordering::SeqCst);
            }
            fn apply(self: Box<Self>, _world: &mut World) {
                self.0.fetch_add(self.1, Ordering::SeqCst);
            }
        }

        let mut world = World::new();
        let buffers = ParallelCommandBuffers::new(4);
        let counter = Arc::new(AtomicI32::new(0));

        // Claim buffers and add commands
        {
            let b1 = buffers.claim();
            b1.push(AddCommand(Arc::clone(&counter), 10));
            
            let b2 = buffers.claim();
            b2.push(AddCommand(Arc::clone(&counter), 20));
            b2.push(AddCommand(Arc::clone(&counter), 5));
        }

        assert_eq!(buffers.claimed_count(), 2);

        // Flush should apply all commands
        buffers.flush(&mut world);

        // All commands should have been applied
        assert_eq!(counter.load(Ordering::SeqCst), 35);

        // Should be reset
        assert_eq!(buffers.claimed_count(), 0);
    }

    #[test]
    fn parallel_buffers_thread_safety() {
        use std::thread;

        let buffers = Arc::new(ParallelCommandBuffers::new(8));
        let claimed = Arc::new(AtomicI32::new(0));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let buffers = Arc::clone(&buffers);
                let claimed = Arc::clone(&claimed);
                thread::spawn(move || {
                    let _buffer = buffers.claim();
                    claimed.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(buffers.claimed_count(), 4);
        assert_eq!(claimed.load(Ordering::SeqCst), 4);
    }

    #[test]
    #[should_panic(expected = "claimed")]
    fn parallel_buffers_overflow_panics() {
        let buffers = ParallelCommandBuffers::new(2);
        let _b1 = buffers.claim();
        let _b2 = buffers.claim();
        let _b3 = buffers.claim(); // Should panic
    }
}

