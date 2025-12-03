//! Simple single-buffered event system.
//!
//! Events are written and read from the same buffer. The buffer is cleared
//! after StartOfFrame, so:
//! - StartOfFrame event handlers see last frame's events
//! - Later priorities can see same-frame events (written earlier in frame)
//!
//! # Example
//!
//! ```
//! use nimbus_ecs::{EventWriter, EventReader};
//!
//! #[derive(Clone)]
//! struct DamageEvent { amount: i32 }
//!
//! // System that sends events
//! fn combat_system(damage_events: EventWriter<DamageEvent>) {
//!     damage_events.send(DamageEvent { amount: 50 });
//! }
//!
//! // System that reads events
//! fn damage_system(damage_events: EventReader<DamageEvent>) {
//!     for event in damage_events.iter() {
//!         println!("Damage: {}", event.amount);
//!     }
//! }
//! ```

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::marker::PhantomData;

use crate::system_param::{SystemParam, SystemParamError};
use crate::util::TypeHashMap;
use crate::world::UnsafeWorldCell;

// ============================================================================
// Events<T> - Single-buffered event queue
// ============================================================================

/// Single-buffered event queue for a specific event type.
pub struct Events<T> {
    /// The event buffer.
    buffer: Vec<T>,
}

impl<T> Default for Events<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Events<T> {
    /// Creates a new empty event queue.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }

    /// Sends an event.
    #[inline]
    pub fn send(&mut self, event: T) {
        self.buffer.push(event);
    }

    /// Returns an iterator over all events.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }

    /// Returns the number of events.
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if there are no events.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clears all events.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

// ============================================================================
// EventQueue trait - Type-erased interface
// ============================================================================

/// Type-erased event queue for storage in EventStorage.
pub(crate) trait EventQueue: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear(&mut self);
}

impl<T: Send + Sync + 'static> EventQueue for Events<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clear(&mut self) {
        Events::clear(self);
    }
}

// ============================================================================
// EventStorage - Holds all event queues
// ============================================================================

/// Storage for all event queues, indexed by event TypeId.
pub struct EventStorage {
    queues: TypeHashMap<RefCell<Box<dyn EventQueue>>>,
}

impl Default for EventStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStorage {
    /// Creates a new empty event storage.
    pub fn new() -> Self {
        Self {
            queues: TypeHashMap::default(),
        }
    }

    /// Ensures an event queue exists for type T, returns mutable reference.
    pub(crate) fn ensure_queue<T: Send + Sync + 'static>(&mut self) -> &RefCell<Box<dyn EventQueue>> {
        let type_id = TypeId::of::<T>();
        self.queues
            .entry(type_id)
            .or_insert_with(|| RefCell::new(Box::new(Events::<T>::new())))
    }

    /// Gets a reference to the event queue for type T.
    #[allow(dead_code)]
    pub(crate) fn queue<T: Send + Sync + 'static>(&self) -> Option<&RefCell<Box<dyn EventQueue>>> {
        let type_id = TypeId::of::<T>();
        self.queues.get(&type_id)
    }

    /// Swaps all event buffers. Called at start of frame.
    /// Clears all event queues. Called after StartOfFrame.
    pub fn clear_all(&mut self) {
        for queue in self.queues.values() {
            queue.borrow_mut().clear();
        }
    }
}

// ============================================================================
// EventWriter<T> - SystemParam for sending events
// ============================================================================

/// System parameter for sending events of type T.
///
/// Events are written to a back buffer and will be readable next frame.
///
/// # Example
/// ```
/// use nimbus_ecs::EventWriter;
///
/// #[derive(Clone)]
/// struct ScoreEvent { points: i32 }
///
/// fn score_system(events: EventWriter<ScoreEvent>) {
///     events.send(ScoreEvent { points: 100 });
/// }
/// ```
pub struct EventWriter<'w, T: Send + Sync + 'static> {
    events: &'w RefCell<Box<dyn EventQueue>>,
    _marker: PhantomData<T>,
}

impl<'w, T: Send + Sync + 'static> EventWriter<'w, T> {
    /// Sends an event.
    #[inline]
    pub fn send(&self, event: T) {
        let mut queue = self.events.borrow_mut();
        let events = queue.as_any_mut().downcast_mut::<Events<T>>()
            .expect("EventWriter type mismatch");
        events.send(event);
    }

    /// Sends multiple events.
    pub fn send_batch(&self, events: impl IntoIterator<Item = T>) {
        let mut queue = self.events.borrow_mut();
        let event_queue = queue.as_any_mut().downcast_mut::<Events<T>>()
            .expect("EventWriter type mismatch");
        for event in events {
            event_queue.send(event);
        }
    }
}

/// State for EventWriter system parameter.
pub struct EventWriterState<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for EventWriterState<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<T: Send + Sync + 'static> SystemParam for EventWriter<'_, T> {
    type State = EventWriterState<T>;
    type Item<'world, 'state> = EventWriter<'world, T>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        let world_mut = world.as_mut();
        let queue = world_mut.event_storage.ensure_queue::<T>();
        
        // Safety: We're getting a reference that will live for 'w
        let queue_ref = unsafe {
            &*(queue as *const RefCell<Box<dyn EventQueue>>)
        };
        
        Ok(EventWriter {
            events: queue_ref,
            _marker: PhantomData,
        })
    }
}

// ============================================================================
// EventReader<T> - SystemParam for reading events
// ============================================================================

/// System parameter for reading events of type T.
///
/// # Example
/// ```
/// use nimbus_ecs::EventReader;
///
/// #[derive(Clone)]
/// struct DamageEvent { amount: i32 }
///
/// fn handle_damage(events: EventReader<DamageEvent>) {
///     for event in events.iter() {
///         println!("Damage: {}", event.amount);
///     }
/// }
/// ```
pub struct EventReader<'w, T: Send + Sync + 'static> {
    events: &'w RefCell<Box<dyn EventQueue>>,
    _marker: PhantomData<T>,
}

