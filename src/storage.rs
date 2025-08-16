use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A key in the storage engine
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key {
    /// The actual key bytes
    pub data: Vec<u8>,
    /// Optional user-defined metadata
    pub metadata: Option<Vec<u8>>,
}

impl Key {
    /// Create a new key from bytes
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            metadata: None,
        }
    }

    /// Create a key with metadata
    pub fn with_metadata(data: Vec<u8>, metadata: Vec<u8>) -> Self {
        Self {
            data,
            metadata: Some(metadata),
        }
    }

    /// Get the key as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get the key length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if key is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data.cmp(&other.data)
    }
}

impl From<Vec<u8>> for Key {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl From<&[u8]> for Key {
    fn from(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Self::new(s.into_bytes())
    }
}

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Self::new(s.as_bytes().to_vec())
    }
}

/// A value in the storage engine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    /// The actual value bytes
    pub data: Vec<u8>,
    /// Optional compression info
    pub compressed: bool,
    /// Optional checksum
    pub checksum: Option<u32>,
}

impl Value {
    /// Create a new value from bytes
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            compressed: false,
            checksum: None,
        }
    }

    /// Create a compressed value
    pub fn compressed(data: Vec<u8>, checksum: u32) -> Self {
        Self {
            data,
            compressed: true,
            checksum: Some(checksum),
        }
    }

    /// Get the value as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get the value length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if value is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Check if this is a "large value" that should go to value log
    pub fn is_large(&self, threshold: usize) -> bool {
        self.data.len() >= threshold
    }
}

impl From<Vec<u8>> for Value {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl From<&[u8]> for Value {
    fn from(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::new(s.into_bytes())
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::new(s.as_bytes().to_vec())
    }
}

/// A value pointer for WAL-time KV separation
/// Points to a location in the value log where the actual value is stored
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValuePointer {
    /// Segment ID in the value log
    pub segment_id: u64,
    /// Offset within the segment
    pub offset: u64,
    /// Length of the value
    pub length: u32,
    /// Optional checksum for validation
    pub checksum: Option<u32>,
}

impl ValuePointer {
    /// Create a new value pointer
    pub fn new(segment_id: u64, offset: u64, length: u32) -> Self {
        Self {
            segment_id,
            offset,
            length,
            checksum: None,
        }
    }

    /// Create a value pointer with checksum
    pub fn with_checksum(segment_id: u64, offset: u64, length: u32, checksum: u32) -> Self {
        Self {
            segment_id,
            offset,
            length,
            checksum: Some(checksum),
        }
    }

    /// Get the end offset (offset + length)
    pub fn end_offset(&self) -> u64 {
        self.offset + self.length as u64
    }

    /// Check if this pointer is valid
    pub fn is_valid(&self) -> bool {
        self.segment_id > 0 && self.offset > 0 && self.length > 0
    }
}

/// A key-value pair entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// The key
    pub key: Key,
    /// The value (if not using value log) or value pointer
    pub value: Option<Value>,
    /// The value pointer (if using value log)
    pub value_pointer: Option<ValuePointer>,
    /// Sequence number for MVCC
    pub sequence: u64,
    /// Operation type
    pub op_type: OpType,
    /// Timestamp
    pub timestamp: u64,
}

impl Entry {
    /// Create a new entry with an inline value
    pub fn new(key: Key, value: Value, sequence: u64) -> Self {
        Self {
            key,
            value: Some(value),
            value_pointer: None,
            sequence,
            op_type: OpType::Put,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Create a new entry with a value pointer
    pub fn with_pointer(key: Key, value_pointer: ValuePointer, sequence: u64) -> Self {
        Self {
            key,
            value: None,
            value_pointer: Some(value_pointer),
            sequence,
            op_type: OpType::Put,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Create a delete entry
    pub fn delete(key: Key, sequence: u64) -> Self {
        Self {
            key,
            value: None,
            value_pointer: None,
            sequence,
            op_type: OpType::Delete,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Check if this entry has an inline value
    pub fn has_inline_value(&self) -> bool {
        self.value.is_some()
    }

    /// Check if this entry has a value pointer
    pub fn has_value_pointer(&self) -> bool {
        self.value_pointer.is_some()
    }

    /// Check if this is a delete operation
    pub fn is_delete(&self) -> bool {
        matches!(self.op_type, OpType::Delete)
    }
}

/// Operation types for entries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpType {
    /// Put operation
    Put,
    /// Delete operation
    Delete,
    /// Merge operation
    Merge,
}

/// A batch of operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    /// The operations in this batch
    pub operations: Vec<Entry>,
    /// Batch sequence number
    pub sequence: u64,
    /// Whether this batch should be synced
    pub sync: bool,
}

impl Batch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            sequence: 0,
            sync: false,
        }
    }

    /// Add an operation to the batch
    pub fn add(&mut self, operation: Entry) {
        self.operations.push(operation);
    }

    /// Set the batch sequence number
    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }

    /// Set whether this batch should be synced
    pub fn with_sync(mut self, sync: bool) -> Self {
        self.sync = sync;
        self
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Get the number of operations
    pub fn len(&self) -> usize {
        self.operations.len()
    }
}

impl Default for Batch {
    fn default() -> Self {
        Self::new()
    }
}

/// A range for scan operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    /// Start key (inclusive)
    pub start: Key,
    /// End key (exclusive)
    pub end: Key,
    /// Maximum number of entries to return
    pub limit: Option<usize>,
}

impl Range {
    /// Create a new range
    pub fn new(start: Key, end: Key) -> Self {
        Self {
            start,
            end,
            limit: None,
        }
    }

    /// Set a limit on the number of entries
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}
