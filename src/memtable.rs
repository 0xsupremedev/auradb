use crate::error::{Error, Result};
use crate::storage::{Entry, Key, Value, ValuePointer};
use crossbeam::epoch::{self, Atomic, Owned, Shared};
use parking_lot::RwLock;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use tracing::{debug, trace};

/// Memtable implementation trait
pub trait MemtableImpl: Send + Sync {
    /// Insert an entry into the memtable
    fn insert(&mut self, entry: Entry) -> Result<()>;
    
    /// Get an entry by key
    fn get(&self, key: &Key) -> Result<Option<Entry>>;
    
    /// Delete an entry by key
    fn delete(&mut self, key: &Key, sequence: u64) -> Result<()>;
    
    /// Get all entries in sorted order
    fn iter(&self) -> Box<dyn Iterator<Item = Entry> + '_>;
    
    /// Get the number of entries
    fn len(&self) -> usize;
    
    /// Check if memtable is empty
    fn is_empty(&self) -> bool;
    
    /// Get the approximate memory usage in bytes
    fn memory_usage(&self) -> usize;
    
    /// Clear all entries
    fn clear(&mut self);
}

/// Skip list node for lock-free memtable
#[derive(Debug, Clone)]
struct SkipListNode {
    /// Entry data
    entry: Entry,
    /// Next pointers at different levels
    next: Vec<Atomic<SkipListNode>>,
    /// Node level
    level: usize,
}

impl SkipListNode {
    /// Create a new skip list node
    fn new(entry: Entry, level: usize) -> Self {
        let mut next = Vec::with_capacity(level + 1);
        for _ in 0..=level {
            next.push(Atomic::null());
        }
        
        Self {
            entry,
            next,
            level,
        }
    }
    
    /// Get the next node at a specific level
    fn next_at(&self, level: usize) -> &Atomic<SkipListNode> {
        &self.next[level]
    }
}

/// Lock-free skip list memtable implementation
pub struct SkipListMemtable {
    /// Head node
    head: Atomic<SkipListNode>,
    /// Maximum level
    max_level: usize,
    /// Current number of entries
    entry_count: AtomicU64,
    /// Memory usage estimate
    memory_usage: AtomicU64,
}

impl SkipListMemtable {
    /// Create a new skip list memtable
    pub fn new() -> Self {
        // Create head node with maximum level
        let head_entry = Entry::new(
            Key::new(vec![]), // Empty key as sentinel
            Value::new(vec![]),
            0,
        );
        let head = Owned::new(SkipListNode::new(head_entry, 32));
        
        Self {
            head: Atomic::from(head),
            max_level: 32,
            entry_count: AtomicU64::new(0),
            memory_usage: AtomicU64::new(0),
        }
    }
    
    /// Generate a random level for new nodes
    fn random_level(&self) -> usize {
        let mut level = 0;
        let mut rng = fastrand::Rng::new();
        
        while level < self.max_level && rng.u64(..100) < 25 {
            level += 1;
        }
        
        level
    }
    
