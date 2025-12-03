//! Task handles for tracking and retrieving async task results.

use crossbeam_channel::Receiver;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for a task.
///
/// Task IDs are globally unique within a process and monotonically increasing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    /// Creates a new unique task ID.
    pub(crate) fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns the raw ID value.
    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Handle to a pending async task.
///
/// Use this to poll for completion or block waiting for the result.
/// The handle can only retrieve the result once - subsequent calls
/// to `try_get()` or `wait()` after success will return `None` or panic.
///
/// # Example
///
/// ```
/// use nimbus_ecs::task::TaskPool;
///
/// let pool = TaskPool::new();
/// let handle = pool.spawn(|| 42);
///
/// // Non-blocking poll
/// loop {
///     if let Some(result) = handle.try_get() {
///         assert_eq!(result, 42);
///         break;
///     }
///     // Do other work...
/// }
/// ```
pub struct TaskHandle<T> {
    id: TaskId,
    receiver: Receiver<T>,
}

impl<T> TaskHandle<T> {
    /// Creates a new task handle.
    pub(crate) fn new(id: TaskId, receiver: Receiver<T>) -> Self {
        Self { id, receiver }
    }

    /// Returns the task's unique identifier.
    #[inline]
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Non-blocking attempt to get the result.
    ///
    /// Returns `Some(result)` if the task has completed, `None` if still pending.
    /// After returning `Some`, subsequent calls will return `None`.
    #[inline]
    pub fn try_get(&self) -> Option<T> {
        self.receiver.try_recv().ok()
    }

    /// Check if the task has completed without consuming the result.
    ///
    /// Note: This only checks if a result is available. The task may complete
    /// between calling `is_ready()` and `try_get()`.
    #[inline]
    pub fn is_ready(&self) -> bool {
        !self.receiver.is_empty()
    }

    /// Block the current thread until the task completes.
    ///
    /// # Panics
    ///
    /// Panics if the task was cancelled or the result was already retrieved.
    ///
    /// # Warning
    ///
    /// Use sparingly - blocking defeats the purpose of async tasks.
    /// Prefer `try_get()` in game loops, or use at sync barriers only.
    pub fn wait(self) -> T {
        self.receiver
            .recv()
            .expect("task channel closed - result already retrieved or task cancelled")
    }

    /// Block with a timeout.
    ///
    /// Returns `Ok(result)` if completed within the timeout, `Err(self)` if timed out.
    pub fn wait_timeout(self, timeout: std::time::Duration) -> Result<T, Self> {
        match self.receiver.recv_timeout(timeout) {
            Ok(result) => Ok(result),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => Err(self),
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                panic!("task channel closed - result already retrieved or task cancelled")
            }
        }
    }
}

impl<T> std::fmt::Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskHandle")
            .field("id", &self.id)
            .field("ready", &self.is_ready())
            .finish()
    }
}

/// A completed task handle that can be created synchronously.
///
/// Useful for APIs that expect a `TaskHandle` but you have an immediate result.
impl<T: Send + 'static> TaskHandle<T> {
    /// Creates a handle that is immediately ready with the given value.
    pub fn completed(value: T) -> Self {
        let (tx, rx) = crossbeam_channel::bounded(1);
        tx.send(value).expect("just created channel");
        Self::new(TaskId::next(), rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_id_unique() {
        let id1 = TaskId::next();
        let id2 = TaskId::next();
        let id3 = TaskId::next();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);

        // Monotonically increasing
        assert!(id1.as_u64() < id2.as_u64());
        assert!(id2.as_u64() < id3.as_u64());
    }

    #[test]
    fn task_handle_completed() {
        let handle = TaskHandle::completed(42);
        assert!(handle.is_ready());
        assert_eq!(handle.try_get(), Some(42));
    }

    #[test]
    fn task_handle_try_get_consumes() {
        let handle = TaskHandle::completed("hello");
        assert_eq!(handle.try_get(), Some("hello"));
        assert_eq!(handle.try_get(), None); // Already consumed
    }

    #[test]
    fn task_handle_wait_timeout() {
        let handle = TaskHandle::completed(123);

        // Should complete immediately
        let result = handle.wait_timeout(std::time::Duration::from_millis(100));
        assert_eq!(result.ok(), Some(123));
    }
}

