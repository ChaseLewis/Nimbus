//! Scoped task execution for parallel work with borrowing.
//!
//! Scopes provide a way to spawn parallel tasks that can borrow from their
//! environment. All tasks spawned within a scope are guaranteed to complete
//! before the scope exits, making it safe to borrow stack data.

/// Re-export rayon's Scope for direct use.
///
/// A scope allows spawning parallel tasks that can borrow data from the stack.
/// All spawned tasks must complete before the scope exits.
///
/// # Example
///
/// ```
/// use nimbus_ecs::task::TaskPool;
///
/// let pool = TaskPool::new();
/// let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];
///
/// pool.scope(|scope| {
///     // Process chunks in parallel - each chunk is disjoint
///     for chunk in data.chunks_mut(2) {
///         scope.spawn(move |_| {
///             for x in chunk {
///                 *x *= 2;
///             }
///         });
///     }
/// });
///
/// assert_eq!(data, vec![2, 4, 6, 8, 10, 12, 14, 16]);
/// ```
pub type Scope<'scope> = rayon::Scope<'scope>;

/// FIFO variant of Scope that processes tasks in spawn order.
///
/// Use when task ordering matters for cache locality or debugging.
pub type ScopeFifo<'scope> = rayon::ScopeFifo<'scope>;

/// Extension trait for convenient parallel chunk processing.
pub trait ParallelChunks<T> {
    /// Process chunks in parallel using the given task pool.
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::task::{TaskPool, ParallelChunks};
    ///
    /// let pool = TaskPool::new();
    /// let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    ///
    /// data.par_chunks_mut(&pool, 2, |chunk| {
    ///     for x in chunk {
    ///         *x *= 10;
    ///     }
    /// });
    ///
    /// assert_eq!(data, vec![10, 20, 30, 40, 50, 60, 70, 80]);
    /// ```
    fn par_chunks_mut<F>(&mut self, pool: &super::TaskPool, chunk_size: usize, f: F)
    where
        F: Fn(&mut [T]) + Sync + Send;

    /// Process chunks in parallel, read-only.
    fn par_chunks<F>(&self, pool: &super::TaskPool, chunk_size: usize, f: F)
    where
        F: Fn(&[T]) + Sync + Send;
}

impl<T: Send + Sync> ParallelChunks<T> for [T] {
    fn par_chunks_mut<F>(&mut self, pool: &super::TaskPool, chunk_size: usize, f: F)
    where
        F: Fn(&mut [T]) + Sync + Send,
    {
        pool.scope(|scope| {
            for chunk in self.chunks_mut(chunk_size) {
                scope.spawn(|_| f(chunk));
            }
        });
    }

    fn par_chunks<F>(&self, pool: &super::TaskPool, chunk_size: usize, f: F)
    where
        F: Fn(&[T]) + Sync + Send,
    {
        pool.scope(|scope| {
            for chunk in self.chunks(chunk_size) {
                scope.spawn(|_| f(chunk));
            }
        });
    }
}

impl<T: Send + Sync> ParallelChunks<T> for Vec<T> {
    fn par_chunks_mut<F>(&mut self, pool: &super::TaskPool, chunk_size: usize, f: F)
    where
        F: Fn(&mut [T]) + Sync + Send,
    {
        self.as_mut_slice().par_chunks_mut(pool, chunk_size, f);
    }

    fn par_chunks<F>(&self, pool: &super::TaskPool, chunk_size: usize, f: F)
    where
        F: Fn(&[T]) + Sync + Send,
    {
        self.as_slice().par_chunks(pool, chunk_size, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskPool;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn par_chunks_mut_processes_all() {
        let pool = TaskPool::new();
        let mut data: Vec<i32> = (0..100).collect();

        data.par_chunks_mut(&pool, 10, |chunk| {
            for x in chunk {
                *x += 1;
            }
        });

        let expected: Vec<i32> = (1..101).collect();
        assert_eq!(data, expected);
    }

    #[test]
    fn par_chunks_read_only() {
        let pool = TaskPool::new();
        let data: Vec<i32> = (1..=10).collect();
        let sum = AtomicUsize::new(0);

        data.par_chunks(&pool, 2, |chunk| {
            let chunk_sum: i32 = chunk.iter().sum();
            sum.fetch_add(chunk_sum as usize, Ordering::Relaxed);
        });

        assert_eq!(sum.load(Ordering::Relaxed), 55);
    }

    #[test]
    fn par_chunks_uneven_size() {
        let pool = TaskPool::new();
        let mut data = vec![1, 2, 3, 4, 5]; // 5 elements, chunk size 2

        data.par_chunks_mut(&pool, 2, |chunk| {
            for x in chunk {
                *x *= 10;
            }
        });

        assert_eq!(data, vec![10, 20, 30, 40, 50]);
    }
}

