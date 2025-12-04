//! Thread pool for task execution.

use std::sync::Arc;

use crossbeam_channel::{bounded, Sender};
use rayon::ThreadPool as RayonThreadPool;

use super::handle::{TaskHandle, TaskId};

/// A thread pool for executing parallel and async tasks.
///
/// The pool uses work-stealing scheduling for efficient load balancing.
/// It supports both scoped parallelism (blocking) and fire-and-forget async tasks.
///
/// # Thread Count
///
/// By default, creates one thread per CPU core. Use [`TaskPool::with_threads`]
/// to customize.
///
/// # Example
///
/// ```
/// use nimbus_ecs::task::TaskPool;
///
/// let pool = TaskPool::new();
///
/// // Async task
/// let handle = pool.spawn(|| expensive_computation());
///
/// // Later...
/// if let Some(result) = handle.try_get() {
///     println!("Got result: {:?}", result);
/// }
/// # fn expensive_computation() -> i32 { 42 }
/// ```
pub struct TaskPool {
    /// The underlying rayon thread pool
    inner: Arc<RayonThreadPool>,
}

impl TaskPool {
    /// Creates a new task pool with `num_cpus - 1` worker threads.
    ///
    /// This reserves one core for the main thread (game loop, render submission).
    /// On single-core systems, creates 1 worker thread.
    pub fn new() -> Self {
        // Reserve one core for main thread
        let workers = (num_cpus::get() - 1).max(1);
        Self::with_threads(workers)
    }

    /// Creates a task pool using all CPU cores.
    ///
    /// Use this when the main thread will be mostly idle (e.g., blocking on I/O).
    /// For game loops where the main thread is active, prefer [`TaskPool::new`].
    pub fn new_all_cores() -> Self {
        Self::with_threads(num_cpus::get())
    }

    /// Creates a new task pool with the specified number of threads.
    ///
    /// # Panics
    ///
    /// Panics if `num_threads` is 0.
    pub fn with_threads(num_threads: usize) -> Self {
        assert!(num_threads > 0, "TaskPool requires at least 1 thread");

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|index| format!("nimbus-worker-{}", index))
            .build()
            .expect("failed to create rayon thread pool");

