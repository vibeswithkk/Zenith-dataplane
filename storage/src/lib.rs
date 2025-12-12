/// Zenith Storage Layer
/// Provides persistent event storage using embedded database
use sled::{Db, Tree};
use serde::{Serialize, Deserialize};
use bincode::{Encode, Decode};
use anyhow::Result;
use std::path::Path;

/// Event storage record
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct StoredEvent {
    pub source_id: u32,
    pub seq_no: u64,
    pub timestamp_ns: u64,
    pub data: Vec<u8>,
}

/// Storage engine for Zenith events
pub struct StorageEngine {
    db: Db,
    events: Tree,
}

impl StorageEngine {
    /// Open or create storage at path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        let events = db.open_tree("events")?;
        
        Ok(Self { db, events })
    }
    
    /// Store an event
    pub fn store_event(&self, event: StoredEvent) -> Result<()> {
        let key = Self::make_key(event.source_id, event.seq_no);
        let config = bincode::config::standard();
        let value = bincode::encode_to_vec(&event, config)?;
        self.events.insert(key, value)?;
        Ok(())
    }
    
    /// Retrieve an event
    pub fn get_event(&self, source_id: u32, seq_no: u64) -> Result<Option<StoredEvent>> {
        let key = Self::make_key(source_id, seq_no);
        match self.events.get(key)? {
            Some(data) => {
                let config = bincode::config::standard();
                let (event, _): (StoredEvent, _) = bincode::decode_from_slice(&data, config)?;
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }
    
    /// Get all events for a source
    pub fn get_source_events(&self, source_id: u32) -> Result<Vec<StoredEvent>> {
        let prefix = source_id.to_be_bytes();
        let mut events = Vec::new();
        let config = bincode::config::standard();
        
        for item in self.events.scan_prefix(prefix) {
            let (_key, value) = item?;
            let (event, _): (StoredEvent, _) = bincode::decode_from_slice(&value, config)?;
            events.push(event);
        }
        
        Ok(events)
    }
    
    /// Count total events
    pub fn count_events(&self) -> usize {
        self.events.len()
    }
    
    /// Delete an event
    pub fn delete_event(&self, source_id: u32, seq_no: u64) -> Result<bool> {
        let key = Self::make_key(source_id, seq_no);
        Ok(self.events.remove(key)?.is_some())
    }
    
    /// Flush to disk
    pub fn flush(&self) -> Result<usize> {
        Ok(self.db.flush()?)
    }
    
    /// Clear all events
    pub fn clear(&self) -> Result<()> {
        self.events.clear()?;
        Ok(())
    }
    
    // Helper: create composite key
    fn make_key(source_id: u32, seq_no: u64) -> [u8; 12] {
        let mut key = [0u8; 12];
        key[0..4].copy_from_slice(&source_id.to_be_bytes());
        key[4..12].copy_from_slice(&seq_no.to_be_bytes());
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_crud() {
        let dir = tempdir().unwrap();
        let storage = StorageEngine::open(dir.path()).unwrap();
        
        // Store
        let event = StoredEvent {
            source_id: 1,
            seq_no: 100,
            timestamp_ns: 123456789,
            data: vec![1, 2, 3, 4],
        };
        storage.store_event(event.clone()).unwrap();
        
        // Retrieve
        let retrieved = storage.get_event(1, 100).unwrap().unwrap();
        assert_eq!(retrieved.source_id, 1);
        assert_eq!(retrieved.seq_no, 100);
        assert_eq!(retrieved.data, vec![1, 2, 3, 4]);
        
        // Count
        assert_eq!(storage.count_events(), 1);
        
        // Delete
        assert!(storage.delete_event(1, 100).unwrap());
        assert_eq!(storage.count_events(), 0);
    }

    #[test]
    fn test_source_scan() {
        let dir = tempdir().unwrap();
        let storage = StorageEngine::open(dir.path()).unwrap();
        
        // Store multiple events for same source
        for i in 0..5 {
            storage.store_event(StoredEvent {
                source_id: 1,
                seq_no: i,
                timestamp_ns: i * 1000,
                data: vec![i as u8],
            }).unwrap();
        }
        
        // Store events for different source
        storage.store_event(StoredEvent {
            source_id: 2,
            seq_no: 0,
            timestamp_ns: 0,
            data: vec![99],
        }).unwrap();
        
        // Scan source 1
        let events = storage.get_source_events(1).unwrap();
        assert_eq!(events.len(), 5);
        
        // Verify ordering
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.seq_no, i as u64);
        }
    }
}
