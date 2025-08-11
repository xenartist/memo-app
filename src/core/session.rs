use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};
use crate::core::encrypt;
use crate::core::rpc_base::{RpcConnection, RpcError};
use crate::core::rpc_mint::MintConfig;
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
    // session timeout in minutes, None means never expire
    timeout_minutes: Option<u32>,
    // minimum amount to confirm in lamports
    confirm_threshold: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout_minutes: None,  // never expire by default
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
    // balance information
    sol_balance: f64,
    token_balance: f64,
    // balance update trigger
    balance_update_needed: bool,
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
            sol_balance: 0.0,
            token_balance: 0.0,
            balance_update_needed: false,
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
        match self.config.timeout_minutes {
            None => false, // never expire
            Some(timeout_minutes) => {
                let current_time = Date::now();
                let elapsed_minutes = (current_time - self.start_time) / (60.0 * 1000.0);
                elapsed_minutes > timeout_minutes as f64
            }
        }
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

    // mint tokens using memo - internal handle all key operations
    pub async fn mint(&mut self, memo: &str) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        // internal get and handle keypair
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;

        let keypair_bytes = keypair.to_bytes().to_vec();

        // call RPC mint method
        let rpc = RpcConnection::new();
        let result = rpc.mint(memo, &keypair_bytes).await
            .map_err(|e| SessionError::InvalidData(format!("Mint failed: {}", e)))?;

        // mark that balances need to be updated after successful mint
        self.mark_balance_update_needed();
        
        Ok(result)
    }

    // check if user has profile
    pub fn has_user_profile(&self) -> bool {
        self.user_profile.is_some()
    }

    // balance related methods
    pub fn get_sol_balance(&self) -> f64 {
        self.sol_balance
    }

    pub fn get_token_balance(&self) -> f64 {
        self.token_balance
    }

    pub fn set_balances(&mut self, sol_balance: f64, token_balance: f64) {
        self.sol_balance = sol_balance;
        self.token_balance = token_balance;
        self.balance_update_needed = false;
    }

    pub fn mark_balance_update_needed(&mut self) {
        self.balance_update_needed = true;
    }

    pub fn is_balance_update_needed(&self) -> bool {
        self.balance_update_needed
    }

    // fetch and update balances
    pub async fn fetch_and_update_balances(&mut self) -> Result<(), SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let pubkey = self.get_public_key()?;
        let rpc = RpcConnection::new();
        
        // Now using global constant instead of local definition
        // get token balance
        match rpc.get_token_balance(&pubkey, MintConfig::TOKEN_MINT).await {
            Ok(token_result) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&token_result) {
                    if let Some(accounts) = json.get("value").and_then(|v| v.as_array()) {
                        if let Some(first_account) = accounts.first() {
                            if let Some(amount) = first_account
                                .get("account")
                                .and_then(|a| a.get("data"))
                                .and_then(|d| d.get("parsed"))
                                .and_then(|p| p.get("info"))
                                .and_then(|i| i.get("tokenAmount"))
                                .and_then(|t| t.get("uiAmount"))
                                .and_then(|a| a.as_f64())
                            {
                                self.token_balance = amount;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get token balance: {}", e);
            }
        }

        // get SOL balance
        match rpc.get_balance(&pubkey).await {
            Ok(balance_result) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&balance_result) {
                    if let Some(lamports) = json.get("value").and_then(|v| v.as_u64()) {
                        let sol = lamports as f64 / 1_000_000_000.0;
                        self.sol_balance = sol;
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to get SOL balance: {}", e);
            }
        }

        self.balance_update_needed = false;
        Ok(())
    }

    // check if expiration is enabled
    pub fn has_expiration(&self) -> bool {
        self.config.timeout_minutes.is_some()
    }

    // get expiration time setting
    pub fn get_timeout_minutes(&self) -> Option<u32> {
        self.config.timeout_minutes
    }

    // set expiration time (None = never expire)
    pub fn set_timeout(&mut self, timeout_minutes: Option<u32>) {
        self.config.timeout_minutes = timeout_minutes;
        // reset start time
        self.start_time = Date::now();
    }

    // get session remaining time
    pub fn get_remaining_time(&self) -> Option<f64> {
        match self.config.timeout_minutes {
            None => None, // never expire
            Some(timeout_minutes) => {
                let current_time = Date::now();
                let elapsed_minutes = (current_time - self.start_time) / (60.0 * 1000.0);
                let remaining = timeout_minutes as f64 - elapsed_minutes;
                Some(remaining.max(0.0))
            }
        }
    }

    // burn tokens using message and signature - internal handle all key operations
    pub async fn burn(&mut self, amount: u64, message: &str, signature: &str) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        // internal get and handle keypair
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;

        let keypair_bytes = keypair.to_bytes().to_vec();

        // call RPC burn method
        let rpc = RpcConnection::new();
        let result = rpc.burn(amount, message, signature, &keypair_bytes).await
            .map_err(|e| SessionError::InvalidData(format!("Burn failed: {}", e)))?;

        // mark that balances need to be updated after successful burn
        self.mark_balance_update_needed();
        
        Ok(result)
    }

    // burn with history - for future use
    pub async fn burn_with_history(&mut self, amount: u64, message: &str, signature: &str) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        // internal get and handle keypair
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;

        let keypair_bytes = keypair.to_bytes().to_vec();

        // call RPC burn_with_history method
        let rpc = RpcConnection::new();
        let result = rpc.burn_with_history(amount, message, signature, &keypair_bytes).await
            .map_err(|e| SessionError::InvalidData(format!("Burn with history failed: {}", e)))?;

        // mark that balances need to be updated after successful burn
        self.mark_balance_update_needed();
        
        Ok(result)
    }

    /// Send chat message to group - internal handle all key operations
    pub async fn send_chat_message(
        &mut self, 
        group_id: u64, 
        message: &str,
        receiver: Option<String>,
        reply_to_sig: Option<String>
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        // internal get and handle keypair
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;

        let keypair_bytes = keypair.to_bytes().to_vec();

        // call RPC send_chat_message method
        let rpc = RpcConnection::new();
        let result = rpc.send_chat_message(group_id, message, &keypair_bytes, receiver, reply_to_sig).await
            .map_err(|e| SessionError::InvalidData(format!("Send chat message failed: {}", e)))?;

        // mark that balances need to be updated after successful message send (user gets mint reward)
        self.mark_balance_update_needed();
        
        Ok(result)
    }

    /// Send a chat message to a group with timeout
    pub async fn send_chat_message_with_timeout(
        &mut self, 
        group_id: u64, 
        message: &str,
        receiver: Option<String>,
        reply_to_sig: Option<String>,
        timeout_ms: Option<u32>
    ) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        // internal get and handle keypair
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;

        let keypair_bytes = keypair.to_bytes().to_vec();

        // call RPC send_chat_message_with_timeout method
        let rpc = RpcConnection::new();
        let result = rpc.send_chat_message_with_timeout(group_id, message, &keypair_bytes, receiver, reply_to_sig, timeout_ms).await
            .map_err(|e| {
                log::error!("Session: RPC send_chat_message_with_timeout failed: {}", e);
                if e.to_string().contains("timeout") {
                    SessionError::InvalidData(format!("Message send timeout: {}", e))
                } else {
                    SessionError::InvalidData(format!("Send chat message failed: {}", e))
                }
            })?;

        log::info!("Session: Chat message sent successfully");

        // mark that balances need to be updated after successful message send (user gets mint reward)
        self.mark_balance_update_needed();
        
        Ok(result)
    }

    /// Create a new chat group - internal handle all key operations
    pub async fn create_chat_group(
        &mut self,
        name: &str,
        description: &str,
        image: &str,
        tags: Vec<String>,
        min_memo_interval: Option<i64>,
        burn_amount: u64,
    ) -> Result<(String, u64), SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        log::info!("Session: Creating chat group '{}' with {} tokens", name, burn_amount / 1_000_000);

        // get keypair
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;
        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array, crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;
        let keypair_bytes = keypair.to_bytes().to_vec();

        // call RPC method
        let rpc = RpcConnection::new();
        let result = rpc.create_chat_group(
            name, description, image, tags, min_memo_interval, burn_amount, &keypair_bytes
        ).await
            .map_err(|e| {
                log::error!("Session: RPC create_chat_group failed: {}", e);
                SessionError::InvalidData(format!("Create chat group failed: {}", e))
            })?;

        log::info!("Session: Chat group '{}' created successfully with ID {}", name, result.1);

        // mark that balances need to be updated after successful group creation (user gets mint reward)
        self.mark_balance_update_needed();
        
        Ok(result)
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