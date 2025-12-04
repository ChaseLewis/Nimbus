//! Archetypes - collection of all archetypes in the world.

use std::any::TypeId;
use hashbrown::HashMap;
use super::{Archetype, ArchetypeKey};
use crate::util::TypeHashMap;

/// Collection of all archetypes in the world.
pub struct Archetypes {
    lookup: HashMap<ArchetypeKey, usize>,
    data: Vec<Archetype>,
    /// Reverse index: TypeId -> archetype indices containing that component.
    /// Enables O(smallest_set) query matching instead of O(all_archetypes).
    component_index: TypeHashMap<Vec<usize>>,
}

/// A Send+Sync wrapper around a pointer to Archetypes.
/// 
/// # Safety
/// This is safe to use when:
/// - The Archetypes reference outlives all uses of this pointer
/// - No structural mutations (add/remove archetypes) occur during use
/// - Only disjoint row ranges are accessed across threads
#[derive(Clone, Copy)]
pub struct SendArchetypesPtr(pub(crate) *const Archetypes);

unsafe impl Send for SendArchetypesPtr {}
unsafe impl Sync for SendArchetypesPtr {}

impl SendArchetypesPtr {
    /// Creates a new SendArchetypesPtr from an Archetypes reference.
    #[inline]
    pub fn new(archetypes: &Archetypes) -> Self {
        Self(archetypes as *const Archetypes)
    }
    
    /// Gets an archetype by index.
    /// 
    /// # Safety
    /// The Archetypes must still be valid (not dropped or structurally modified).
    #[inline]
    pub unsafe fn get(&self, index: usize) -> Option<&Archetype> {
        unsafe { (*self.0).get(index) }
    }
}

#[allow(dead_code)]
impl Archetypes {
    pub fn new() -> Self {
        Self {
            lookup: HashMap::new(),
            data: Vec::new(),
            component_index: TypeHashMap::default(),
        }
    }

    /// Updates the component index when a new archetype is added.
    fn index_archetype(&mut self, index: usize, key: &ArchetypeKey) {
        for &type_id in key.iter() {
            self.component_index
                .entry(type_id)
                .or_insert_with(Vec::new)
                .push(index);
        }
    }

    /// Ensures an archetype exists for the given key, returns its index.
    pub fn ensure(&mut self, key: ArchetypeKey) -> usize {
        if let Some(&index) = self.lookup.get(&key) {
            return index;
        }

        let index = self.data.len();
        self.index_archetype(index, &key);
        self.lookup.insert(key.clone(), index);
        self.data.push(Archetype::new(key));
        index
    }

    /// Ensures an archetype exists for the given key, initializing columns from a source archetype.
    /// Columns that exist in the source will be created as empty columns of the same type.
    /// Returns the archetype index.
    pub fn ensure_from(&mut self, key: ArchetypeKey, source_index: usize) -> usize {
        if let Some(&index) = self.lookup.get(&key) {
            return index;
        }

        // Collect column creators from source before creating new archetype
        // (to avoid borrowing issues)
        let column_creators: Vec<_> = {
            let source = &self.data[source_index];
            key.iter()
                .filter_map(|&type_id| {
                    source.column_raw(type_id).map(|col| (type_id, col.create_empty()))
                })
                .collect()
        };

        let index = self.data.len();
        self.index_archetype(index, &key);
        self.lookup.insert(key.clone(), index);
        
        let mut archetype = Archetype::new(key);
        // Initialize columns from source
        for (type_id, column) in column_creators {
            archetype.columns.insert(type_id, column);
        }
        
        self.data.push(archetype);
        index
    }

