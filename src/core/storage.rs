use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use web_sys::{window, Storage};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MintRecord {
    pub signature: String,
    pub memo_json: String,
    pub timestamp: f64,
    pub id: String, // use signature as ID
}

#[derive(Debug)]
pub enum StorageError {
    StorageNotAvailable,
    SerializationError(String),
    NotFound,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::StorageNotAvailable => write!(f, "Local storage not available"),
            StorageError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            StorageError::NotFound => write!(f, "Record not found"),
        }
    }
}

pub struct MintStorage {
    storage_key: String,
}

impl MintStorage {
    pub fn new() -> Self {
        Self {
            storage_key: "memo_mint_records".to_string(),
        }
    }

    // get localStorage
    fn get_local_storage(&self) -> Result<Storage, StorageError> {
        let window = window().ok_or(StorageError::StorageNotAvailable)?;
        window.local_storage()
            .map_err(|_| StorageError::StorageNotAvailable)?
            .ok_or(StorageError::StorageNotAvailable)
    }

    // get all records
    fn get_all_records_map(&self) -> Result<HashMap<String, MintRecord>, StorageError> {
        let storage = self.get_local_storage()?;
        
        match storage.get_item(&self.storage_key) {
            Ok(Some(data)) => {
                serde_json::from_str(&data)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))
            }
            Ok(None) => Ok(HashMap::new()),
            Err(_) => Err(StorageError::StorageNotAvailable),
        }
    }

    // save all records
    fn save_all_records_map(&self, records: &HashMap<String, MintRecord>) -> Result<(), StorageError> {
        let storage = self.get_local_storage()?;
        let serialized = serde_json::to_string(records)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        storage.set_item(&self.storage_key, &serialized)
            .map_err(|_| StorageError::StorageNotAvailable)?;
        
        Ok(())
    }

    // save mint record
    pub fn save_mint_record(&self, signature: &str, memo_json: &str) -> Result<(), StorageError> {
        let mut records = self.get_all_records_map()?;
        
        let record = MintRecord {
            signature: signature.to_string(),
            memo_json: memo_json.to_string(),
            timestamp: js_sys::Date::now(),
            id: signature.to_string(),
        };
        
        records.insert(signature.to_string(), record);
        self.save_all_records_map(&records)?;
        
        log::info!("Saved mint record for signature: {}", signature);
        Ok(())
    }

    // get all records (sorted by timestamp)
    pub fn get_all_records(&self) -> Result<Vec<MintRecord>, StorageError> {
        let records_map = self.get_all_records_map()?;
        let mut records: Vec<MintRecord> = records_map.into_iter().map(|(_, record)| record).collect();
        
        // sort by timestamp
        records.sort_by(|a, b| b.timestamp.partial_cmp(&a.timestamp).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(records)
    }

    // get record by signature
    pub fn get_record_by_signature(&self, signature: &str) -> Result<Option<MintRecord>, StorageError> {
        let records = self.get_all_records_map()?;
        Ok(records.get(signature).cloned())
    }

    // delete record
    pub fn delete_record(&self, signature: &str) -> Result<(), StorageError> {
        let mut records = self.get_all_records_map()?;
        
        if records.remove(signature).is_some() {
            self.save_all_records_map(&records)?;
            log::info!("Deleted mint record for signature: {}", signature);
            Ok(())
        } else {
            Err(StorageError::NotFound)
        }
    }

    // get record count
    pub fn get_record_count(&self) -> Result<usize, StorageError> {
        let records = self.get_all_records_map()?;
        Ok(records.len())
    }

    // clear all records
    pub fn clear_all_records(&self) -> Result<(), StorageError> {
        let storage = self.get_local_storage()?;
        storage.remove_item(&self.storage_key)
            .map_err(|_| StorageError::StorageNotAvailable)?;
        
        log::info!("Cleared all mint records");
        Ok(())
    }
}

// global storage instance
static mut STORAGE_INSTANCE: Option<MintStorage> = None;
static INIT: std::sync::Once = std::sync::Once::new();

pub fn get_storage() -> &'static MintStorage {
    unsafe {
        INIT.call_once(|| {
            STORAGE_INSTANCE = Some(MintStorage::new());
        });
        STORAGE_INSTANCE.as_ref().unwrap()
    }
} 