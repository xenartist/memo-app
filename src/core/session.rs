use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};
use crate::core::encrypt;
use crate::core::rpc_base::RpcConnection;
use web_sys::js_sys::Date;
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;
use hex;
use solana_sdk::pubkey::Pubkey;
use serde_json;
use base64;
use std::fmt;
use log;

#[derive(Clone, Debug)]
pub struct UserProfile {
    pub pubkey: String,           // pubkey
    pub total_minted: u64,        // total minted
    pub total_burned: u64,        // total burned
    pub mint_count: u64,          // mint count
    pub burn_count: u64,          // burn count
    pub created_at: i64,          // created at
    pub last_updated: i64,        // last updated
}

// cached LatestBurnShard 
#[derive(Clone, Debug)]
pub struct LatestBurnShard {
    pub data: Vec<BurnRecord>,
    pub last_updated: f64,     // cache last updated time
    pub cache_ttl: u64,        // TTL in milliseconds (default 10 minutes)
}

#[derive(Clone, Debug)]
pub struct BurnRecord {
    pub pubkey: String,      // wallet address
    pub signature: String,   // transaction signature (base58 encoded)
    pub slot: u64,          // slot
    pub blocktime: i64,     // blocktime
    pub amount: u64,        // amount of tokens burned (lamports)
}

impl Default for LatestBurnShard {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            last_updated: 0.0,
            cache_ttl: 10 * 60 * 1000, // 10 minutes
        }
    }
}

impl LatestBurnShard {
    pub fn is_expired(&self) -> bool {
        let current_time = Date::now();
        (current_time - self.last_updated) > self.cache_ttl as f64
    }

    pub fn update_data(&mut self, data: Vec<BurnRecord>) {
        self.data = data;
        self.last_updated = Date::now();
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.last_updated = 0.0;
    }
}

#[derive(Debug, Clone)]
pub enum SessionError {
    Encryption(String),
    Expired,
    InvalidPassword,
    NotInitialized,
    InvalidData(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            SessionError::Expired => write!(f, "Session expired"),
            SessionError::InvalidPassword => write!(f, "Invalid password"),
            SessionError::NotInitialized => write!(f, "Session not initialized"),
            SessionError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionConfig {
    // session timeout in minutes
    timeout_minutes: u32,
    // minimum amount to confirm in lamports
    confirm_threshold: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout_minutes: 60,  // default 1 hour timeout
            confirm_threshold: 1_000_000_000,  // default 1 SOL confirmation
        }
    }
}

#[derive(Clone)]
pub struct Session {
    // session config
    config: SessionConfig,
    // session start time
    start_time: f64,
    // encrypted seed
    encrypted_seed: Option<String>,
    // session key
    session_key: Option<Secret<String>>,
    // UI locked
    ui_locked: bool,
    // user profile
    user_profile: Option<UserProfile>,
    // cached pubkey
    cached_pubkey: Option<String>,
    // latest burn shard cache
    latest_burn_shard: Option<LatestBurnShard>,
}