    /// Find the node with the given key and its predecessors
    fn find_node(&self, key: &Key) -> (Vec<Shared<SkipListNode>>, Vec<Shared<SkipListNode>>) {
        let mut preds = Vec::with_capacity(self.max_level + 1);
        let mut currs = Vec::with_capacity(self.max_level + 1);
        
        // Initialize with head
        for _ in 0..=self.max_level {
            preds.push(Shared::null());
            currs.push(Shared::null());
        }
        
        let guard = epoch::pin();
        let mut pred = self.head.load(AtomicOrdering::Acquire, &guard);
        
        // Search from top level down
        for level in (0..=self.max_level).rev() {
            let mut curr = pred;
            
            // Traverse at current level
            while let Some(curr_ref) = curr.as_ref() {
                let next = curr_ref.next_at(level).load(AtomicOrdering::Acquire, &guard);
                
                if let Some(next_ref) = next.as_ref() {
                    match next_ref.entry.key.cmp(key) {
                        Ordering::Less => {
                            pred = next;
                            curr = next;
                        }
                        Ordering::Equal => {
                            preds[level] = pred;
                            currs[level] = curr;
                            break;
                        }
                        Ordering::Greater => {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            
            preds[level] = pred;
            currs[level] = curr;
        }
        
        (preds, currs)
    }
}

impl MemtableImpl for SkipListMemtable {
    fn insert(&mut self, entry: Entry) -> Result<()> {
        let level = self.random_level();
        let new_node = Owned::new(SkipListNode::new(entry.clone(), level));
        
        let guard = epoch::pin();
        let (preds, currs) = self.find_node(&entry.key);
        
        // Check if key already exists
        if let Some(curr) = currs[0].as_ref() {
            if curr.entry.key == entry.key {
                // Update existing entry
                // In a real implementation, you'd want to handle this more carefully
                return Ok(());
            }
        }
        
        // Link the new node
        for i in 0..=level {
            if let Some(pred) = preds[i].as_ref() {
                new_node.next[i].store(
                    pred.next_at(i).load(AtomicOrdering::Acquire, &guard),
                    AtomicOrdering::Release,
                );
                pred.next_at(i).store(new_node.clone(), AtomicOrdering::Release);
            }
        }
        
        // Update counters
        self.entry_count.fetch_add(1, AtomicOrdering::Relaxed);
        self.memory_usage.fetch_add(
            entry.key.len() as u64 + entry.value.as_ref().map_or(0, |v| v.len() as u64),
            AtomicOrdering::Relaxed,
        );
        
        Ok(())
    }
    
    fn get(&self, key: &Key) -> Result<Option<Entry>> {
        let guard = epoch::pin();
        let (_, currs) = self.find_node(key);
        
        if let Some(curr) = currs[0].as_ref() {
            if curr.entry.key == *key {
                return Ok(Some(curr.entry.clone()));
            }
        }
        
        Ok(None)
    }
    
    fn delete(&mut self, key: &Key, sequence: u64) -> Result<()> {
        let guard = epoch::pin();
        let (preds, currs) = self.find_node(key);
        
        if let Some(curr) = currs[0].as_ref() {
            if curr.entry.key == *key {
                // Mark as deleted by setting a tombstone
                let delete_entry = Entry::delete(key.clone(), sequence);
                let _ = self.insert(delete_entry);
                return Ok(());
            }
        }
        
        Ok(())
    }
    
    fn iter(&self) -> Box<dyn Iterator<Item = Entry> + '_> {
        // This is a simplified iterator - in practice you'd want a proper iterator
        Box::new(std::iter::empty())
    }
    
    fn len(&self) -> usize {
        self.entry_count.load(AtomicOrdering::Relaxed) as usize
    }
    
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    fn memory_usage(&self) -> usize {
        self.memory_usage.load(AtomicOrdering::Relaxed) as usize
    }
    
    fn clear(&mut self) {
        // In a lock-free structure, clearing is complex
        // For now, we'll just reset counters
        self.entry_count.store(0, AtomicOrdering::Relaxed);
        self.memory_usage.store(0, AtomicOrdering::Relaxed);
    }
}

/// B-tree memtable implementation
pub struct BTreeMemtable {
    /// Internal B-tree map
    map: RwLock<BTreeMap<Key, Entry>>,
    /// Memory usage estimate
    memory_usage: AtomicU64,
}

impl BTreeMemtable {
    /// Create a new B-tree memtable
    pub fn new() -> Self {
        Self {
            map: RwLock::new(BTreeMap::new()),
            memory_usage: AtomicU64::new(0),
        }
    }
}

impl MemtableImpl for BTreeMemtable {
    fn insert(&mut self, entry: Entry) -> Result<()> {
        let mut map = self.map.write();
        let old_entry = map.insert(entry.key.clone(), entry.clone());
        
        // Update memory usage
        if let Some(old) = old_entry {
            self.memory_usage.fetch_sub(
                old.key.len() as u64 + old.value.as_ref().map_or(0, |v| v.len() as u64),
                AtomicOrdering::Relaxed,
            );
        }
        
        self.memory_usage.fetch_add(
            entry.key.len() as u64 + entry.value.as_ref().map_or(0, |v| v.len() as u64),
            AtomicOrdering::Relaxed,
        );
        
        Ok(())
    }
    
    fn get(&self, key: &Key) -> Result<Option<Entry>> {
        let map = self.map.read();
        Ok(map.get(key).cloned())
    }
    
    fn delete(&mut self, key: &Key, sequence: u64) -> Result<()> {
        let mut map = self.map.write();
        
        if let Some(old_entry) = map.remove(key) {
            // Update memory usage
            self.memory_usage.fetch_sub(
                old_entry.key.len() as u64 + old_entry.value.as_ref().map_or(0, |v| v.len() as u64),
                AtomicOrdering::Relaxed,
            );
            
            // Insert tombstone
            let delete_entry = Entry::delete(key.clone(), sequence);
            let _ = map.insert(key.clone(), delete_entry);
        }
        
        Ok(())
    }
    
    fn iter(&self) -> Box<dyn Iterator<Item = Entry> + '_> {
        let map = self.map.read();
        Box::new(map.values().cloned().collect::<Vec<_>>().into_iter())
    }
    
    fn len(&self) -> usize {
        self.map.read().len()
    }
    
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    fn memory_usage(&self) -> usize {
        self.memory_usage.load(AtomicOrdering::Relaxed) as usize
    }
    
    fn clear(&mut self) {
        let mut map = self.map.write();
        map.clear();
        self.memory_usage.store(0, AtomicOrdering::Relaxed);
    }
}

/// Adaptive Radix Tree (ART) memtable implementation
/// This is a simplified version - a full ART implementation would be much more complex
pub struct ArtMemtable {
    /// Internal map (simplified for now)
    map: RwLock<BTreeMap<Key, Entry>>,
    /// Memory usage estimate
    memory_usage: AtomicU64,
}

impl ArtMemtable {
    /// Create a new ART memtable
    pub fn new() -> Self {
        Self {
            map: RwLock::new(BTreeMap::new()),
            memory_usage: AtomicU64::new(0),
        }
    }
}

impl MemtableImpl for ArtMemtable {
    fn insert(&mut self, entry: Entry) -> Result<()> {
        let mut map = self.map.write();
        let old_entry = map.insert(entry.key.clone(), entry.clone());
        
        // Update memory usage
        if let Some(old) = old_entry {
            self.memory_usage.fetch_sub(
                old.key.len() as u64 + old.value.as_ref().map_or(0, |v| v.len() as u64),
                AtomicOrdering::Relaxed,
            );
        }
        
        self.memory_usage.fetch_add(
            entry.key.len() as u64 + entry.value.as_ref().map_or(0, |v| v.len() as u64),
            AtomicOrdering::Relaxed,
        );
        
        Ok(())
    }
    
    fn get(&self, key: &Key) -> Result<Option<Entry>> {
        let map = self.map.read();
        Ok(map.get(key).cloned())
    }
    
    fn delete(&mut self, key: &Key, sequence: u64) -> Result<()> {
        let mut map = self.map.write();
        
        if let Some(old_entry) = map.remove(key) {
            // Update memory usage
            self.memory_usage.fetch_sub(
                old_entry.key.len() as u64 + old_entry.value.as_ref().map_or(0, |v| v.len() as u64),
                AtomicOrdering::Relaxed,
            );
            
            // Insert tombstone
            let delete_entry = Entry::delete(key.clone(), sequence);
            let _ = map.insert(key.clone(), delete_entry);
        }
        
        Ok(())
    }
    
    fn iter(&self) -> Box<dyn Iterator<Item = Entry> + '_> {
        let map = self.map.read();
        Box::new(map.values().cloned().collect::<Vec<_>>().into_iter())
    }
    
    fn len(&self) -> usize {
        self.map.read().len()
    }
    
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    fn memory_usage(&self) -> usize {
        self.memory_usage.load(AtomicOrdering::Relaxed) as usize
    }
    
    fn clear(&mut self) {
        let mut map = self.map.write();
        map.clear();
        self.memory_usage.store(0, AtomicOrdering::Relaxed);
    }
}

/// Main memtable that wraps the implementation
pub struct Memtable {
    /// Implementation
    implementation: Box<dyn MemtableImpl>,
    /// Maximum size in bytes
    max_size: usize,
    /// Flush threshold
    flush_threshold: f64,
}

impl Memtable {
    /// Create a new memtable with the specified implementation
    pub fn new(implementation: Box<dyn MemtableImpl>, max_size: usize, flush_threshold: f64) -> Self {
        Self {
            implementation,
            max_size,
            flush_threshold,
        }
    }
    
