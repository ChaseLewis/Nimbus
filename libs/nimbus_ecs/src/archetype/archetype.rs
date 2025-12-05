//! Archetype - stores entities with identical component signatures.

use std::any::Any;

use crate::component::{Component, ComponentId};
use crate::entity::Entity;
use crate::util::{TypeHashMap, ComponentIdHashMap};
use super::ArchetypeKey;

// ============================================================================
// Column traits and types
// ============================================================================

/// Type-erased component column operations.
#[allow(dead_code)]
pub(crate) trait Column: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Returns the number of components in this column.
    fn len(&self) -> usize;

    /// Reserves capacity for at least `additional` more elements.
    fn reserve(&mut self, additional: usize);

    /// Swap-removes a component at the given index, DROPPING the removed value.
    /// Called when despawning an entity.
    fn swap_remove(&mut self, index: usize);

    /// Swap-removes a component at the given index WITHOUT dropping it.
    /// Called after `copy_to` since the value was already moved out.
    /// 
    /// # Safety
    /// The value at `index` must have been moved out (e.g., via `copy_to`).
    unsafe fn swap_remove_no_drop(&mut self, index: usize);

    /// Moves a component from this column to another column of the same type.
    /// The source slot becomes uninitialized after this call.
    /// Caller must call `swap_remove_no_drop` to clean up the slot.
    fn move_to(&self, index: usize, target: &mut dyn Column);

    /// Bulk moves multiple components from this column to another.
    /// `indices` must be sorted in DESCENDING order for safe swap_remove.
    /// Returns the starting index in the target column where elements were placed.
    /// 
    /// # Safety
    /// - All indices must be valid
    /// - Indices must be sorted descending (for swap_remove_no_drop compatibility)
    /// - Target column must be the same type
    unsafe fn move_bulk(&self, indices: &[usize], target: &mut dyn Column) -> usize;

    /// Creates a new empty column of the same type.
    fn create_empty(&self) -> Box<dyn Column>;

    /// Returns a raw pointer to the component at the given index.
    fn get_ptr(&self, index: usize) -> *const u8;

    /// Returns a raw mutable pointer to the component at the given index.
    fn get_mut_ptr(&mut self, index: usize) -> *mut u8;
}

/// Concrete column storage for a specific component type.
pub(crate) struct ColumnData<T> {
    data: Vec<T>,
}

impl<T> ColumnData<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Appends a value to the end of the column.
    /// Use this when adding a new entity to the archetype.
    #[inline]
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// Overwrites the value at the given index, dropping the old value.
    /// Use this when replacing an existing component.
    /// 
    /// # Panics
    /// Panics if index is out of bounds.
    #[inline]
    pub fn set(&mut self, index: usize, value: T) {
        self.data[index] = value;
    }

    /// Writes a value at the given index WITHOUT dropping what's there.
    /// 
    /// # Safety
    /// The caller must ensure:
    /// - `index < self.len()` (slot exists)
    /// - The slot is uninitialized or was previously read with `ptr::read`
    /// - The slot will not be read again before being properly initialized
    #[inline]
    pub unsafe fn write_uninit(&mut self, index: usize, value: T) {
        // Safety: caller guarantees index is valid and slot is uninitialized
        unsafe {
            std::ptr::write(self.data.as_mut_ptr().add(index), value);
        }
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl<T: 'static> Column for ColumnData<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    fn swap_remove(&mut self, index: usize) {
        self.data.swap_remove(index);
    }

    unsafe fn swap_remove_no_drop(&mut self, index: usize) {
        let len = self.data.len();
        debug_assert!(index < len);
        
        if index < len - 1 {
            // Move last element into the slot (overwriting uninitialized memory)
            // Safety: both indices are valid, and we're moving (not copying)
            unsafe {
                let last_ptr = self.data.as_ptr().add(len - 1);
                let slot_ptr = self.data.as_mut_ptr().add(index);
                std::ptr::copy_nonoverlapping(last_ptr, slot_ptr, 1);
            }
        }
        // Shrink the vec without dropping the "removed" element
        // Safety: the slot was either uninitialized or we moved the last element into it
        unsafe {
            self.data.set_len(len - 1);
        }
    }

    fn move_to(&self, index: usize, target: &mut dyn Column) {
        let target = target
            .as_any_mut()
            .downcast_mut::<ColumnData<T>>()
            .expect("column type mismatch");
        // Read (move) the value at index - source slot is now uninitialized
        let ptr = &self.data[index] as *const T;
        let value = unsafe { std::ptr::read(ptr) };
        target.data.push(value);
    }

    unsafe fn move_bulk(&self, indices: &[usize], target: &mut dyn Column) -> usize {
        let target = target
            .as_any_mut()
            .downcast_mut::<ColumnData<T>>()
            .expect("column type mismatch");
        
        let start_idx = target.data.len();
        let count = indices.len();
        
        // Reserve space in target
        target.data.reserve(count);
        
        // Get raw pointer to target's end
        let target_ptr = unsafe { target.data.as_mut_ptr().add(start_idx) };
        
        // Copy each element (indices are in descending order, but we write sequentially)
        for (i, &src_idx) in indices.iter().enumerate() {
            unsafe {
                let src_ptr = self.data.as_ptr().add(src_idx);
                std::ptr::copy_nonoverlapping(src_ptr, target_ptr.add(i), 1);
            }
        }
        
        // Update target length
        unsafe {
            target.data.set_len(start_idx + count);
        }
        
        start_idx
    }

    fn create_empty(&self) -> Box<dyn Column> {
        Box::new(ColumnData::<T>::new())
    }

    fn get_ptr(&self, index: usize) -> *const u8 {
        &self.data[index] as *const T as *const u8
    }

    fn get_mut_ptr(&mut self, index: usize) -> *mut u8 {
        &mut self.data[index] as *mut T as *mut u8
    }
}

