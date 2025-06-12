use serde::{Serialize, Deserialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CircularStorageData<T> {
    pub records: Vec<Option<T>>,
    pub next_index: usize,
    pub total_count: usize,
}

#[derive(Debug)]
pub enum StorageError {
    TauriError(String),
    SerializationError(String),
    NotFound,
    NotSupported,
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::TauriError(msg) => write!(f, "Tauri Store error: {}", msg),
            StorageError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            StorageError::NotFound => write!(f, "Record not found"),
            StorageError::NotSupported => write!(f, "Storage not supported"),
        }
    }
}

// Tauri Store JavaScript API binding
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "store"], js_name = "load")]
    async fn tauri_store_load(path: &str) -> JsValue;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "store"], js_name = "Store")]
    type TauriStore;

    #[wasm_bindgen(method, js_name = "get")]
    async fn get(this: &TauriStore, key: &str) -> JsValue;

    #[wasm_bindgen(method, js_name = "set")]
    async fn set(this: &TauriStore, key: &str, value: &JsValue) -> JsValue;

    #[wasm_bindgen(method, js_name = "save")]
    async fn save(this: &TauriStore) -> JsValue;

    #[wasm_bindgen(method, js_name = "has")]
    async fn has(this: &TauriStore, key: &str) -> JsValue;

    #[wasm_bindgen(method, js_name = "delete")]
    async fn delete(this: &TauriStore, key: &str) -> JsValue;
}

/// Generic Tauri Store storage base class
#[derive(Clone)]
pub struct StorageBase<T> {
    pub store_filename: String,
    pub storage_key: String,
    pub max_records: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> StorageBase<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// Create a new storage instance
    pub fn new(store_filename: String, storage_key: String, max_records: usize) -> Self {
        Self {
            store_filename,
            storage_key,
            max_records,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Check if running in Tauri environment
    pub fn is_tauri_available() -> bool {
        let window = web_sys::window();
        if let Some(window) = window {
            let tauri_obj = js_sys::Reflect::get(&window, &"__TAURI__".into());
            !tauri_obj.is_err() && !tauri_obj.unwrap().is_undefined()
        } else {
            false
        }
    }

    /// Get Tauri Store instance
    async fn get_store(&self) -> Result<TauriStore, StorageError> {
        if !Self::is_tauri_available() {
            return Err(StorageError::NotSupported);
        }

        let store_js = tauri_store_load(&self.store_filename).await;
        
        if store_js.is_undefined() || store_js.is_null() {
            return Err(StorageError::TauriError("Failed to load store".to_string()));
        }

        Ok(store_js.unchecked_into::<TauriStore>())
    }

    /// Get storage data structure
    pub async fn get_storage_data(&self) -> Result<CircularStorageData<T>, StorageError> {
        let store = self.get_store().await?;
        let js_value = store.get(&self.storage_key).await;

        if js_value.is_null() || js_value.is_undefined() {
            // Create new storage data
            Ok(CircularStorageData {
                records: vec![None; self.max_records],
                next_index: 0,
                total_count: 0,
            })
        } else {
            // Parse existing data
            serde_wasm_bindgen::from_value(js_value)
                .map_err(|e| StorageError::SerializationError(e.to_string()))
        }
    }

    /// Save storage data structure
    pub async fn save_storage_data(&self, data: &CircularStorageData<T>) -> Result<(), StorageError> {
        let store = self.get_store().await?;
        
        let js_value = serde_wasm_bindgen::to_value(data)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        store.set(&self.storage_key, &js_value).await;
        store.save().await;
        
        Ok(())
    }

    /// Generic save record method
    pub async fn save_record<F>(&self, record: T, get_id: F) -> Result<(), StorageError>
    where
        F: Fn(&T) -> String,
    {
        let mut data = self.get_storage_data().await?;
        let record_id = get_id(&record);
        
        // Check if record with same ID already exists
        let existing_index = data.records.iter().position(|r| {
            r.as_ref().map_or(false, |rec| get_id(rec) == record_id)
        });
        
        if let Some(index) = existing_index {
            // If exists, update the position
            data.records[index] = Some(record);
            log::debug!("Updated existing record with ID: {}", record_id);
        } else {
            // New record, use circular queue logic
            let was_overwriting = data.records[data.next_index].is_some();
            data.records[data.next_index] = Some(record);
            data.next_index = (data.next_index + 1) % self.max_records;
            data.total_count += 1;
            
            if was_overwriting {
                log::debug!("Overwrote old record at position {} with ID: {}", 
                    (data.next_index + self.max_records - 1) % self.max_records, record_id);
            } else {
                log::debug!("Added new record at position {} with ID: {}", 
                    (data.next_index + self.max_records - 1) % self.max_records, record_id);
            }
        }
        
        self.save_storage_data(&data).await?;
        Ok(())
    }

    /// Get all records
    pub async fn get_all_records(&self) -> Result<Vec<T>, StorageError> {
        let data = self.get_storage_data().await?;
        let records: Vec<T> = data.records
            .into_iter()
            .filter_map(|r| r)
            .collect();
        Ok(records)
    }

    /// Get record by ID
    pub async fn get_record_by_id<F>(&self, id: &str, get_id: F) -> Result<Option<T>, StorageError>
    where
        F: Fn(&T) -> String,
    {
        let data = self.get_storage_data().await?;
        
        for record_opt in &data.records {
            if let Some(record) = record_opt {
                if get_id(record) == id {
                    return Ok(Some(record.clone()));
                }
            }
        }
        
        Ok(None)
    }

    /// Delete record
    pub async fn delete_record<F>(&self, id: &str, get_id: F) -> Result<(), StorageError>
    where
        F: Fn(&T) -> String,
    {
        let mut data = self.get_storage_data().await?;
        
        for record_opt in &mut data.records {
            if let Some(record) = record_opt {
                if get_id(record) == id {
                    *record_opt = None;
                    self.save_storage_data(&data).await?;
                    return Ok(());
                }
            }
        }
        
        Err(StorageError::NotFound)
    }

    /// Get storage statistics
    pub async fn get_storage_info(&self) -> Result<(usize, usize, usize), StorageError> {
        let data = self.get_storage_data().await?;
        let record_count = data.records.iter().filter(|r| r.is_some()).count();
        Ok((record_count, self.max_records, data.total_count))
    }

    /// Get storage usage bytes (estimated)
    pub async fn get_storage_usage_bytes(&self) -> Result<usize, StorageError> {
        let data = self.get_storage_data().await?;
        
        // Estimated size after serialization
        let serialized = serde_json::to_string(&data)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        Ok(serialized.len())
    }

    /// Clear all records
    pub async fn clear_all_records(&self) -> Result<(), StorageError> {
        let store = self.get_store().await?;
        store.delete(&self.storage_key).await;
        store.save().await;
        Ok(())
    }

    /// Check if storage is full
    pub async fn is_full(&self) -> Result<bool, StorageError> {
        let (count, max, _) = self.get_storage_info().await?;
        Ok(count >= max)
    }

    /// Get storage file name
    pub fn get_store_filename(&self) -> &str {
        &self.store_filename
    }

    /// Get storage key
    pub fn get_storage_key(&self) -> &str {
        &self.storage_key
    }

    /// Get maximum number of records
    pub fn get_max_records(&self) -> usize {
        self.max_records
    }
} 