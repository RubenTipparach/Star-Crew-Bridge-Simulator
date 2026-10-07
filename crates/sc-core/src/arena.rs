//! Fixed-capacity arenas with generational ids: the engine's stand-in for an ECS (design section 4).
//!
//! Crew, craft, projectiles and network entities live in plain arrays sized at startup from the
//! budget (CLAUDE.md 2: allocate up front, fail loudly). An id names a slot and the generation it
//! was filled in, so an id kept after its thing was removed can never reach the slot's next
//! occupant. It lives in the core because saves, the server and replays all name things by these ids.

/// A stable reference to an arena slot: its index and the generation that filled it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id {
    index: u32,
    generation: u32,
}

impl Id {
    /// The slot index, for tables kept beside the arena.
    pub fn index(self) -> u32 {
        self.index
    }
    /// The generation the slot had when this id was made.
    pub fn generation(self) -> u32 {
        self.generation
    }
}

/// The arena is at its capacity: the insert changed nothing (CLAUDE.md 6.6, a limit handled whole).
#[derive(Debug, PartialEq, Eq)]
pub struct Full {
    /// The arena's fixed capacity.
    pub capacity: usize,
}

impl std::fmt::Display for Full {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the arena is full at its capacity of {}", self.capacity)
    }
}
impl std::error::Error for Full {}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// A fixed-capacity arena. Iteration is in slot order, which is stable (CLAUDE.md 6.4).
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    len: usize,
}

impl<T> Arena<T> {
    /// An arena of exactly `capacity` slots, all allocated now.
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = u32::try_from(capacity).expect("arena capacity fits in u32");
        let slots = (0..cap).map(|_| Slot { generation: 0, value: None }).collect();
        // Free slots are taken lowest first, so the order things are inserted in decides their ids.
        let free = (0..cap).rev().collect();
        Self { slots, free, len: 0 }
    }

    /// The fixed capacity.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// How many slots are filled.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether no slot is filled.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Put `value` in the lowest free slot and return its id, or `Full` without changing anything.
    pub fn insert(&mut self, value: T) -> Result<Id, Full> {
        let index = self.free.pop().ok_or(Full { capacity: self.capacity() })?;
        let slot = &mut self.slots[index as usize];
        slot.value = Some(value);
        self.len += 1;
        Ok(Id { index, generation: slot.generation })
    }

    /// Take the value `id` names out of the arena; `None` when the id is stale or out of range.
    pub fn remove(&mut self, id: Id) -> Option<T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation || slot.value.is_none() {
            return None;
        }
        let v = slot.value.take();
        slot.generation = slot.generation.wrapping_add(1);
        // Keep the free list sorted high to low so the lowest index is reused first.
        let pos = self.free.partition_point(|&f| f > id.index);
        self.free.insert(pos, id.index);
        self.len -= 1;
        v
    }

    /// The value `id` names, if it is still there.
    pub fn get(&self, id: Id) -> Option<&T> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation == id.generation {
            slot.value.as_ref()
        } else {
            None
        }
    }

    /// The value `id` names, mutably, if it is still there.
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation == id.generation {
            slot.value.as_mut()
        } else {
            None
        }
    }

    /// Every filled slot with its id, in slot order.
    pub fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.value.as_ref().map(|v| (Id { index: i as u32, generation: s.generation }, v)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stale_id_never_reaches_the_slots_next_occupant() {
        let mut a = Arena::with_capacity(2);
        let fighter = a.insert("Swift").unwrap();
        assert_eq!(a.remove(fighter), Some("Swift"));
        let shuttle = a.insert("Petrel").unwrap();
        assert_eq!(shuttle.index(), fighter.index(), "the slot is reused");
        assert_eq!(a.get(fighter), None, "the old id must not see the shuttle");
        assert_eq!(a.get(shuttle), Some(&"Petrel"));
    }

    #[test]
    fn a_full_arena_refuses_and_changes_nothing() {
        let mut a = Arena::with_capacity(1);
        a.insert(1).unwrap();
        assert_eq!(a.insert(2), Err(Full { capacity: 1 }));
        assert_eq!(a.len(), 1);
        assert_eq!(a.iter().map(|(_, v)| *v).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn ids_follow_insertion_order_and_iteration_is_slot_order() {
        let mut a = Arena::with_capacity(4);
        let ids: Vec<_> = (0..3).map(|i| a.insert(i).unwrap()).collect();
        assert_eq!(ids.iter().map(|i| i.index()).collect::<Vec<_>>(), vec![0, 1, 2]);
        a.remove(ids[0]);
        let again = a.insert(9).unwrap();
        assert_eq!(again.index(), 0, "the lowest free slot is taken first");
        assert_eq!(a.iter().map(|(_, v)| *v).collect::<Vec<_>>(), vec![9, 1, 2]);
    }
}
