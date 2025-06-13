use serde::{Serialize, Deserialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::*;
use web_sys::{window, Storage};

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
    LocalStorageError(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::TauriError(msg) => write!(f, "Tauri Store error: {}", msg),
            StorageError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            StorageError::NotFound => write!(f, "Record not found"),
            StorageError::NotSupported => write!(f, "Storage not supported"),
            StorageError::LocalStorageError(msg) => write!(f, "LocalStorage error: {}", msg),
        }
    }
}

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageBackend {
    TauriStore,
    LocalStorage,
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

/// Generic adaptive storage base class
#[derive(Clone)]
pub struct StorageBase<T> {
    pub store_filename: String,
    pub storage_key: String,
    pub max_records: usize,
    backend: StorageBackend,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> StorageBase<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// Create new storage instance, automatically detect environment
    pub fn new(store_filename: String, storage_key: String, max_records: usize) -> Self {
        let backend = Self::detect_storage_backend();
        log::info!("Storage backend detected: {:?}", backend);
        
        Self {
            store_filename,
            storage_key,
            max_records,
            backend,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Detect storage backend
    fn detect_storage_backend() -> StorageBackend {
        if Self::is_tauri_available() {
            StorageBackend::TauriStore
        } else {
            StorageBackend::LocalStorage
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

    /// Get current used storage backend
    pub fn get_backend(&self) -> StorageBackend {
        self.backend
    }

    /// Get storage data structure (uniform interface)
    pub async fn get_storage_data(&self) -> Result<CircularStorageData<T>, StorageError> {
        match self.backend {
            StorageBackend::TauriStore => self.get_storage_data_tauri().await,
            StorageBackend::LocalStorage => self.get_storage_data_localstorage().await,
        }
    }

    /// Save storage data structure (uniform interface)
    pub async fn save_storage_data(&self, data: &CircularStorageData<T>) -> Result<(), StorageError> {
        match self.backend {
            StorageBackend::TauriStore => self.save_storage_data_tauri(data).await,
            StorageBackend::LocalStorage => self.save_storage_data_localstorage(data).await,
        }
    }

    // ============ Tauri Store implementation ============

    /// Get Tauri Store instance
    async fn get_store(&self) -> Result<TauriStore, StorageError> {
        let store_js = tauri_store_load(&self.store_filename).await;
        
        if store_js.is_undefined() || store_js.is_null() {
            return Err(StorageError::TauriError("Failed to load store".to_string()));
        }

        Ok(store_js.unchecked_into::<TauriStore>())
    }

    /// Tauri Store version of get data
    async fn get_storage_data_tauri(&self) -> Result<CircularStorageData<T>, StorageError> {
        let store = self.get_store().await?;
        let js_value = store.get(&self.storage_key).await;

        if js_value.is_null() || js_value.is_undefined() {
            Ok(CircularStorageData {
                records: vec![None; self.max_records],
                next_index: 0,
                total_count: 0,
            })
        } else {
            serde_wasm_bindgen::from_value(js_value)
                .map_err(|e| StorageError::SerializationError(e.to_string()))
        }
    }

    /// Tauri Store version of save data
    async fn save_storage_data_tauri(&self, data: &CircularStorageData<T>) -> Result<(), StorageError> {
        let store = self.get_store().await?;
        
        let js_value = serde_wasm_bindgen::to_value(data)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        store.set(&self.storage_key, &js_value).await;
        store.save().await;
        
        Ok(())
    }

    // ============ LocalStorage implementation ============

    /// Get localStorage instance
    fn get_local_storage(&self) -> Result<Storage, StorageError> {
        let window = window().ok_or(StorageError::LocalStorageError("Window not available".to_string()))?;
        window.local_storage()
            .map_err(|_| StorageError::LocalStorageError("localStorage access denied".to_string()))?
            .ok_or(StorageError::LocalStorageError("localStorage not available".to_string()))
    }

    /// LocalStorage version of get data
    async fn get_storage_data_localstorage(&self) -> Result<CircularStorageData<T>, StorageError> {
        let storage = self.get_local_storage()?;
        
        match storage.get_item(&self.storage_key) {
            Ok(Some(data)) => {
                log::debug!("Found existing localStorage data for key: {}", self.storage_key);
                serde_json::from_str(&data)
                    .map_err(|e| {
                        log::error!("Failed to deserialize localStorage data: {}", e);
                        StorageError::SerializationError(format!("Deserialization failed: {}", e))
                    })
            }
            Ok(None) => {
                log::debug!("No existing localStorage data found for key: {}, creating new", self.storage_key);
                Ok(CircularStorageData {
                    records: vec![None; self.max_records],
                    next_index: 0,
                    total_count: 0,
                })
            }
            Err(e) => {
                log::error!("Failed to access localStorage for key {}: {:?}", self.storage_key, e);
                Err(StorageError::LocalStorageError("Failed to read from localStorage".to_string()))
            }
        }
    }

    /// LocalStorage version of save data
    async fn save_storage_data_localstorage(&self, data: &CircularStorageData<T>) -> Result<(), StorageError> {
        let storage = self.get_local_storage()?;
        let serialized = serde_json::to_string(data)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        storage.set_item(&self.storage_key, &serialized)
            .map_err(|_| StorageError::LocalStorageError("Failed to write to localStorage".to_string()))?;
        
        Ok(())
    }

    // ============ Unified public interface ============

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
                log::debug!("Overwrote old record at position {} with ID: {} using {:?}", 
                    (data.next_index + self.max_records - 1) % self.max_records, record_id, self.backend);
            } else {
                log::debug!("Added new record at position {} with ID: {} using {:?}", 
                    (data.next_index + self.max_records - 1) % self.max_records, record_id, self.backend);
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
        match self.backend {
            StorageBackend::TauriStore => {
                let store = self.get_store().await?;
                store.delete(&self.storage_key).await;
                store.save().await;
            }
            StorageBackend::LocalStorage => {
                let storage = self.get_local_storage()?;
                storage.remove_item(&self.storage_key)
                    .map_err(|_| StorageError::LocalStorageError("Failed to clear localStorage".to_string()))?;
            }
        }
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

    /// Get environment description
    pub fn get_environment_info(&self) -> String {
        match self.backend {
            StorageBackend::TauriStore => "Local Storage".to_string(),
            StorageBackend::LocalStorage => "Local Storage".to_string(),
        }
    }
} 