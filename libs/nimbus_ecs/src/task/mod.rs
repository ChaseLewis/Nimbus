//! Async task execution and thread pool management.
//!
//! This module provides a thread pool for parallel and async task execution.
//! It supports three patterns of parallelism:
//!
//! 1. **Scoped tasks** - Parallel work that must complete before returning.
//!    Perfect for parallel query iteration within a system.
//!
//! 2. **Async tasks** - Fire-and-forget work with a handle to poll for results.
//!    Use for pathfinding, AI planning, procedural generation.
//!
//! 3. **Channels** - Long-running producers that stream results over time.
//!    Use for asset loading, network I/O.
//!
//! # Example
//!
//! ```
//! use nimbus_ecs::task::{TaskPool, TaskHandle};
//!
//! let pool = TaskPool::new();
//!
//! // Scoped parallelism (blocks until complete)
//! let mut data = vec![1, 2, 3, 4];
//! pool.scope(|s| {
//!     for chunk in data.chunks_mut(2) {
//!         s.spawn(move |_| {
//!             for x in chunk { *x *= 2; }
//!         });
//!     }
//! });
//! assert_eq!(data, vec![2, 4, 6, 8]);
//!
//! // Async task (non-blocking)
//! let handle: TaskHandle<i32> = pool.spawn(|| {
//!     std::thread::sleep(std::time::Duration::from_millis(10));
//!     42
//! });
//!
//! // Poll for result (non-blocking)
//! assert!(handle.try_get().is_none()); // Not ready yet
//!
//! // Or block until ready
//! let result = handle.wait();
//! assert_eq!(result, 42);
//! ```

mod handle;
mod pool;
mod scope;

pub use handle::{TaskHandle, TaskId};
pub use pool::TaskPool;
pub use scope::{ParallelChunks, Scope, ScopeFifo};

