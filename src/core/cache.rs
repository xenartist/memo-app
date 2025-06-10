use serde::{Serialize, Deserialize};
use web_sys::js_sys::Date;
use crate::core::rpc_base::RpcConnection;
use solana_sdk::pubkey::Pubkey;
use serde_json;
use base64;
use log;
use std::sync::Mutex;
use std::sync::Arc;

// Public burn record structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BurnRecord {
    pub pubkey: String,      // wallet address
    pub signature: String,   // transaction signature (base58 encoded)
    pub slot: u64,          // slot
    pub blocktime: i64,     // blocktime
    pub amount: u64,        // amount of tokens burned (lamports)
}

// Latest burn shard cache structure
#[derive(Clone, Debug)]
pub struct LatestBurnShardCache {
    pub data: Vec<BurnRecord>,
    pub last_updated: f64,     // cache last updated time
    pub cache_ttl: u64,        // TTL in milliseconds (default 10 minutes)
}

impl Default for LatestBurnShardCache {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            last_updated: 0.0,
            cache_ttl: 10 * 60 * 1000, // 10 minutes
        }
    }
}

impl LatestBurnShardCache {
    pub fn is_expired(&self) -> bool {
        let current_time = Date::now();
        (current_time - self.last_updated) > self.cache_ttl as f64
    }

    pub fn update_data(&mut self, data: Vec<BurnRecord>) {
        self.data = data;
        self.last_updated = Date::now();
    }

    pub fn clear_data(&mut self) {
        self.data.clear();
        self.last_updated = 0.0;
    }

    pub fn set_ttl(&mut self, ttl_ms: u64) {
        self.cache_ttl = ttl_ms;
    }
}

// Global cache manager
pub struct CacheManager {
    latest_burn_shard: Arc<Mutex<LatestBurnShardCache>>,
}