    /// Gets an archetype by index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&Archetype> {
        self.data.get(index)
    }

    /// Gets a mutable archetype by index.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Archetype> {
        self.data.get_mut(index)
    }

    /// Returns the archetype key for an index.
    pub fn key(&self, archetype: usize) -> Option<&ArchetypeKey> {
        self.data.get(archetype).map(|arch| &arch.key)
    }

    /// Finds the archetype index for a given key.
    pub fn find(&self, key: &ArchetypeKey) -> Option<usize> {
        self.lookup.get(key).copied()
    }

    /// Returns an iterator over all archetypes.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &Archetype)> {
        self.data.iter().enumerate()
    }

    /// Returns a mutable iterator over all archetypes.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut Archetype)> {
        self.data.iter_mut().enumerate()
    }

    /// Returns indices of archetypes that contain all the given types.
    /// Uses the component index for O(smallest_set) lookup instead of O(all_archetypes).
    pub fn matching(&self, required_types: &[TypeId]) -> Vec<usize> {
        if required_types.is_empty() {
            // No requirements - all archetypes match
            return (0..self.data.len()).collect();
        }

        // Find the rarest component (smallest set) to start with
        let mut smallest_set: Option<&Vec<usize>> = None;
        let mut smallest_len = usize::MAX;

        for &type_id in required_types {
            match self.component_index.get(&type_id) {
                Some(indices) if indices.len() < smallest_len => {
                    smallest_len = indices.len();
                    smallest_set = Some(indices);
                }
                None => {
                    // No archetypes have this component - empty result
                    return Vec::new();
                }
                _ => {}
            }
        }

        let candidates = smallest_set.unwrap();
        
        if required_types.len() == 1 {
            // Single component - no need to filter further
            return candidates.clone();
        }

        // Filter candidates by checking they have ALL required types
        candidates
            .iter()
            .copied()
            .filter(|&idx| {
                let arch = &self.data[idx];
                required_types.iter().all(|ty| arch.key.contains(*ty))
            })
            .collect()
    }

    /// Returns indices of archetypes matching required types but excluding excluded types.
    /// Uses the component index for efficient initial filtering.
    pub fn matching_filtered(
        &self,
        required_types: &[TypeId],
        excluded_types: &[TypeId],
    ) -> Vec<usize> {
        if required_types.is_empty() && excluded_types.is_empty() {
            return (0..self.data.len()).collect();
        }

        // Get candidates from required types
        let candidates = if required_types.is_empty() {
            (0..self.data.len()).collect::<Vec<_>>()
        } else {
            self.matching(required_types)
        };

        if excluded_types.is_empty() {
            return candidates;
        }

        // Filter out archetypes with excluded types
        candidates
            .into_iter()
            .filter(|&idx| {
                let arch = &self.data[idx];
                excluded_types.iter().all(|ty| !arch.key.contains(*ty))
            })
            .collect()
    }

    /// Returns archetypes containing a specific component type.
    #[inline]
    pub fn with_component(&self, type_id: TypeId) -> Option<&[usize]> {
        self.component_index.get(&type_id).map(|v| v.as_slice())
    }

    /// Returns the number of archetypes.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    // ========================================================================
    // Graph-cached transitions (O(1) after first computation)
    // ========================================================================

    /// Gets the target archetype for adding a component type.
    /// Uses cached graph edge if available, otherwise computes and caches.
    /// Returns None if the source archetype already contains the type.
    pub fn get_add_target(&mut self, source: usize, adding: TypeId) -> Option<usize> {
        // Check cache first (fast path)
        if let Some(&cached) = self.data[source].add_edges.get(&adding) {
            return Some(cached);
        }

        // Source already has this type - no transition possible
        if self.data[source].key.contains(adding) {
            return None;
        }

        // Compute new key
        let new_key = self.data[source].key.with_type(adding)?;
        
        // Ensure target archetype exists, copying column structure from source
        let target = self.ensure_from(new_key, source);

        // Cache the edge for future lookups
        self.data[source].add_edges.insert(adding, target);

        Some(target)
    }

    /// Gets the target archetype for removing a component type.
    /// Uses cached graph edge if available, otherwise computes and caches.
    /// Returns None if the source archetype doesn't contain the type.
    pub fn get_remove_target(&mut self, source: usize, removing: TypeId) -> Option<usize> {
        // Check cache first (fast path)
        if let Some(&cached) = self.data[source].remove_edges.get(&removing) {
            return Some(cached);
        }

        // Source doesn't have this type - no transition possible
        if !self.data[source].key.contains(removing) {
            return None;
        }

        // Compute new key
        let new_key = self.data[source].key.without_type(removing)?;

        // Ensure target archetype exists, copying column structure from source
        let target = self.ensure_from(new_key, source);

        // Cache the edge for future lookups
        self.data[source].remove_edges.insert(removing, target);

        Some(target)
    }

    /// Gets the target archetype for adding a bundle.
    /// Uses Bundle's TypeId as the cache key for O(1) repeated lookups.
    /// 
    /// # Arguments
    /// * `source` - Source archetype index
    /// * `bundle_type_id` - TypeId::of::<B>() where B is the bundle type
    /// * `component_type_ids` - The individual component TypeIds in the bundle
    /// 
    /// Returns None if all component types are already present.
    pub fn get_add_bundle_target(
        &mut self, 
        source: usize, 
        bundle_type_id: TypeId,
        component_type_ids: &[TypeId],
    ) -> Option<usize> {
        // Fast path: check bundle cache (keyed by Bundle's TypeId)
        if let Some(&cached) = self.data[source].bundle_add_edges.get(&bundle_type_id) {
            return Some(cached);
        }

        // Compute new key using with_types (single allocation, no intermediate keys)
        let new_key = self.data[source].key.with_types(component_type_ids)?;

        // Check if this exact target archetype already exists
        let target = if let Some(&existing) = self.lookup.get(&new_key) {
            existing
        } else {
            // Create new archetype with columns initialized from source
            self.ensure_from(new_key, source)
        };

        // Cache by Bundle's TypeId for O(1) future lookups
        self.data[source].bundle_add_edges.insert(bundle_type_id, target);

        Some(target)
    }
}

impl Default for Archetypes {
    fn default() -> Self {
        Self::new()
    }
}