    /// Insert an entry
    pub fn insert(&mut self, entry: Entry) -> Result<()> {
        self.implementation.insert(entry)
    }
    
    /// Get an entry by key
    pub fn get(&self, key: &Key) -> Result<Option<Entry>> {
        self.implementation.get(key)
    }
    
    /// Delete an entry by key
    pub fn delete(&mut self, key: &Key, sequence: u64) -> Result<()> {
        self.implementation.delete(key, sequence)
    }
    
    /// Get all entries in sorted order
    pub fn iter(&self) -> Box<dyn Iterator<Item = Entry> + '_> {
        self.implementation.iter()
    }
    
    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.implementation.len()
    }
    
    /// Check if memtable is empty
    pub fn is_empty(&self) -> bool {
        self.implementation.is_empty()
    }
    
    /// Get the memory usage
    pub fn memory_usage(&self) -> usize {
        self.implementation.memory_usage()
    }
    
    /// Check if memtable should be flushed
    pub fn should_flush(&self) -> bool {
        self.memory_usage() >= (self.max_size as f64 * self.flush_threshold) as usize
    }
    
    /// Check if memtable is full
    pub fn is_full(&self) -> bool {
        self.memory_usage() >= self.max_size
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.implementation.clear();
    }
    
    /// Take all entries and clear the memtable
    pub fn take_entries(&mut self) -> Vec<Entry> {
        let entries: Vec<Entry> = self.iter().collect();
        self.clear();
        entries
    }
}

