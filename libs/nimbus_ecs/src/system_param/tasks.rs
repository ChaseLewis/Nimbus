//! Task pool system parameter for parallel work within systems.

use std::ops::Deref;

use crate::{
    task::{TaskHandle, TaskPool},
    world::UnsafeWorldCell,
};

use super::{singleton_not_found, SystemParam, SystemParamError};

/// Provides access to the global [`TaskPool`] for spawning parallel work.
///
/// The `TaskPool` must be inserted as a singleton before systems using `Tasks` can run.
///
/// # Example
///
/// ```
/// use nimbus_ecs::{App, World, WorldInit, SystemPriority, Tasks};
/// use nimbus_ecs::task::TaskPool;
///
/// // Insert the task pool singleton
/// let mut app = App::new();
/// app.insert_singleton(TaskPool::new());
///
/// // System that uses parallel iteration
/// fn parallel_work(tasks: Tasks) {
///     let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
///     tasks.par_for_each(&data, |x| {
///         println!("Processing {} on thread", x);
///     });
/// }
///
/// app.register_system(SystemPriority::Update, parallel_work);
/// ```
///
/// # Scoped Parallelism
///
/// Use `scope()` for parallel work that borrows from the stack:
///
/// ```ignore
/// fn chunked_work(tasks: Tasks) {
///     let mut results = vec![0; 1000];
///     
///     tasks.scope(|s| {
///         for chunk in results.chunks_mut(100) {
///             s.spawn(|_| {
///                 for x in chunk {
///                     *x = compute();
///                 }
///             });
///         }
///     }); // All tasks complete before returning
/// }
/// ```
///
/// # Async Tasks
///
/// Use `spawn()` for fire-and-forget work:
///
/// ```ignore
/// fn background_work(tasks: Tasks) {
///     let handle = tasks.spawn(|| expensive_computation());
///     
///     // Poll later
///     if let Some(result) = handle.try_get() {
///         // Use result
///     }
/// }
/// 
/// fn 
/// ```
pub struct Tasks<'a> {
    pool: &'a TaskPool,
}

impl<'a> Deref for Tasks<'a> {
    type Target = TaskPool;

    fn deref(&self) -> &Self::Target {
        self.pool
    }
}

