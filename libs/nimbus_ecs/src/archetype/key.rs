//! ArchetypeKey - identifies a unique combination of component types.

use std::any::TypeId;
use std::hash::{Hash, Hasher};
use crate::util::TypeHashSet;

// ============================================================================
// ArchetypeKey
// ============================================================================

/// Identifies a unique combination of component types.
/// Uses a HashSet with identity hasher for O(1) lookups.
#[derive(Clone, Default, Debug)]
pub struct ArchetypeKey {
    types: TypeHashSet,
}

impl ArchetypeKey {
    pub fn new(types: Vec<TypeId>) -> Self {
        Self {
            types: types.into_iter().collect(),
        }
    }

    /// Returns an iterator over the component types in this archetype.
    pub fn iter(&self) -> impl Iterator<Item = &TypeId> {
        self.types.iter()
    }

    /// Returns the number of component types in this archetype.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Returns true if this archetype has no component types.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Returns true if this archetype contains the given type.
    #[inline]
    pub fn contains(&self, ty: TypeId) -> bool {
        self.types.contains(&ty)
    }

    /// Returns true if this archetype contains all the given types.
    pub fn contains_all(&self, types: &[TypeId]) -> bool {
        types.iter().all(|ty| self.contains(*ty))
    }

    /// Creates a new archetype key with an additional type.
    /// Returns None if the type already exists.
    pub fn with_type(&self, ty: TypeId) -> Option<Self> {
        if self.contains(ty) {
            return None;
        }
        let mut types = self.types.clone();
        types.insert(ty);
        Some(Self { types })
    }

    /// Creates a new archetype key with multiple additional types.
    /// Returns None if ALL types already exist (no change needed).
    /// If some types exist and some don't, only the new ones are added.
    pub fn with_types(&self, tys: &[TypeId]) -> Option<Self> {
        let mut types = self.types.clone();
        let mut any_added = false;
        
        for &ty in tys {
            if types.insert(ty) {
                any_added = true;
            }
        }
        
        if any_added {
            Some(Self { types })
        } else {
            None // All types already present
        }
    }

    /// Creates a new archetype key without the given type.
    /// Returns None if the type doesn't exist.
    pub fn without_type(&self, ty: TypeId) -> Option<Self> {
        if !self.contains(ty) {
            return None;
        }
        let mut types = self.types.clone();
        types.remove(&ty);
        Some(Self { types })
    }
}

// Implement Hash using XOR of all TypeId hashes (commutative, order-independent)
impl Hash for ArchetypeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // XOR all TypeId hashes together for order-independent hashing
        let mut combined: u64 = 0;
        for ty in &self.types {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            ty.hash(&mut hasher);
            combined ^= hasher.finish();
        }
        combined.hash(state);
    }
}

impl PartialEq for ArchetypeKey {
    fn eq(&self, other: &Self) -> bool {
        self.types == other.types
    }
}

impl Eq for ArchetypeKey {}

impl<'a> IntoIterator for &'a ArchetypeKey {
    type Item = &'a TypeId;
    type IntoIter = hashbrown::hash_set::Iter<'a, TypeId>;

    fn into_iter(self) -> Self::IntoIter {
        self.types.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetype_key_iteration() {
        let types = vec![
            TypeId::of::<u32>(),
            TypeId::of::<String>(),
            TypeId::of::<f64>(),
        ];
        let key = ArchetypeKey::new(types.clone());

        // Test len()
        assert_eq!(key.len(), 3);
        assert!(!key.is_empty());

        // Test iter()
        let collected: Vec<_> = key.iter().copied().collect();
        assert_eq!(collected.len(), 3);
        assert!(collected.contains(&TypeId::of::<u32>()));
        assert!(collected.contains(&TypeId::of::<String>()));
        assert!(collected.contains(&TypeId::of::<f64>()));

        // Test IntoIterator for &ArchetypeKey
        let mut count = 0;
        for type_id in &key {
            assert!(key.contains(*type_id));
            count += 1;
        }
        assert_eq!(count, 3);

        // Test empty key
        let empty_key = ArchetypeKey::default();
        assert_eq!(empty_key.len(), 0);
        assert!(empty_key.is_empty());
        assert_eq!(empty_key.iter().count(), 0);
    }
}

