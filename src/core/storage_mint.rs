use serde::{Serialize, Deserialize};
use crate::core::storage_base::{StorageBase, StorageError, StorageBackend};
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MintRecord {
    pub signature: String,
    pub memo_json: String,
    pub timestamp: f64,
    pub id: String,
}

/// Mint record storage manager
pub struct MintStorage {
    base: StorageBase<MintRecord>,
}

impl MintStorage {
    /// Create new Mint storage instance
    pub fn new() -> Self {
        Self {
            base: StorageBase::new(
                "memo-mint-records.dat".to_string(),
                "memo_mint_records".to_string(),
                100
            ),
        }
    }

    /// Save mint record (async version)
    pub async fn save_mint_record_async(&self, signature: &str, memo_json: &str) -> Result<(), StorageError> {
        let record = MintRecord {
            signature: signature.to_string(),
            memo_json: memo_json.to_string(),
            timestamp: js_sys::Date::now(),
            id: signature.to_string(),
        };
        
        self.base.save_record(record, |r| r.signature.clone()).await
    }

    /// Save mint record (sync interface, internal async handling)
    pub fn save_mint_record(&self, signature: &str, memo_json: &str) -> Result<(), StorageError> {
        let signature = signature.to_string();
        let memo_json = memo_json.to_string();
        let base = self.base.clone();
        
        spawn_local(async move {
            let record = MintRecord {
                signature: signature.clone(),
                memo_json,
                timestamp: js_sys::Date::now(),
                id: signature,
            };
            
            if let Err(e) = base.save_record(record, |r| r.signature.clone()).await {
                log::error!("Failed to save mint record: {}", e);
            } else {
                log::info!("Mint record saved successfully");
            }
        });
        
        Ok(())
    }

    /// Get storage status information (sync version, for UI)
    pub fn get_storage_status(&self) -> Result<String, StorageError> {
        match self.base.get_backend() {
            StorageBackend::TauriStore => {
                Ok("Tauri Store: Ready".to_string())
            }
            StorageBackend::LocalStorage => {
                Ok("LocalStorage: Ready".to_string())
            }
        }
    }

    /// Async get detailed storage status
    pub async fn get_detailed_storage_status(&self) -> Result<String, StorageError> {
        let (count, max, total) = self.base.get_storage_info().await?;
        let usage_bytes = self.base.get_storage_usage_bytes().await?;
        let usage_kb = usage_bytes as f64 / 1024.0;
        let backend_info = self.base.get_environment_info();
        
        Ok(format!(
            "Mint Records: {}/{} (Total: {}), Storage: {:.1}KB [{}]",
            count, max, total, usage_kb, backend_info
        ))
    }

    /// Async get all records
    pub async fn get_all_records(&self) -> Result<Vec<MintRecord>, StorageError> {
        let mut records = self.base.get_all_records().await?;
        records.sort_by(|a, b| b.timestamp.partial_cmp(&a.timestamp).unwrap_or(std::cmp::Ordering::Equal));
        Ok(records)
    }

    /// Check if near capacity limit
    pub async fn is_near_capacity(&self, threshold_percent: usize) -> Result<bool, StorageError> {
        let (count, max, _) = self.base.get_storage_info().await?;
        let usage_percent = (count * 100) / max;
        Ok(usage_percent >= threshold_percent)
    }

    /// Get record by signature
    pub async fn get_record_by_signature(&self, signature: &str) -> Result<Option<MintRecord>, StorageError> {
        self.base.get_record_by_id(signature, |r| r.signature.clone()).await
    }

    /// Delete record
    pub async fn delete_record(&self, signature: &str) -> Result<(), StorageError> {
        self.base.delete_record(signature, |r| r.signature.clone()).await
    }

    /// Get record count
    pub async fn get_record_count(&self) -> Result<usize, StorageError> {
        let (count, _, _) = self.base.get_storage_info().await?;
        Ok(count)
    }

    /// Clear all records
    pub async fn clear_all_records(&self) -> Result<(), StorageError> {
        self.base.clear_all_records().await
    }

    /// Get current used storage backend
    pub fn get_backend(&self) -> StorageBackend {
        self.base.get_backend()
    }
}

// Global instance
static mut MINT_STORAGE_INSTANCE: Option<MintStorage> = None;
static MINT_INIT: std::sync::Once = std::sync::Once::new();

/// Get global Mint storage instance
pub fn get_mint_storage() -> &'static MintStorage {
    unsafe {
        MINT_INIT.call_once(|| {
            MINT_STORAGE_INSTANCE = Some(MintStorage::new());
        });
        MINT_STORAGE_INSTANCE.as_ref().unwrap()
    }
}