use std::sync::atomic::{AtomicU64, AtomicI32, AtomicU32, Ordering};
use std::hash::Hash;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use super::types::FutureState;

pub struct TranspositionEntry {
    pub hash: AtomicU64,
    pub score: AtomicI32,
    pub depth: AtomicU32,
}

impl Default for TranspositionEntry {
    fn default() -> Self {
        Self {
            hash: AtomicU64::new(0),
            score: AtomicI32::new(0),
            depth: AtomicU32::new(0),
        }
    }
}

pub struct TranspositionTable {
    table: Vec<TranspositionEntry>,
    mask: u64,
}

impl TranspositionTable {
    pub fn new(size_power_of_two: usize) -> Self {
        let size = 1usize << size_power_of_two;
        let mut table = Vec::with_capacity(size);
        for _ in 0..size {
            table.push(TranspositionEntry::default());
        }
        let mask = (size - 1) as u64;
        Self { table, mask }
    }

    pub fn lookup(&self, hash: u64, depth_remaining: usize) -> Option<i32> {
        if hash == 0 {
            return None;
        }
        let idx = (hash & self.mask) as usize;
        let entry = &self.table[idx];
        
        let entry_hash = entry.hash.load(Ordering::Acquire);
        if entry_hash == hash {
            let entry_depth = entry.depth.load(Ordering::Acquire) as usize;
            if entry_depth >= depth_remaining {
                return Some(entry.score.load(Ordering::Acquire));
            }
        }
        None
    }

    pub fn store(&self, hash: u64, depth_remaining: usize, score: i32) {
        if hash == 0 {
            return;
        }
        let idx = (hash & self.mask) as usize;
        let entry = &self.table[idx];
        
        let entry_depth = entry.depth.load(Ordering::Relaxed) as usize;
        let entry_hash = entry.hash.load(Ordering::Relaxed);
        
        if entry_hash == 0 || entry_hash == hash || depth_remaining >= entry_depth {
            entry.score.store(score, Ordering::Release);
            entry.depth.store(depth_remaining as u32, Ordering::Release);
            entry.hash.store(hash, Ordering::Release);
        }
    }
}

pub(super) fn hash_future_state(state: &FutureState) -> u64 {
    let mut hasher = DefaultHasher::new();
    state.player.cells.hash(&mut hasher);
    state.player.active_cards.hash(&mut hasher);
    state.player.completed_cards.hash(&mut hasher);
    state.player.empty_hexes.hash(&mut hasher);
    
    state.central_groups.hash(&mut hasher);
    state.river_cards.hash(&mut hasher);
    
    state.bag_counts.water.hash(&mut hasher);
    state.bag_counts.mountain.hash(&mut hasher);
    state.bag_counts.trunk.hash(&mut hasher);
    state.bag_counts.foliage.hash(&mut hasher);
    state.bag_counts.field.hash(&mut hasher);
    state.bag_counts.building.hash(&mut hasher);
    state.bag_counts.unknown.hash(&mut hasher);
    
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transposition_table_lookup_store() {
        let tt = TranspositionTable::new(10); // Size 1024
        
        let hash1 = 123456789u64;
        let hash2 = 987654321u64;
        
        // Lookup empty
        assert_eq!(tt.lookup(hash1, 2), None);
        
        // Store and lookup exact depth
        tt.store(hash1, 2, 42);
        assert_eq!(tt.lookup(hash1, 2), Some(42));
        
        // Lookup lower depth (should work)
        assert_eq!(tt.lookup(hash1, 1), Some(42));
        
        // Lookup higher depth (should return None)
        assert_eq!(tt.lookup(hash1, 3), None);
        
        // Store deeper depth and overwrite
        tt.store(hash1, 3, 50);
        assert_eq!(tt.lookup(hash1, 3), Some(50));
        assert_eq!(tt.lookup(hash1, 2), Some(50));
        
        // Store other hash and verify no interference (if no collision)
        tt.store(hash2, 1, 99);
        assert_eq!(tt.lookup(hash2, 1), Some(99));
        assert_eq!(tt.lookup(hash1, 3), Some(50)); // hash1 still there
    }
}