impl<'a> Tasks<'a> {
    /// Spawns an async task and returns a handle to retrieve the result.
    ///
    /// This is a convenience method that delegates to [`TaskPool::spawn`].
    #[inline]
    pub fn spawn<T, F>(&self, f: F) -> TaskHandle<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.pool.spawn(f)
    }

    /// Executes work in parallel using scoped tasks.
    ///
    /// All spawned tasks are guaranteed to complete before this method returns,
    /// allowing safe borrowing of stack data.
    ///
    /// This is a convenience method that delegates to [`TaskPool::scope`].
    #[inline]
    pub fn scope<'scope, R, F>(&'scope self, f: F) -> R
    where
        R: Send,
        F: FnOnce(&rayon::Scope<'scope>) -> R + Send,
    {
        self.pool.scope(f)
    }

    /// Executes a function for each item in parallel.
    ///
    /// This is a convenience method that delegates to [`TaskPool::par_for_each`].
    #[inline]
    pub fn par_for_each<T, F>(&self, items: &[T], f: F)
    where
        T: Sync,
        F: Fn(&T) + Sync + Send,
    {
        self.pool.par_for_each(items, f);
    }

    /// Executes a function for each item in parallel, with mutable access.
    ///
    /// This is a convenience method that delegates to [`TaskPool::par_for_each_mut`].
    #[inline]
    pub fn par_for_each_mut<T, F>(&self, items: &mut [T], f: F)
    where
        T: Send,
        F: Fn(&mut T) + Sync + Send,
    {
        self.pool.par_for_each_mut(items, f);
    }

    /// Maps items in parallel, collecting results.
    ///
    /// This is a convenience method that delegates to [`TaskPool::par_map`].
    #[inline]
    pub fn par_map<T, U, F>(&self, items: &[T], f: F) -> Vec<U>
    where
        T: Sync,
        U: Send,
        F: Fn(&T) -> U + Sync + Send,
    {
        self.pool.par_map(items, f)
    }
    
    /// Processes slice chunks in parallel (immutable).
    ///
    /// Each chunk is processed by a separate thread. The function receives
    /// an immutable slice of `chunk_size` items (last chunk may be smaller).
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn process_data(tasks: Tasks) {
    ///     let data: Vec<i32> = (0..1000).collect();
    ///     let sum = AtomicUsize::new(0);
    ///
    ///     tasks.par_chunks(&data, 100, |chunk| {
    ///         let chunk_sum: i32 = chunk.iter().sum();
    ///         sum.fetch_add(chunk_sum as usize, Ordering::Relaxed);
    ///     });
    /// }
    /// ```
    #[inline]
    pub fn par_chunks<T, F>(&self, items: &[T], chunk_size: usize, f: F)
    where
        T: Sync,
        F: Fn(&[T]) + Sync + Send,
    {
        self.pool.par_chunks(items, chunk_size, f);
    }
    
    /// Processes slice chunks in parallel (mutable).
    ///
    /// Each chunk is processed by a separate thread. The function receives
    /// a mutable slice of `chunk_size` items (last chunk may be smaller).
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn double_data(tasks: Tasks) {
    ///     let mut data: Vec<i32> = (0..1000).collect();
    ///
    ///     tasks.par_chunks_mut(&mut data, 100, |chunk| {
    ///         for x in chunk {
    ///             *x *= 2;
    ///         }
    ///     });
    /// }
    /// ```
    #[inline]
    pub fn par_chunks_mut<T, F>(&self, items: &mut [T], chunk_size: usize, f: F)
    where
        T: Send,
        F: Fn(&mut [T]) + Sync + Send,
    {
        self.pool.par_chunks_mut(items, chunk_size, f);
    }

    /// Returns the number of worker threads in the pool.
    #[inline]
    pub fn thread_count(&self) -> usize {
        self.pool.thread_count()
    }
    
    /// Returns the underlying task pool reference.
    #[inline]
    pub fn pool(&self) -> &TaskPool {
        self.pool
    }
}

impl SystemParam for Tasks<'_> {
    type State = ();
    type Item<'w, 's> = Tasks<'w>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        let pool = world
            .get_singleton::<TaskPool>()
            .ok_or_else(singleton_not_found::<TaskPool>)?;
        
        // SAFETY: The pointer is valid for the lifetime 'w
        let ptr = pool as *const TaskPool;
        Ok(Tasks { pool: unsafe { &*ptr } })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    fn tasks_system_param() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());

        fn use_tasks(tasks: Tasks) {
            assert!(tasks.thread_count() > 0);
        }

        world.run_system(use_tasks).unwrap();
    }

    #[test]
    fn tasks_parallel_iteration() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());

        use std::sync::atomic::{AtomicUsize, Ordering};

        fn sum_parallel(tasks: Tasks) {
            let data: Vec<usize> = (1..=100).collect();
            let sum = AtomicUsize::new(0);

            tasks.par_for_each(&data, |x| {
                sum.fetch_add(*x, Ordering::Relaxed);
            });

            assert_eq!(sum.load(Ordering::Relaxed), 5050); // 1+2+...+100
        }

        world.run_system(sum_parallel).unwrap();
    }

    #[test]
    fn tasks_scoped_parallelism() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());

        fn chunked_work(tasks: Tasks) {
            let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];

            tasks.scope(|s| {
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

        world.run_system(chunked_work).unwrap();
    }

    #[test]
    fn tasks_async_spawn() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());

        fn spawn_work(tasks: Tasks) {
            let handle = tasks.spawn(|| 42);
            let result = handle.wait();
            assert_eq!(result, 42);
        }

        world.run_system(spawn_work).unwrap();
    }

    #[test]
    fn tasks_par_map() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());

        fn map_parallel(tasks: Tasks) {
            let data = vec![1, 2, 3, 4, 5];
            let squared = tasks.par_map(&data, |x| x * x);
            assert_eq!(squared, vec![1, 4, 9, 16, 25]);
        }

        world.run_system(map_parallel).unwrap();
    }

    #[test]
    fn tasks_missing_singleton_error() {
        let mut world = World::new();
        // Don't insert TaskPool

        fn needs_tasks(_tasks: Tasks) {
            panic!("Should not run");
        }

        let result = world.run_system(needs_tasks);
        assert!(result.is_err());
    }
    
    #[test]
    fn tasks_par_chunks() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());
        
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        fn chunked_sum(tasks: Tasks) {
            let data: Vec<i32> = (0..1000).collect();
            let sum = AtomicUsize::new(0);
            
            tasks.par_chunks(&data, 100, |chunk| {
                let chunk_sum: i32 = chunk.iter().sum();
                sum.fetch_add(chunk_sum as usize, Ordering::Relaxed);
            });
            
            // Sum of 0..1000 = 999 * 1000 / 2 = 499500
            assert_eq!(sum.load(Ordering::SeqCst), 499500);
        }
        
        world.run_system(chunked_sum).unwrap();
    }
    
    #[test]
    fn tasks_par_chunks_mut() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());
        
        fn double_chunks(tasks: Tasks) {
            let mut data: Vec<i32> = (0..100).collect();
            
            tasks.par_chunks_mut(&mut data, 10, |chunk| {
                for x in chunk {
                    *x *= 2;
                }
            });
            
            // Verify all values were doubled
            assert_eq!(data[0], 0);
            assert_eq!(data[1], 2);
            assert_eq!(data[50], 100);
            assert_eq!(data[99], 198);
        }
        
        world.run_system(double_chunks).unwrap();
    }
}