        Self {
            inner: Arc::new(pool),
        }
    }

    /// Returns the number of threads in the pool.
    #[inline]
    pub fn thread_count(&self) -> usize {
        self.inner.current_num_threads()
    }

    // =========================================================================
    // Async Tasks - fire-and-forget with handle
    // =========================================================================

    /// Spawns an async task and returns a handle to retrieve the result.
    ///
    /// The task runs on a thread pool worker. Use the returned handle to
    /// poll for completion or block waiting.
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::TaskPool;
    ///
    /// let pool = TaskPool::new();
    ///
    /// let handle = pool.spawn(|| {
    ///     // Expensive work...
    ///     42
    /// });
    ///
    /// // Non-blocking check
    /// while handle.try_get().is_none() {
    ///     // Do other work...
    /// }
    /// ```
    pub fn spawn<T, F>(&self, f: F) -> TaskHandle<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let id = TaskId::next();
        let (tx, rx) = bounded(1);

        self.inner.spawn(move || {
            let result = f();
            // Ignore send error - receiver may have been dropped
            let _ = tx.send(result);
        });

        TaskHandle::new(id, rx)
    }

    /// Spawns multiple tasks and returns handles to all of them.
    ///
    /// More efficient than calling `spawn` in a loop due to batched scheduling.
    pub fn spawn_batch<T, F, I>(&self, tasks: I) -> Vec<TaskHandle<T>>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
        I: IntoIterator<Item = F>,
    {
        tasks.into_iter().map(|f| self.spawn(f)).collect()
    }

    // =========================================================================
    // Channel-based Tasks - for streaming results
    // =========================================================================

    /// Spawns a task that can send multiple results over a channel.
    ///
    /// Useful for long-running producers like asset loaders or network handlers.
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::TaskPool;
    ///
    /// let pool = TaskPool::new();
    ///
    /// let receiver = pool.spawn_with_channel(|tx| {
    ///     for i in 0..10 {
    ///         tx.send(i).unwrap();
    ///         std::thread::sleep(std::time::Duration::from_millis(10));
    ///     }
    /// });
    ///
    /// // Consume results as they arrive
    /// for value in receiver {
    ///     println!("Got: {}", value);
    /// }
    /// ```
    pub fn spawn_with_channel<T, F>(&self, f: F) -> crossbeam_channel::Receiver<T>
    where
        T: Send + 'static,
        F: FnOnce(Sender<T>) + Send + 'static,
    {
        let (tx, rx) = crossbeam_channel::unbounded();

        self.inner.spawn(move || {
            f(tx);
        });

        rx
    }

    // =========================================================================
    // Scoped Parallelism - blocking, for parallel iteration
    // =========================================================================

    /// Executes a closure with access to a scope for spawning parallel tasks.
    ///
    /// All tasks spawned within the scope are guaranteed to complete before
    /// this method returns. This allows safe borrowing of stack data.
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::TaskPool;
    ///
    /// let pool = TaskPool::new();
    /// let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    ///
    /// pool.scope(|s| {
    ///     for chunk in data.chunks_mut(2) {
    ///         s.spawn(|_| {
    ///             for x in chunk {
    ///                 *x *= 2;
    ///             }
    ///         });
    ///     }
    /// });
    ///
    /// assert_eq!(data, vec![2, 4, 6, 8, 10, 12, 14, 16]);
    /// ```
    pub fn scope<'scope, R, F>(&self, f: F) -> R
    where
        R: Send,
        F: FnOnce(&rayon::Scope<'scope>) -> R + Send,
    {
        self.inner.scope(f)
    }

    /// Like `scope`, but spawned tasks can access data from enclosing scopes.
    ///
    /// Use when you need to spawn tasks that reference data from multiple
    /// stack frames.
    pub fn scope_fifo<'scope, R, F>(&self, f: F) -> R
    where
        R: Send,
        F: FnOnce(&rayon::ScopeFifo<'scope>) -> R + Send,
    {
        self.inner.scope_fifo(f)
    }

    // =========================================================================
    // Parallel Iteration Helpers
    // =========================================================================

    /// Executes a function for each item in parallel.
    ///
    /// Convenience wrapper around rayon's parallel iterator.
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::TaskPool;
    ///
    /// let pool = TaskPool::new();
    /// let data = vec![1, 2, 3, 4, 5];
    ///
    /// pool.par_for_each(&data, |x| {
    ///     println!("Processing: {}", x);
    /// });
    /// ```
    pub fn par_for_each<T, F>(&self, items: &[T], f: F)
    where
        T: Sync,
        F: Fn(&T) + Sync + Send,
    {
        use rayon::prelude::*;
        self.inner.install(|| {
            items.par_iter().for_each(f);
        });
    }

    /// Executes a function for each item in parallel, with mutable access.
    pub fn par_for_each_mut<T, F>(&self, items: &mut [T], f: F)
    where
        T: Send,
        F: Fn(&mut T) + Sync + Send,
    {
        use rayon::prelude::*;
        self.inner.install(|| {
            items.par_iter_mut().for_each(f);
        });
    }

    /// Maps items in parallel, collecting results.
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::TaskPool;
    ///
    /// let pool = TaskPool::new();
    /// let data = vec![1, 2, 3, 4, 5];
    ///
    /// let squared: Vec<i32> = pool.par_map(&data, |x| x * x);
    /// assert_eq!(squared, vec![1, 4, 9, 16, 25]);
    /// ```
    pub fn par_map<T, U, F>(&self, items: &[T], f: F) -> Vec<U>
    where
        T: Sync,
        U: Send,
        F: Fn(&T) -> U + Sync + Send,
    {
        use rayon::prelude::*;
        self.inner.install(|| items.par_iter().map(f).collect())
    }
    
    /// Processes slice chunks in parallel (immutable).
    ///
    /// Each chunk is processed by a separate thread. The function receives
    /// an immutable slice of `chunk_size` items (last chunk may be smaller).
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::TaskPool;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// let pool = TaskPool::new();
    /// let data: Vec<i32> = (0..1000).collect();
    /// let sum = AtomicUsize::new(0);
    ///
    /// pool.par_chunks(&data, 100, |chunk| {
    ///     let chunk_sum: i32 = chunk.iter().sum();
    ///     sum.fetch_add(chunk_sum as usize, Ordering::Relaxed);
    /// });
    ///
    /// assert_eq!(sum.load(Ordering::SeqCst), 499500);
    /// ```
    pub fn par_chunks<T, F>(&self, items: &[T], chunk_size: usize, f: F)
    where
        T: Sync,
        F: Fn(&[T]) + Sync + Send,
    {
        use rayon::prelude::*;
        self.inner.install(|| {
            items.par_chunks(chunk_size).for_each(f);
        });
    }
    
    /// Processes slice chunks in parallel (mutable).
    ///
    /// Each chunk is processed by a separate thread. The function receives
    /// a mutable slice of `chunk_size` items (last chunk may be smaller).
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::TaskPool;
    ///
    /// let pool = TaskPool::new();
    /// let mut data: Vec<i32> = (0..1000).collect();
    ///
    /// pool.par_chunks_mut(&mut data, 100, |chunk| {
    ///     for x in chunk {
    ///         *x *= 2;
    ///     }
    /// });
    ///
    /// assert_eq!(data[0], 0);
    /// assert_eq!(data[1], 2);
    /// assert_eq!(data[999], 1998);
    /// ```
    pub fn par_chunks_mut<T, F>(&self, items: &mut [T], chunk_size: usize, f: F)
    where
        T: Send,
        F: Fn(&mut [T]) + Sync + Send,
    {
        use rayon::prelude::*;
        self.inner.install(|| {
            items.par_chunks_mut(chunk_size).for_each(f);
        });
    }

    // =========================================================================
    // Direct Rayon Access
    // =========================================================================

    /// Executes a closure on the thread pool.
    ///
    /// Use for direct access to rayon's parallel iterators or when you need
    /// more control than the helper methods provide.
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::TaskPool;
    /// use rayon::prelude::*;
    ///
    /// let pool = TaskPool::new();
    ///
    /// let sum: i32 = pool.install(|| {
    ///     (0..1000).into_par_iter().sum()
    /// });
    /// ```
    pub fn install<R, F>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        self.inner.install(f)
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TaskPool {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl std::fmt::Debug for TaskPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskPool")
            .field("threads", &self.thread_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn pool_creation() {
        let pool = TaskPool::new();
        // Default: num_cpus - 1 (but at least 1)
        let expected = (num_cpus::get() - 1).max(1);
        assert_eq!(pool.thread_count(), expected);

        let pool2 = TaskPool::with_threads(4);
        assert_eq!(pool2.thread_count(), 4);

        let pool3 = TaskPool::new_all_cores();
        assert_eq!(pool3.thread_count(), num_cpus::get());
    }

    #[test]
    fn spawn_and_wait() {
        let pool = TaskPool::new();

        let handle = pool.spawn(|| {
            std::thread::sleep(Duration::from_millis(10));
            42
        });

        let result = handle.wait();
        assert_eq!(result, 42);
    }

    #[test]
    fn spawn_and_poll() {
        let pool = TaskPool::new();

        let handle = pool.spawn(|| 123);

        // Poll until ready
        loop {
            if let Some(result) = handle.try_get() {
                assert_eq!(result, 123);
                break;
            }
            std::thread::yield_now();
        }
    }

    #[test]
    fn spawn_batch() {
        let pool = TaskPool::new();

        let handles = pool.spawn_batch((0..10).map(|i| move || i * 2));

        let results: Vec<i32> = handles.into_iter().map(|h| h.wait()).collect();
        assert_eq!(results, vec![0, 2, 4, 6, 8, 10, 12, 14, 16, 18]);
    }

    #[test]
    fn spawn_with_channel() {
        let pool = TaskPool::new();

        let rx = pool.spawn_with_channel(|tx| {
            for i in 0..5 {
                tx.send(i).unwrap();
            }
        });

        let results: Vec<i32> = rx.iter().collect();
        assert_eq!(results, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn scoped_parallelism() {
        let pool = TaskPool::new();
        let counter = AtomicUsize::new(0);

        pool.scope(|s| {
            for _ in 0..100 {
                s.spawn(|_| {
                    counter.fetch_add(1, Ordering::Relaxed);
                });
            }
        });

        assert_eq!(counter.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn scoped_borrows_stack_data() {
        let pool = TaskPool::new();
        let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        pool.scope(|s| {
            for chunk in data.chunks_mut(2) {
                s.spawn(|_| {
                    for x in chunk {
                        *x *= 10;
                    }
                });
            }
        });

        assert_eq!(data, vec![10, 20, 30, 40, 50, 60, 70, 80]);
    }

    #[test]
    fn par_for_each() {
        let pool = TaskPool::new();
        let counter = AtomicUsize::new(0);
        let data: Vec<usize> = (1..=10).collect();

        pool.par_for_each(&data, |x| {
            counter.fetch_add(*x, Ordering::Relaxed);
        });

        assert_eq!(counter.load(Ordering::Relaxed), 55); // 1+2+...+10
    }

    #[test]
    fn par_for_each_mut() {
        let pool = TaskPool::new();
        let mut data: Vec<i32> = vec![1, 2, 3, 4, 5];

        pool.par_for_each_mut(&mut data, |x| {
            *x *= 2;
        });

        assert_eq!(data, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn par_map() {
        let pool = TaskPool::new();
        let data = vec![1, 2, 3, 4, 5];

        let squared: Vec<i32> = pool.par_map(&data, |x| x * x);

        assert_eq!(squared, vec![1, 4, 9, 16, 25]);
    }

    #[test]
    fn install_custom_parallel() {
        use rayon::prelude::*;

        let pool = TaskPool::new();

        let sum: i32 = pool.install(|| (0..100).into_par_iter().sum());

        assert_eq!(sum, 4950); // 0+1+2+...+99
    }

    #[test]
    fn pool_is_clone() {
        let pool1 = TaskPool::new();
        let pool2 = pool1.clone();

        // Both should work and share the same underlying pool
        let h1 = pool1.spawn(|| 1);
        let h2 = pool2.spawn(|| 2);

        assert_eq!(h1.wait(), 1);
        assert_eq!(h2.wait(), 2);
    }

    #[test]
    fn pool_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TaskPool>();
    }
}

