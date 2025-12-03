use std::fmt;

/// Unique handle that an object inside the world
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl Entity {
    /// A placeholder entity that is never valid.
    /// Used for deferred spawns where the actual entity isn't known yet.
    pub const PLACEHOLDER: Entity = Entity { index: u32::MAX, generation: u32::MAX };
    
    #[inline(always)]
    pub(crate) const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns the dense index associated with this entity.
    #[inline(always)]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Generation is bumped every time an entity id is reused to avoid stale references.
    #[inline(always)]
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

#[derive(Clone, Copy)]
pub(crate) struct EntityLocation {
    pub archetype: usize,
    pub slot: usize,
}

#[derive(Clone)]
struct EntitySlot {
    generation: u32,
    location: Option<EntityLocation>,
}

impl EntitySlot {
    #[inline]
    fn is_alive(&self) -> bool {
        self.location.is_some()
    }
}

impl Default for EntitySlot {
    fn default() -> Self {
        Self {
            generation: 0,
            location: None,
        }
    }
}

/// Internal allocator for `Entity` handles.
#[derive(Default)]
pub(crate) struct Entities {
    slots: Vec<EntitySlot>,
    free: Vec<u32>,
    //Not keeping a free list of entities, just a free list of indices.
    //can create sparseness in the slots vector that may need to be addressed.
    //However: we generally shouldn't iterate entities and just archetypes so likely not a big deal.
}

#[allow(dead_code)]
impl Entities {
    /// Allocates an entity slot. The entity is not alive until a location is set.
    pub fn alloc(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            let slot = &self.slots[index as usize];
            return Entity::new(index, slot.generation);
        }

        let index = self.slots.len() as u32;
        self.slots.push(EntitySlot::default());
        Entity::new(index, 0)
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        let slot = &mut self.slots[entity.index as usize];
        slot.generation = slot.generation.wrapping_add(1);
        slot.location = None;
        self.free.push(entity.index);
        true
    }

    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.slots
            .get(entity.index as usize)
            .map(|slot| slot.is_alive() && slot.generation == entity.generation)
            .unwrap_or(false)
    }

    #[inline]
    pub fn resolve(&self, index: u32) -> Option<Entity> {
        self.slots.get(index as usize).and_then(|slot| {
            if slot.is_alive() {
                Some(Entity::new(index, slot.generation))
            } else {
                None
            }
        })
    }

    #[inline]
    pub fn location(&self, entity: Entity) -> Option<EntityLocation> {
        self.slots
            .get(entity.index as usize)
            .and_then(|slot| slot.location)
    }

    #[inline]
    pub fn set_location(&mut self, entity: Entity, location: EntityLocation) {
        if let Some(slot) = self.slots.get_mut(entity.index as usize) {
            slot.location = Some(location);
        }
    }

    pub(crate) fn iter(&self) -> EntitiesIter<'_> {
        EntitiesIter {
            entities: self,
            index: 0,
        }
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.index, self.generation)
    }
}

#[allow(dead_code)]
pub(crate) struct EntitiesIter<'a> {
    entities: &'a Entities,
    index: usize,
}

impl<'a> Iterator for EntitiesIter<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.entities.slots.len() {
            self.index += 1;

            let slot = &self.entities.slots[self.index];
            if slot.is_alive() {
                return Some(Entity::new(self.index as u32, slot.generation));
            }
        }

        None
    }
}
