use serde::{Serialize, Deserialize};
use crate::core::storage_base::{StorageBase, StorageError, StorageBackend};
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BurnRecord {
    pub signature: String,
    pub memo_json: String,
    pub amount: u64,
    pub timestamp: f64,
    pub id: String,
}

/// Burn record storage manager
pub struct BurnStorage {
    base: StorageBase<BurnRecord>,
}

impl BurnStorage {
    /// Create a new Burn storage instance
    pub fn new() -> Self {
        Self {
            base: StorageBase::new(
                "memo-burn-records.dat".to_string(),
                "memo_burn_records".to_string(),
                100
            ),
        }
    }

    /// Save burn record (async version)
    pub async fn save_burn_record_async(&self, signature: &str, memo_json: &str, amount: u64) -> Result<(), StorageError> {
        let record = BurnRecord {
            signature: signature.to_string(),
            memo_json: memo_json.to_string(),
            amount,
            timestamp: js_sys::Date::now(),
            id: signature.to_string(),
        };
        
        self.base.save_record(record, |r| r.signature.clone()).await
    }

    /// Save burn record (sync interface, internal async handling)
    pub fn save_burn_record(&self, signature: &str, memo_json: &str, amount: u64) -> Result<(), StorageError> {
        let signature = signature.to_string();
        let memo_json = memo_json.to_string();
        let base = self.base.clone();
        
        spawn_local(async move {
            let record = BurnRecord {
                signature: signature.clone(),
                memo_json,
                amount,
                timestamp: js_sys::Date::now(),
                id: signature,
            };
            
            if let Err(e) = base.save_record(record, |r| r.signature.clone()).await {
                log::error!("Failed to save burn record: {}", e);
            } else {
                log::info!("Burn record saved successfully");
            }
        });
        
        Ok(())
    }

    /// Get storage status information (sync version, for UI)
    pub fn get_storage_status(&self) -> Result<String, StorageError> {
        Ok("Storage: Ready".to_string())
    }

    /// Async get detailed storage status
    pub async fn get_detailed_storage_status(&self) -> Result<String, StorageError> {
        let (count, max, total) = self.base.get_storage_info().await?;
        let usage_bytes = self.base.get_storage_usage_bytes().await?;
        let usage_kb = usage_bytes as f64 / 1024.0;
        
        Ok(format!(
            "Burn Records: {}/{} (Total: {}), Storage: {:.1}KB",
            count, max, total, usage_kb
        ))
    }

    /// Async get all records (sorted by timestamp)
    pub async fn get_all_records(&self) -> Result<Vec<BurnRecord>, StorageError> {
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
    pub async fn get_record_by_signature(&self, signature: &str) -> Result<Option<BurnRecord>, StorageError> {
        self.base.get_record_by_id(signature, |r| r.signature.clone()).await
    }

    /// Delete record
    pub async fn delete_record(&self, signature: &str) -> Result<(), StorageError> {
        self.base.delete_record(signature, |r| r.signature.clone()).await?;
        log::info!("Deleted burn record for signature: {}", signature);
        Ok(())
    }

    /// Get record count
    pub async fn get_record_count(&self) -> Result<usize, StorageError> {
        let (count, _, _) = self.base.get_storage_info().await?;
        Ok(count)
    }

    /// Clear all records
    pub async fn clear_all_records(&self) -> Result<(), StorageError> {
        self.base.clear_all_records().await?;
        log::info!("Cleared all burn records");
        Ok(())
    }

    /// Get recent N records
    pub async fn get_recent_records(&self, count: usize) -> Result<Vec<BurnRecord>, StorageError> {
        let mut records = self.get_all_records().await?;
        records.truncate(count);
        Ok(records)
    }

    /// Get records by time range
    pub async fn get_records_by_time_range(&self, start_time: f64, end_time: f64) -> Result<Vec<BurnRecord>, StorageError> {
        let records = self.base.get_all_records().await?;
        let filtered: Vec<BurnRecord> = records
            .into_iter()
            .filter(|r| r.timestamp >= start_time && r.timestamp <= end_time)
            .collect();
        Ok(filtered)
    }

    /// Get records by amount range
    pub async fn get_records_by_amount_range(&self, min_amount: u64, max_amount: u64) -> Result<Vec<BurnRecord>, StorageError> {
        let records = self.base.get_all_records().await?;
        let filtered: Vec<BurnRecord> = records
            .into_iter()
            .filter(|r| r.amount >= min_amount && r.amount <= max_amount)
            .collect();
        Ok(filtered)
    }

    /// Calculate total burned amount
    pub async fn get_total_burned_amount(&self) -> Result<u64, StorageError> {
        let records = self.base.get_all_records().await?;
        let total = records.iter().map(|r| r.amount).sum();
        Ok(total)
    }

    /// Get storage statistics
    pub async fn get_statistics(&self) -> Result<BurnStorageStats, StorageError> {
        let (count, max, total) = self.base.get_storage_info().await?;
        let usage_bytes = self.base.get_storage_usage_bytes().await?;
        let avg_record_size = if count > 0 { usage_bytes / count } else { 0 };
        let total_burned = self.get_total_burned_amount().await?;
        
        Ok(BurnStorageStats {
            current_count: count,
            max_capacity: max,
            total_writes: total,
            storage_bytes: usage_bytes,
            avg_record_size,
            is_full: self.base.is_full().await?,
            total_burned_amount: total_burned,
        })
    }

    /// Get current used storage backend
    pub fn get_backend(&self) -> StorageBackend {
        self.base.get_backend()
    }
}

/// Burn storage statistics
#[derive(Debug, Clone)]
pub struct BurnStorageStats {
    pub current_count: usize,
    pub max_capacity: usize,
    pub total_writes: usize,
    pub storage_bytes: usize,
    pub avg_record_size: usize,
    pub is_full: bool,
    pub total_burned_amount: u64,
}

// Global instance
static mut BURN_STORAGE_INSTANCE: Option<BurnStorage> = None;
static BURN_INIT: std::sync::Once = std::sync::Once::new();

/// Get global Burn storage instance
pub fn get_burn_storage() -> &'static BurnStorage {
    unsafe {
        BURN_INIT.call_once(|| {
            BURN_STORAGE_INSTANCE = Some(BurnStorage::new());
        });
        BURN_STORAGE_INSTANCE.as_ref().unwrap()
    }
} 