impl<'w, T: Send + Sync + 'static> EventReader<'w, T> {
    /// Returns an iterator over all readable events.
    /// 
    /// Holds a borrow guard for safety - will panic if EventWriter
    /// tries to write to the same event type concurrently.
    #[inline]
    pub fn iter(&self) -> EventIter<'_, T> {
        EventIter::new(self.events)
    }

    /// Returns the number of readable events.
    #[inline]
    pub fn len(&self) -> usize {
        let queue = self.events.borrow();
        let events = queue.as_any().downcast_ref::<Events<T>>()
            .expect("EventReader type mismatch");
        events.len()
    }

    /// Returns true if there are no readable events.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Iterator over events - holds borrow guard for safety.
pub struct EventIter<'a, T: Send + Sync + 'static> {
    /// Borrow guard - keeps RefCell borrowed for duration of iteration.
    _guard: std::cell::Ref<'a, Box<dyn EventQueue>>,
    /// Pointer to the slice (valid while guard is held).
    ptr: *const T,
    len: usize,
    index: usize,
}

impl<'a, T: Send + Sync + 'static> EventIter<'a, T> {
    fn new(events: &'a RefCell<Box<dyn EventQueue>>) -> Self {
        let guard = events.borrow();
        let events_typed = guard.as_any().downcast_ref::<Events<T>>()
            .expect("EventIter type mismatch");
        let ptr = events_typed.buffer.as_ptr();
        let len = events_typed.buffer.len();
        Self {
            _guard: guard,
            ptr,
            len,
            index: 0,
        }
    }
}

impl<'a, T: Send + Sync + 'static> Iterator for EventIter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.len {
            // Safety: guard keeps the Events alive, index is in bounds
            let event = unsafe { &*self.ptr.add(self.index) };
            self.index += 1;
            Some(event)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, T: Send + Sync + 'static> ExactSizeIterator for EventIter<'a, T> {}

/// State for EventReader system parameter.
pub struct EventReaderState<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for EventReaderState<T> {
    fn default() -> Self {
        Self { _marker: PhantomData }
    }
}

impl<T: Send + Sync + 'static> SystemParam for EventReader<'_, T> {
    type State = EventReaderState<T>;
    type Item<'world, 'state> = EventReader<'world, T>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        let world_mut = world.as_mut();
        let queue = world_mut.event_storage.ensure_queue::<T>();
        
        // Safety: queue lives for 'w (owned by World)
        let queue_ref = unsafe {
            &*(queue as *const RefCell<Box<dyn EventQueue>>)
        };
        
        Ok(EventReader {
            events: queue_ref,
            _marker: PhantomData,
        })
    }

    fn event_type_id() -> Option<TypeId> {
        Some(TypeId::of::<T>())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestEvent {
        value: i32,
    }

    #[test]
    fn events_single_buffer() {
        let mut events = Events::<TestEvent>::new();
        
        // Write events
        events.send(TestEvent { value: 1 });
        events.send(TestEvent { value: 2 });
        
        // Events are immediately readable (single buffer)
        assert_eq!(events.len(), 2);
        let values: Vec<_> = events.iter().map(|e| e.value).collect();
        assert_eq!(values, vec![1, 2]);
        
        // Clear
        events.clear();
        assert!(events.is_empty());
        
        // Write new events
        events.send(TestEvent { value: 3 });
        assert_eq!(events.len(), 1);
        assert_eq!(events.iter().next().unwrap().value, 3);
    }

    #[test]
    fn event_storage_clear_all() {
        let mut storage = EventStorage::new();
        
        // Ensure queues exist and write some events
        {
            let queue = storage.ensure_queue::<TestEvent>();
            let mut q = queue.borrow_mut();
            let events = q.as_any_mut().downcast_mut::<Events<TestEvent>>().unwrap();
            events.send(TestEvent { value: 42 });
        }
        
        // Events are immediately readable
        {
            let queue = storage.queue::<TestEvent>().unwrap();
            let q = queue.borrow();
            let events = q.as_any().downcast_ref::<Events<TestEvent>>().unwrap();
            assert_eq!(events.len(), 1);
        }
        
        // Clear all
        storage.clear_all();
        
        // After clear, events are gone
        {
            let queue = storage.queue::<TestEvent>().unwrap();
            let q = queue.borrow();
            let events = q.as_any().downcast_ref::<Events<TestEvent>>().unwrap();
            assert!(events.is_empty());
        }
    }

    #[test]
    fn event_reader_introspection() {
        use crate::system_param::SystemParam;
        use crate::Query;
        use crate::Component;
        
        #[derive(Clone)]
        struct OtherEvent;
        
        #[derive(Component)]
        struct Pos;
        
        // EventReader reports its event type
        assert!(EventReader::<TestEvent>::is_event_reader());
        assert_eq!(
            EventReader::<TestEvent>::event_type_id(),
            Some(TypeId::of::<TestEvent>())
        );
        
        // Different event types have different TypeIds
        assert_ne!(
            EventReader::<TestEvent>::event_type_id(),
            EventReader::<OtherEvent>::event_type_id()
        );
        
        // EventWriter is NOT an event reader
        assert!(!EventWriter::<TestEvent>::is_event_reader());
        assert_eq!(EventWriter::<TestEvent>::event_type_id(), None);
        
        // Query is NOT an event reader
        assert!(!Query::<&Pos>::is_event_reader());
        assert_eq!(Query::<&Pos>::event_type_id(), None);
    }
}