impl Session {
    pub fn new(config: Option<SessionConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            start_time: Date::now(),
            encrypted_seed: None,
            session_key: None,
            ui_locked: false,
            user_profile: None,
            cached_pubkey: None,
            latest_burn_shard: None,
        }
    }

    // initialize session, decrypt seed using user password and re-encrypt using session key
    pub async fn initialize(&mut self, encrypted_seed: &str, password: &str) -> Result<(), SessionError> {
        // decrypt original seed
        let seed = encrypt::decrypt(encrypted_seed, password)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        // generate new session key
        let session_key = encrypt::generate_random_key();

        // re-encrypt seed using session key
        let session_encrypted_seed = encrypt::encrypt(&seed, session_key.expose_secret())
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        // get pubkey
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;
        
        let seed: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (_, pubkey) = crate::core::wallet::derive_keypair_from_seed(
            &seed,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption("Failed to derive keypair".to_string()))?;

        // save session info
        self.session_key = Some(session_key);
        self.encrypted_seed = Some(session_encrypted_seed);
        self.start_time = Date::now();
        self.cached_pubkey = Some(pubkey);

        Ok(())
    }

    // check if session is expired
    pub fn is_expired(&self) -> bool {
        let current_time = Date::now();
        let elapsed_minutes = (current_time - self.start_time) / (60.0 * 1000.0);
        elapsed_minutes > self.config.timeout_minutes as f64
    }

    // get decrypted seed (if session is valid)
    pub fn get_seed(&self) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let session_key = self.session_key.as_ref()
            .ok_or(SessionError::NotInitialized)?;
        let encrypted_seed = self.encrypted_seed.as_ref()
            .ok_or(SessionError::NotInitialized)?;

        encrypt::decrypt(encrypted_seed, session_key.expose_secret())
            .map_err(|e| SessionError::Encryption(e.to_string()))
    }

    // check if operation needs additional password confirmation
    pub fn needs_confirmation(&self, amount: u64) -> bool {
        amount >= self.config.confirm_threshold
    }

    // verify password (for operations that need confirmation)
    pub fn verify_password(&self, password: &str, original_encrypted_seed: &str) -> Result<bool, SessionError> {
        // try to decrypt original encrypted seed
        encrypt::decrypt(original_encrypted_seed, password)
            .map(|_| true)
            .map_err(|_| SessionError::InvalidPassword)
    }

    // refresh session time
    pub fn refresh(&mut self) {
        self.start_time = Date::now();
    }

    // clear session data
    pub fn clear(&mut self) {
        if let Some(encrypted_seed) = self.encrypted_seed.as_mut() {
            encrypted_seed.zeroize();
        }
        self.encrypted_seed = None;
        self.session_key = None;
        self.cached_pubkey = None;
        self.latest_burn_shard = None; // clear cache
    }

    // update config
    pub fn update_config(&mut self, config: SessionConfig) {
        self.config = config;
    }

    pub fn get_public_key(&self) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        self.cached_pubkey.clone()
            .ok_or(SessionError::NotInitialized)
    }

    // lock UI
    pub fn lock_ui(&mut self) {
        self.ui_locked = true;
    }

    pub fn unlock_ui(&mut self, password: &str, original_encrypted_seed: &str) -> Result<(), SessionError> {
        match self.verify_password(password, original_encrypted_seed) {
            Ok(true) => {
                self.ui_locked = false;
                Ok(())
            },
            Ok(false) => Err(SessionError::InvalidPassword),
            Err(e) => Err(e),
        }
    }

    // check if UI is locked
    pub fn can_access_ui(&self) -> bool {
        !self.ui_locked
    }

    // get user profile
    pub fn get_user_profile(&self) -> Option<UserProfile> {
        self.user_profile.clone()
    }

    // set user profile
    pub fn set_user_profile(&mut self, profile: Option<UserProfile>) {
        self.user_profile = profile;
    }

    // initialize session with seed
    pub async fn initialize_with_seed(&mut self, seed: &str) -> Result<(), SessionError> {
        // generate new session key
        let session_key = encrypt::generate_random_key();

        // re-encrypt seed using session key
        let session_encrypted_seed = encrypt::encrypt(seed, session_key.expose_secret())
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        // get pubkey
        let seed_bytes = hex::decode(seed)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;
        
        let seed: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (_, pubkey) = crate::core::wallet::derive_keypair_from_seed(
            &seed,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption("Failed to derive keypair".to_string()))?;

        // save session info
        self.session_key = Some(session_key);
        self.encrypted_seed = Some(session_encrypted_seed.to_string());
        self.start_time = Date::now();
        self.cached_pubkey = Some(pubkey);

        Ok(())
    }

    // fetch and cache user profile
    pub async fn fetch_and_cache_user_profile(&mut self) -> Result<Option<UserProfile>, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let pubkey = self.get_public_key()?;
        let rpc = RpcConnection::new();

        match rpc.get_user_profile(&pubkey).await {
            Ok(account_data) => {
                // check if account exists
                let account_info: serde_json::Value = serde_json::from_str(&account_data)
                    .map_err(|e| SessionError::InvalidData(format!("Failed to parse account data: {}", e)))?;

                if account_info["value"].is_null() {
                    // account not exists
                    log::info!("User profile not found for pubkey: {}", pubkey);
                    self.user_profile = None;
                    Ok(None)
                } else {
                    // parse user profile
                    match parse_user_profile(&account_data) {
                        Ok(profile) => {
                            log::info!("Successfully fetched and cached user profile");
                            self.user_profile = Some(profile.clone());
                            Ok(Some(profile))
                        },
                        Err(e) => {
                            log::error!("Failed to parse user profile: {}", e);
                            Err(e)
                        }
                    }
                }
            },
            Err(e) => {
                log::error!("Failed to fetch user profile: {}", e);
                Err(SessionError::InvalidData(format!("RPC error: {}", e)))
            }
        }
    }

    // check if user has profile
    pub fn has_user_profile(&self) -> bool {
        self.user_profile.is_some()
    }

    // get cached latest burn shard data
    pub async fn get_latest_burn_shard(&mut self) -> Result<Vec<BurnRecord>, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        // check if cache exists and is not expired
        if let Some(cache) = &self.latest_burn_shard {
            if !cache.is_expired() {
                log::info!("Using cached latest burn shard data");
                return Ok(cache.data.clone());
            } else {
                log::info!("Latest burn shard cache expired, fetching fresh data");
            }
        } else {
            log::info!("No latest burn shard cache found, fetching fresh data");
        }

        // cache does not exist or is expired, fetch data from chain
        self.fetch_and_cache_latest_burn_shard().await
    }

    // fetch and cache latest burn shard data from chain
    async fn fetch_and_cache_latest_burn_shard(&mut self) -> Result<Vec<BurnRecord>, SessionError> {
        let rpc = RpcConnection::new();
        
        match rpc.get_latest_burn_shard().await {
            Ok(data_str) => {
                match self.parse_latest_burn_shard(&data_str) {
                    Ok(burn_records) => {
                        // update cache
                        if self.latest_burn_shard.is_none() {
                            self.latest_burn_shard = Some(LatestBurnShard::default());
                        }
                        
                        if let Some(cache) = &mut self.latest_burn_shard {
                            cache.update_data(burn_records.clone());
                        }
                        
                        log::info!("Successfully fetched and cached {} burn records", burn_records.len());
                        Ok(burn_records)
                    }
                    Err(e) => {
                        log::error!("Failed to parse latest burn shard data: {}", e);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to fetch latest burn shard: {}", e);
                Err(SessionError::InvalidData(format!("RPC error: {}", e)))
            }
        }
    }

    // parse
    fn parse_latest_burn_shard(&self, data_str: &str) -> Result<Vec<BurnRecord>, SessionError> {
        let value: serde_json::Value = serde_json::from_str(data_str)
            .map_err(|e| SessionError::InvalidData(format!("Failed to parse JSON: {}", e)))?;

        log::info!("Parsing latest burn shard data...");

        // check if account exists
        if value["value"].is_null() {
            log::info!("Latest burn shard account does not exist yet");
            return Ok(Vec::new());
        }

        // get base64 encoded data
        if let Some(data_str) = value["value"]["data"].get(0).and_then(|v| v.as_str()) {
            log::info!("Found base64 encoded burn shard data");
            
            // decode base64 data
            let data_bytes = base64::decode(data_str)
                .map_err(|e| SessionError::InvalidData(format!("Failed to decode base64: {}", e)))?;

            if data_bytes.len() < 8 {
                return Err(SessionError::InvalidData("Data too short".to_string()));
            }

            // skip discriminator (8 bytes)
            let mut data = &data_bytes[8..];

            if data.is_empty() {
                log::info!("No burn records in latest burn shard");
                return Ok(Vec::new());
            }

            // parse current_index (1 byte)
            if data.len() < 1 {
                return Err(SessionError::InvalidData("Invalid data format".to_string()));
            }
            let _current_index = data[0];
            data = &data[1..];

            // parse record vector length (4 bytes)
            if data.len() < 4 {
                return Err(SessionError::InvalidData("Invalid vector length".to_string()));
            }
            let vec_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
            data = &data[4..];

            log::info!("Found {} burn records in latest burn shard", vec_len);

            let mut burn_records = Vec::new();

            // parse each record (according to the correct BurnRecord structure)
            for i in 0..vec_len {
                // parse pubkey (32 bytes)
                if data.len() < 32 {
                    log::warn!("Record {} has insufficient data for pubkey", i);
                    break;
                }
                let mut pubkey_bytes = [0u8; 32];
                pubkey_bytes.copy_from_slice(&data[..32]);
                let pubkey = solana_sdk::pubkey::Pubkey::new_from_array(pubkey_bytes).to_string();
                data = &data[32..];

                // parse signature length (4 bytes) + signature string (88 bytes)
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
                    .map_err(|e| SessionError::InvalidData(format!("Invalid UTF-8 in signature: {}", e)))?;
                data = &data[sig_len..];

                // parse slot (8 bytes)
                if data.len() < 8 {
                    log::warn!("Record {} missing slot field", i);
                    break;
                }
                let slot = u64::from_le_bytes(data[..8].try_into().unwrap());
                data = &data[8..];

                // parse blocktime (8 bytes)
                if data.len() < 8 {
                    log::warn!("Record {} missing blocktime field", i);
                    break;
                }
                let blocktime = i64::from_le_bytes(data[..8].try_into().unwrap());
                data = &data[8..];

                // parse amount (8 bytes)
                if data.len() < 8 {
                    log::warn!("Record {} missing amount field", i);
                    break;
                }
                let amount = u64::from_le_bytes(data[..8].try_into().unwrap());
                data = &data[8..];

                // log first, then create structure to avoid moving error
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

            // sort by blocktime in descending order (latest first)
            burn_records.sort_by(|a, b| b.blocktime.cmp(&a.blocktime));

            log::info!("Successfully parsed {} burn records", burn_records.len());
            Ok(burn_records)
        } else {
            log::warn!("No data field found in account");
            Ok(Vec::new())
        }
    }

    // clear latest burn shard cache
    pub fn clear_latest_burn_shard_cache(&mut self) {
        if let Some(cache) = &mut self.latest_burn_shard {
            cache.clear();
        }
        log::info!("Cleared latest burn shard cache");
    }

    // force refresh latest burn shard cache
    pub async fn refresh_latest_burn_shard(&mut self) -> Result<Vec<BurnRecord>, SessionError> {
        self.clear_latest_burn_shard_cache();
        self.fetch_and_cache_latest_burn_shard().await
    }
}

// implement Drop trait to ensure session data is properly cleaned up
impl Drop for Session {
    fn drop(&mut self) {
        self.clear();
    }
}

pub fn parse_user_profile(account_data: &str) -> Result<UserProfile, SessionError> {
    log::info!("Starting to parse user profile from account data");
    
    let value: serde_json::Value = serde_json::from_str(account_data)
        .map_err(|e| {
            log::error!("Failed to parse account data as JSON: {}", e);
            SessionError::InvalidData(e.to_string())
        })?;

    // get data from JSON
    if let Some(data) = value.get("value").and_then(|v| v.get("data")) {
        if let Some(data_str) = data.get(0).and_then(|v| v.as_str()) {
            log::info!("Found base64 encoded data");
            
            // decode base64 data to bytes
            let data_bytes = base64::decode(data_str)
                .map_err(|e| {
                    log::error!("Failed to decode base64: {}", e);
                    SessionError::InvalidData(format!("Failed to decode base64: {}", e))
                })?;
            
            log::info!("Successfully decoded base64 data, length: {}", data_bytes.len());
            
            // ensure data length is enough
            if data_bytes.len() < 8 {
                log::error!("Data too short: {}", data_bytes.len());
                return Err(SessionError::InvalidData("Data too short".to_string()));
            }
            
            // skip discriminator
            let mut data = &data_bytes[8..];
            
            // read pubkey
            if data.len() < 32 {
                log::error!("Invalid pubkey length: {}", data.len());
                return Err(SessionError::InvalidData("Invalid pubkey length".to_string()));
            }
            let pubkey = Pubkey::new_from_array(data[..32].try_into().unwrap()).to_string();
            log::info!("Parsed pubkey: {}", pubkey);
            data = &data[32..];
            
            // read stats data (mint/burn data only)
            if data.len() < 32 {
                log::error!("Invalid stats data length");
                return Err(SessionError::InvalidData("Invalid stats data".to_string()));
            }
            let total_minted = u64::from_le_bytes(data[..8].try_into().unwrap());
            let total_burned = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let mint_count = u64::from_le_bytes(data[16..24].try_into().unwrap());
            let burn_count = u64::from_le_bytes(data[24..32].try_into().unwrap());
            log::info!("Parsed stats - minted: {}, burned: {}, mint_count: {}, burn_count: {}", 
                total_minted, total_burned, mint_count, burn_count);
            data = &data[32..];
            
            // read timestamp
            if data.len() < 16 {
                log::error!("Invalid timestamp data length");
                return Err(SessionError::InvalidData("Invalid timestamp data".to_string()));
            }
            let created_at = i64::from_le_bytes(data[..8].try_into().unwrap());
            let last_updated = i64::from_le_bytes(data[8..16].try_into().unwrap());
            log::info!("Parsed timestamps - created: {}, updated: {}", created_at, last_updated);

            let profile = UserProfile {
                pubkey,
                total_minted,
                total_burned,
                mint_count,
                burn_count,
                created_at,
                last_updated,
            };
            
            log::info!("Successfully parsed user profile");
            Ok(profile)
        } else {
            log::error!("Data field is not a string");
            Err(SessionError::InvalidData("Data field is not a string".to_string()))
        }
    } else {
        log::error!("Invalid account data format");
        Err(SessionError::InvalidData("Invalid account data format".to_string()))
    }
} 