// ============================================================================
// Archetype
// ============================================================================

/// An archetype stores entities with identical component signatures.
/// Components are stored in dense SoA columns for efficient iteration.
pub(crate) struct Archetype {
    pub(super) key: ArchetypeKey,
    /// Entities in this archetype (dense, indices match column indices)
    entities: Vec<Entity>,
    /// Component columns indexed by ComponentId
    pub(super) columns: ComponentIdHashMap<Box<dyn Column>>,

    // Graph edges for O(1) archetype transitions (computed lazily, cached forever)
    /// Cache: adding single ComponentId → target archetype index
    pub(super) add_edges: ComponentIdHashMap<usize>,
    /// Cache: removing single ComponentId → target archetype index  
    pub(super) remove_edges: ComponentIdHashMap<usize>,
    /// Cache: adding Bundle (by Bundle's TypeId) → target archetype index
    /// This caches entire bundle transitions for O(1) repeated bundle inserts
    pub(super) bundle_add_edges: TypeHashMap<usize>,
}

#[allow(dead_code)]
impl Archetype {
    pub fn new(key: ArchetypeKey) -> Self {
        Self {
            key,
            entities: Vec::new(),
            columns: ComponentIdHashMap::default(),
            add_edges: ComponentIdHashMap::default(),
            remove_edges: ComponentIdHashMap::default(),
            bundle_add_edges: TypeHashMap::default(),
        }
    }

    /// Returns the archetype key.
    #[inline]
    pub fn key(&self) -> &ArchetypeKey {
        &self.key
    }

    /// Returns a reference to the columns map for iteration.
    #[inline]
    pub fn columns(&self) -> &ComponentIdHashMap<Box<dyn Column>> {
        &self.columns
    }

    /// Returns the number of entities in this archetype.
    #[inline]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns the entities slice.
    #[inline]
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Returns the entity at the given row index.
    #[inline]
    pub fn entity_at(&self, row: usize) -> Entity {
        self.entities[row]
    }

    /// Swap-removes just the entity at the given row (not the columns).
    /// Used during migration after columns have been moved separately.
    #[inline]
    pub fn entities_swap_remove(&mut self, row: usize) {
        self.entities.swap_remove(row);
    }

    /// Ensures a column exists for the given component type.
    pub fn ensure_column<T: Component>(&mut self) {
        let component_id = ComponentId::of::<T>();
        self.columns
            .entry(component_id)
            .or_insert_with(|| Box::new(ColumnData::<T>::new()));
    }

    /// Gets a reference to a typed column.
    #[inline]
    pub fn column<T: Component>(&self) -> Option<&ColumnData<T>> {
        self.columns
            .get(&ComponentId::of::<T>())
            .and_then(|col| col.as_any().downcast_ref::<ColumnData<T>>())
    }

    /// Gets a mutable reference to a typed column.
    #[inline]
    pub fn column_mut<T: Component>(&mut self) -> Option<&mut ColumnData<T>> {
        self.columns
            .get_mut(&ComponentId::of::<T>())
            .and_then(|col| col.as_any_mut().downcast_mut::<ColumnData<T>>())
    }

    /// Gets a type-erased column by ComponentId.
    #[inline]
    pub fn column_raw(&self, component_id: ComponentId) -> Option<&dyn Column> {
        self.columns.get(&component_id).map(|c| c.as_ref())
    }

    /// Gets a mutable type-erased column by ComponentId.
    #[inline]
    pub fn column_raw_mut(&mut self, component_id: ComponentId) -> Option<&mut dyn Column> {
        self.columns.get_mut(&component_id).map(|c| c.as_mut())
    }

    /// Ensures a column exists for the given ComponentId, creating it from a source column if needed.
    pub fn ensure_column_from(&mut self, component_id: ComponentId, source: &dyn Column) {
        self.columns.entry(component_id).or_insert_with(|| source.create_empty());
    }


    //This push_entity -> push_component things seems weird that we aren't taking an entity and its components 
    //in a single method

    /// Adds an entity to this archetype. Returns the row index.
    /// Caller must ensure all columns have a value pushed for this entity.
    pub fn push_entity(&mut self, entity: Entity) -> usize {
        let index = self.entities.len();
        self.entities.push(entity);
        index
    }

    /// Reserves capacity for `additional` entities in both the entity list and all columns.
    pub fn reserve_bulk(&mut self, additional: usize) {
        self.entities.reserve(additional);
        for column in self.columns.values_mut() {
            column.reserve(additional);
        }
    }