#[derive(Debug, Clone)]
pub enum CacheError {
    RpcError(String),
    ParseError(String),
    LockError(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            CacheError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            CacheError::LockError(msg) => write!(f, "Lock error: {}", msg),
        }
    }
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            latest_burn_shard: Arc::new(Mutex::new(LatestBurnShardCache::default())),
        }
    }

    // Get latest burn shard data (with automatic refresh if expired)
    pub async fn get_latest_burn_shard(&self) -> Result<Vec<BurnRecord>, CacheError> {
        // Check if cache is expired
        let should_refresh = {
            let cache = self.latest_burn_shard.lock()
                .map_err(|e| CacheError::LockError(e.to_string()))?;
            cache.is_expired() || cache.data.is_empty()
        };

        if should_refresh {
            log::info!("Latest burn shard cache expired or empty, fetching fresh data");
            self.refresh_latest_burn_shard().await?;
        } else {
            log::info!("Using cached latest burn shard data");
        }

        // Return cached data
        let cache = self.latest_burn_shard.lock()
            .map_err(|e| CacheError::LockError(e.to_string()))?;
        Ok(cache.data.clone())
    }

    // Force refresh latest burn shard data from chain
    pub async fn refresh_latest_burn_shard(&self) -> Result<Vec<BurnRecord>, CacheError> {
        let rpc = RpcConnection::new();
        
        match rpc.get_latest_burn_shard().await {
            Ok(data_str) => {
                let burn_records = self.parse_latest_burn_shard(&data_str)?;
                
                // Update cache
                {
                    let mut cache = self.latest_burn_shard.lock()
                        .map_err(|e| CacheError::LockError(e.to_string()))?;
                    cache.update_data(burn_records.clone());
                }
                
                log::info!("Successfully refreshed and cached {} burn records", burn_records.len());
                Ok(burn_records)
            }
            Err(e) => {
                log::error!("Failed to fetch latest burn shard: {}", e);
                Err(CacheError::RpcError(format!("Failed to fetch data: {}", e)))
            }
        }
    }

    // Clear latest burn shard cache
    pub fn clear_latest_burn_shard_cache(&self) -> Result<(), CacheError> {
        let mut cache = self.latest_burn_shard.lock()
            .map_err(|e| CacheError::LockError(e.to_string()))?;
        cache.clear_data();
        log::info!("Cleared latest burn shard cache");
        Ok(())
    }

    // Set cache TTL for latest burn shard
    pub fn set_latest_burn_shard_ttl(&self, ttl_ms: u64) -> Result<(), CacheError> {
        let mut cache = self.latest_burn_shard.lock()
            .map_err(|e| CacheError::LockError(e.to_string()))?;
        cache.set_ttl(ttl_ms);
        log::info!("Set latest burn shard cache TTL to {} ms", ttl_ms);
        Ok(())
    }

    // Get cache status
    pub fn get_cache_status(&self) -> Result<(usize, f64, bool), CacheError> {
        let cache = self.latest_burn_shard.lock()
            .map_err(|e| CacheError::LockError(e.to_string()))?;
        Ok((cache.data.len(), cache.last_updated, cache.is_expired()))
    }

    // Parse latest burn shard data from RPC response
    fn parse_latest_burn_shard(&self, data_str: &str) -> Result<Vec<BurnRecord>, CacheError> {
        let value: serde_json::Value = serde_json::from_str(data_str)
            .map_err(|e| CacheError::ParseError(format!("Failed to parse JSON: {}", e)))?;

        log::info!("Parsing latest burn shard data...");

        // Check if account exists
        if value["value"].is_null() {
            log::info!("Latest burn shard account does not exist yet");
            return Ok(Vec::new());
        }

        // Get base64 encoded data
        if let Some(data_str) = value["value"]["data"].get(0).and_then(|v| v.as_str()) {
            log::info!("Found base64 encoded burn shard data");
            
            // Decode base64 data
            let data_bytes = base64::decode(data_str)
                .map_err(|e| CacheError::ParseError(format!("Failed to decode base64: {}", e)))?;

            if data_bytes.len() < 8 {
                return Err(CacheError::ParseError("Data too short".to_string()));
            }

            // Skip discriminator (8 bytes)
            let mut data = &data_bytes[8..];

            if data.is_empty() {
                log::info!("No burn records in latest burn shard");
                return Ok(Vec::new());
            }

            // Parse current_index (1 byte)
            if data.len() < 1 {
                return Err(CacheError::ParseError("Invalid data format".to_string()));
            }
            let _current_index = data[0];
            data = &data[1..];

            // Parse record vector length (4 bytes)
            if data.len() < 4 {
                return Err(CacheError::ParseError("Invalid vector length".to_string()));
            }
            let vec_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
            data = &data[4..];

            log::info!("Found {} burn records in latest burn shard", vec_len);

            let mut burn_records = Vec::new();

            // Parse each record
            for i in 0..vec_len {
                // Parse pubkey (32 bytes)
                if data.len() < 32 {
                    log::warn!("Record {} has insufficient data for pubkey", i);
                    break;
                }
                let mut pubkey_bytes = [0u8; 32];
                pubkey_bytes.copy_from_slice(&data[..32]);
                let pubkey = Pubkey::new_from_array(pubkey_bytes).to_string();
                data = &data[32..];

                // Parse signature length (4 bytes) + signature string
                if data.len() < 4 {
                    log::warn!("Record {} has insufficient data for signature length", i);
                    break;
                }
                let sig_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
                data = &data[4..];

                if data.len() < sig_len {
                    log::warn!("Record {} has invalid signature length", i);
                    break;
                }
                
                let signature = String::from_utf8(data[..sig_len].to_vec())
                    .map_err(|e| CacheError::ParseError(format!("Invalid UTF-8 in signature: {}", e)))?;
                data = &data[sig_len..];

                // Parse slot (8 bytes)
                if data.len() < 8 {
                    log::warn!("Record {} missing slot field", i);
                    break;
                }
                let slot = u64::from_le_bytes(data[..8].try_into().unwrap());
                data = &data[8..];

                // Parse blocktime (8 bytes)
                if data.len() < 8 {
                    log::warn!("Record {} missing blocktime field", i);
                    break;
                }
                let blocktime = i64::from_le_bytes(data[..8].try_into().unwrap());
                data = &data[8..];

                // Parse amount (8 bytes)
                if data.len() < 8 {
                    log::warn!("Record {} missing amount field", i);
                    break;
                }
                let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
                data = &data[8..];

                log::info!("Parsed record {}: {} lamports burned by {} at slot {} ({})", 
                    i, amount, &pubkey, slot, blocktime);

                burn_records.push(BurnRecord {
                    pubkey,
                    signature,
                    slot,
                    blocktime,
                    amount,
                });
            }

            // Sort by blocktime in descending order (latest first)
            burn_records.sort_by(|a, b| b.blocktime.cmp(&a.blocktime));

            log::info!("Successfully parsed {} burn records", burn_records.len());
            Ok(burn_records)
        } else {
            log::warn!("No data field found in account");
            Ok(Vec::new())
        }
    }
}

// Global cache instance (lazy static)
use std::sync::OnceLock;

static CACHE: OnceLock<CacheManager> = OnceLock::new();

// Get global cache instance
pub fn get_cache() -> &'static CacheManager {
    CACHE.get_or_init(|| {
        log::info!("Initializing global cache manager");
        CacheManager::new()
    })
}

// Convenience functions for easy access
pub async fn get_latest_burn_shard() -> Result<Vec<BurnRecord>, CacheError> {
    get_cache().get_latest_burn_shard().await
}

pub async fn refresh_latest_burn_shard() -> Result<Vec<BurnRecord>, CacheError> {
    get_cache().refresh_latest_burn_shard().await
}

pub fn clear_latest_burn_shard_cache() -> Result<(), CacheError> {
    get_cache().clear_latest_burn_shard_cache()
} 