/// Factory function to create memtables
pub fn create_memtable(
    implementation: crate::config::MemtableImpl,
    max_size: usize,
    flush_threshold: f64,
) -> Memtable {
    let impl_box: Box<dyn MemtableImpl> = match implementation {
        crate::config::MemtableImpl::SkipList => Box::new(SkipListMemtable::new()),
        crate::config::MemtableImpl::Art => Box::new(ArtMemtable::new()),
        crate::config::MemtableImpl::BTree => Box::new(BTreeMemtable::new()),
    };
    
    Memtable::new(impl_box, max_size, flush_threshold)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Entry, Key, Value, OpType};
    
    #[test]
    fn test_btree_memtable_insert_get() {
        let mut memtable = BTreeMemtable::new();
        let key = Key::new(b"test_key".to_vec());
        let value = Value::new(b"test_value".to_vec());
        let entry = Entry::new(key.clone(), value, 1);
        
        memtable.insert(entry.clone()).unwrap();
        let retrieved = memtable.get(&key).unwrap().unwrap();
        assert_eq!(retrieved.key, entry.key);
        assert_eq!(retrieved.value, entry.value);
    }
    
    #[test]
    fn test_btree_memtable_delete() {
        let mut memtable = BTreeMemtable::new();
        let key = Key::new(b"test_key".to_vec());
        let value = Value::new(b"test_value".to_vec());
        let entry = Entry::new(key.clone(), value, 1);
        
        memtable.insert(entry).unwrap();
        assert!(memtable.get(&key).unwrap().is_some());
        
        memtable.delete(&key, 2).unwrap();
        let retrieved = memtable.get(&key).unwrap().unwrap();
        assert!(retrieved.is_delete());
    }
    
    #[test]
    fn test_memtable_factory() {
        let memtable = create_memtable(
            crate::config::MemtableImpl::BTree,
            1024,
            0.8,
        );
        
        assert!(memtable.is_empty());
        assert_eq!(memtable.max_size, 1024);
        assert_eq!(memtable.flush_threshold, 0.8);
    }
}