    /// Pushes multiple entities at once. Returns the starting row index.
    /// Caller must ensure all columns have values pushed for these entities.
    pub fn push_entities_bulk(&mut self, entities: &[Entity]) -> usize {
        let start_idx = self.entities.len();
        self.entities.extend_from_slice(entities);
        start_idx
    }

    /// Bulk swap-removes multiple rows WITHOUT dropping their components.
    /// `rows` MUST be sorted in descending order.
    /// Returns the entities that were swapped into each row (if any).
    /// 
    /// # Safety
    /// All components at these rows must have been moved out.
    pub unsafe fn swap_remove_moved_bulk(&mut self, rows: &[usize]) -> Vec<Option<Entity>> {
        let mut swapped = Vec::with_capacity(rows.len());
        
        for &row in rows {
            if row >= self.entities.len() {
                swapped.push(None);
                continue;
            }

            // Swap-remove from all columns WITHOUT dropping
            for column in self.columns.values_mut() {
                unsafe { column.swap_remove_no_drop(row); }
            }

            // Record the entity that was swapped in (if any)
            let len = self.entities.len();
            let swap_result = if row < len - 1 {
                Some(self.entities[len - 1])
            } else {
                None
            };
            
            self.entities.swap_remove(row);
            swapped.push(swap_result);
        }
        
        swapped
    }

    /// Pushes a component value to the end of the column.
    /// Used when adding a new entity to the archetype.
    pub fn push_component<T: Component>(&mut self, value: T) {
        self.ensure_column::<T>();
        self.column_mut::<T>().unwrap().push(value);
    }

    /// Sets a component value at a specific row, dropping the old value.
    /// Used when replacing an existing component on an entity.
    pub fn set_component<T: Component>(&mut self, row: usize, value: T) {
        self.ensure_column::<T>();
        self.column_mut::<T>().unwrap().set(row, value);
    }

    /// Writes a component value at a specific row WITHOUT dropping.
    /// 
    /// # Safety
    /// The slot must be uninitialized (e.g., after copy_to from another archetype
    /// that didn't drop, or a freshly extended column).
    #[allow(dead_code)]
    pub unsafe fn write_component_uninit<T: Component>(&mut self, row: usize, value: T) {
        self.ensure_column::<T>();
        // Safety: caller guarantees slot is uninitialized
        unsafe {
            self.column_mut::<T>().unwrap().write_uninit(row, value);
        }
    }

    /// Gets a component reference by row index.
    #[inline]
    pub fn get<T: Component>(&self, row: usize) -> Option<&T> {
        self.column::<T>().and_then(|col| col.get(row))
    }

    /// Gets a mutable component reference by row index.
    #[inline]
    pub fn get_mut<T: Component>(&mut self, row: usize) -> Option<&mut T> {
        self.column_mut::<T>().and_then(|col| col.get_mut(row))
    }

    /// Removes an entity at the given row index using swap-remove.
    /// Drops all components at that row.
    /// Returns the entity that was swapped into this slot (if any).
    pub fn swap_remove(&mut self, row: usize) -> Option<Entity> {
        if row >= self.entities.len() {
            return None;
        }

        // Swap-remove from all columns (drops the removed components)
        for column in self.columns.values_mut() {
            column.swap_remove(row);
        }

        // Swap-remove from entities
        self.entities.swap_remove(row);

        // Return the entity that was swapped in (if any)
        if row < self.entities.len() {
            Some(self.entities[row])
        } else {
            None
        }
    }

    /// Removes an entity at the given row index after its components were moved out.
    /// Does NOT drop the components (they were already moved via `move_to`).
    /// Returns the entity that was swapped into this slot (if any).
    /// 
    /// # Safety
    /// All components at `row` must have been moved out via `column.move_to()`.
    pub unsafe fn swap_remove_moved(&mut self, row: usize) -> Option<Entity> {
        if row >= self.entities.len() {
            return None;
        }

        // Swap-remove from all columns WITHOUT dropping (values were moved out)
        for column in self.columns.values_mut() {
            // Safety: caller guarantees all components were moved out
            unsafe {
                column.swap_remove_no_drop(row);
            }
        }

        // Swap-remove from entities
        self.entities.swap_remove(row);

        // Return the entity that was swapped in (if any)
        if row < self.entities.len() {
            Some(self.entities[row])
        } else {
            None
        }
    }

    /// Returns column pointers for fast iteration.
    /// Returns (entities_ptr, column_ptr, len)
    #[inline]
    pub fn column_ptr<T: Component>(&self) -> Option<(*const T, usize)> {
        self.column::<T>().map(|col| {
            let slice = col.as_slice();
            (slice.as_ptr(), slice.len())
        })
    }

    /// Returns mutable column pointer for fast iteration.
    #[inline]
    pub fn column_mut_ptr<T: Component>(&mut self) -> Option<(*mut T, usize)> {
        self.column_mut::<T>().map(|col| {
            let slice = col.as_mut_slice();
            (slice.as_mut_ptr(), slice.len())
        })
    }
}

