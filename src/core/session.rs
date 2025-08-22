use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};
use crate::core::encrypt;
use crate::core::rpc_base::{RpcConnection, RpcError};
use crate::core::rpc_mint::MintConfig;
use crate::core::rpc_profile::{UserProfile, parse_user_profile_new};
use web_sys::js_sys::Date;
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;
use hex;
use solana_sdk::pubkey::Pubkey;
use serde_json;
use base64;
use std::fmt;
use log;

#[derive(Debug, Clone)]
pub enum SessionError {
    Encryption(String),
    Expired,
    InvalidPassword,
    NotInitialized,
    InvalidData(String),
    ProfileError(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            SessionError::Expired => write!(f, "Session expired"),
            SessionError::InvalidPassword => write!(f, "Invalid password"),
            SessionError::NotInitialized => write!(f, "Session not initialized"),
            SessionError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            SessionError::ProfileError(msg) => write!(f, "Profile error: {}", msg),
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

        match (&self.encrypted_seed, &self.session_key) {
            (Some(encrypted_seed), Some(session_key)) => {
                encrypt::decrypt(encrypted_seed, session_key.expose_secret())
                    .map_err(|e| SessionError::Encryption(e.to_string()))
            },
            _ => Err(SessionError::NotInitialized),
        }
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
        // clear sensitive data
        self.session_key = None; // Secret will be dropped automatically
        self.encrypted_seed = None;
        self.cached_pubkey = None;
        self.user_profile = None;
        self.sol_balance = 0.0;
        self.token_balance = 0.0;
        self.balance_update_needed = false;
        self.ui_locked = false;
    }

    // update config
    pub fn update_config(&mut self, config: SessionConfig) {
        self.config = config;
    }

    // get public key
    pub fn get_public_key(&self) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        match &self.cached_pubkey {
            Some(pubkey) => Ok(pubkey.clone()),
            None => Err(SessionError::NotInitialized),
        }
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

    // fetch and cache user profile (updated for new profile system)
    pub async fn fetch_and_cache_user_profile(&mut self) -> Result<Option<UserProfile>, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let pubkey = self.get_public_key()?;
        let rpc = RpcConnection::new();

        match rpc.get_profile(&pubkey).await {
            Ok(Some(profile)) => {
                log::info!("Successfully fetched and cached user profile");
                self.user_profile = Some(profile.clone());
                Ok(Some(profile))
            },
            Ok(None) => {
                log::info!("User profile not found for pubkey: {}", pubkey);
                self.user_profile = None;
                Ok(None)
            },
            Err(e) => {
                log::error!("Failed to fetch user profile: {}", e);
                Err(SessionError::ProfileError(format!("RPC error: {}", e)))
            }
        }
    }

    /// Create user profile
    pub async fn create_profile(
        &mut self,
        burn_amount: u64,
        username: String,
        image: String,
        about_me: Option<String>,
    ) -> Result<String, SessionError> {
        // get keypair using the correct method
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;
        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array, crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;

        let rpc = RpcConnection::new();
        
        // Convert parameters to the expected types
        let about_me_str = about_me.unwrap_or_default();
        
        match rpc.create_profile(&keypair, burn_amount, &username, &image, &about_me_str).await {
            Ok(tx_hash) => {
                log::info!("Profile created successfully: {}", tx_hash);
                // Refresh profile cache after successful creation
                let _ = self.fetch_and_cache_user_profile().await;
                Ok(tx_hash)
            },
            Err(e) => {
                log::error!("Failed to create profile: {}", e);
                Err(SessionError::ProfileError(format!("Failed to create profile: {}", e)))
            }
        }
    }

    /// Update user profile
    pub async fn update_profile(
        &mut self,
        burn_amount: u64,
        username: Option<String>,
        image: Option<String>,
        about_me: Option<String>, 
    ) -> Result<String, SessionError> {
        // get keypair using the correct method
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;
        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array, crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption(format!("Failed to derive keypair: {:?}", e)))?;

        let rpc = RpcConnection::new();
        
        match rpc.update_profile(&keypair, burn_amount, username, image, about_me).await {
            Ok(tx_hash) => {
                log::info!("Profile updated successfully: {}", tx_hash);
                // Refresh profile cache after successful update
                let _ = self.fetch_and_cache_user_profile().await;
                Ok(tx_hash)
            },
            Err(e) => {
                log::error!("Failed to update profile: {}", e);
                Err(SessionError::ProfileError(format!("Failed to update profile: {}", e)))
            }
        }
    }

    // delete user profile
    pub async fn delete_profile(&mut self) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption("Failed to derive keypair".to_string()))?;

        let rpc = RpcConnection::new();
        
        match rpc.delete_profile(&keypair).await {
            Ok(tx_hash) => {
                log::info!("Profile deleted successfully: {}", tx_hash);
                // Clear profile cache after successful deletion
                self.user_profile = None;
                Ok(tx_hash)
            },
            Err(e) => {
                log::error!("Failed to delete profile: {}", e);
                Err(SessionError::ProfileError(format!("Delete profile error: {}", e)))
            }
        }
    }

    // helper function to get keypair bytes
    fn get_keypair_bytes(&self) -> Result<Vec<u8>, SessionError> {
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption("Failed to derive keypair".to_string()))?;

        Ok(keypair.to_bytes().to_vec())
    }

    // mint tokens using memo - internal handle all key operations
    pub async fn mint(&mut self, memo: &str) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let keypair_bytes = self.get_keypair_bytes()?;
        let rpc = RpcConnection::new();
        
        match rpc.mint_legacy(memo, &keypair_bytes).await {
            Ok(tx_hash) => {
                log::info!("Mint transaction sent: {}", tx_hash);
                self.balance_update_needed = true;
                Ok(tx_hash)
            },
            Err(e) => {
                log::error!("Mint transaction failed: {}", e);
                Err(SessionError::InvalidData(format!("Mint error: {}", e)))
            }
        }
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
    pub async fn burn(&mut self, amount: u64, message: &str) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let keypair_bytes = self.get_keypair_bytes()?;
        let rpc = RpcConnection::new();
        
        match rpc.burn(amount, "", message, &keypair_bytes).await {
            Ok(tx_hash) => {
                log::info!("Burn transaction sent: {}", tx_hash);
                self.balance_update_needed = true;
                Ok(tx_hash)
            },
            Err(e) => {
                log::error!("Burn transaction failed: {}", e);
                Err(SessionError::InvalidData(format!("Burn error: {}", e)))
            }
        }
    }

    // burn with history - for future use
    pub async fn burn_with_history(&mut self, amount: u64, message: &str, signature: &str) -> Result<String, SessionError> {
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        let keypair_bytes = self.get_keypair_bytes()?;
        let rpc = RpcConnection::new();
        
        match rpc.burn_with_history(amount, signature, message, &keypair_bytes).await {
            Ok(tx_hash) => {
                log::info!("Burn with history transaction sent: {}", tx_hash);
                self.balance_update_needed = true;
                Ok(tx_hash)
            },
            Err(e) => {
                log::error!("Burn with history transaction failed: {}", e);
                Err(SessionError::InvalidData(format!("Burn with history error: {}", e)))
            }
        }
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

        let keypair_bytes = self.get_keypair_bytes()?;
        let rpc = RpcConnection::new();
        
        match rpc.send_chat_message(group_id, message, &keypair_bytes, receiver, reply_to_sig).await {
            Ok(tx_hash) => {
                log::info!("Send chat message transaction sent: {}", tx_hash);
                self.balance_update_needed = true;
                Ok(tx_hash)
            },
            Err(e) => {
                log::error!("Send chat message transaction failed: {}", e);
                Err(SessionError::InvalidData(format!("Send chat message error: {}", e)))
            }
        }
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

        let keypair_bytes = self.get_keypair_bytes()?;
        let rpc = RpcConnection::new();
        
        match rpc.send_chat_message_with_timeout(group_id, message, &keypair_bytes, receiver, reply_to_sig, timeout_ms).await {
            Ok(tx_hash) => {
                log::info!("Send chat message with timeout transaction sent: {}", tx_hash);
                self.balance_update_needed = true;
                Ok(tx_hash)
            },
            Err(e) => {
                log::error!("Send chat message with timeout transaction failed: {}", e);
                Err(SessionError::InvalidData(format!("Send chat message with timeout error: {}", e)))
            }
        }
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

    /// Burn tokens for a chat group
    /// 
    /// # Parameters
    /// * `group_id` - The ID of the chat group to burn tokens for
    /// * `amount` - Amount of MEMO tokens to burn (in token units, not lamports)
    /// * `message` - Optional burn message (max 512 characters)
    /// 
    /// # Returns
    /// Result containing transaction signature
    pub async fn burn_tokens_for_group(
        &mut self,
        group_id: u64,
        amount: u64,
        message: &str,
    ) -> Result<String, SessionError> {
        // Check if session is valid
        if self.is_expired() {
            return Err(SessionError::Expired);
        }

        // Get the seed and convert to keypair
        let seed = self.get_seed()?;
        let seed_bytes = hex::decode(&seed)
            .map_err(|e| SessionError::Encryption(format!("Failed to decode seed: {}", e)))?;
        
        let seed_array: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (keypair, _) = crate::core::wallet::derive_keypair_from_seed(
            &seed_array,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::InvalidData(format!("Failed to derive keypair: {:?}", e)))?;

        // Convert amount from tokens to lamports
        let amount_lamports = amount * 1_000_000;

        // Call RPC
        let rpc = crate::core::rpc_base::RpcConnection::new();
        let signature = rpc.burn_tokens_for_group(
            group_id,
            amount_lamports,
            message,
            &keypair.to_bytes(),
        ).await.map_err(|e| SessionError::InvalidData(format!("Burn tokens for group failed: {}", e)))?;

        // Update balances after successful burn
        match self.fetch_and_update_balances().await {
            Ok(()) => {
                log::info!("Successfully updated balances after burning tokens for group");
            },
            Err(e) => {
                log::error!("Failed to update balances after burning tokens for group: {}", e);
                // Mark that we need to update balances later
                self.mark_balance_update_needed();
            }
        }

        Ok(signature)
    }
}

// implement zeroize for Session to ensure sensitive data is cleared
impl Zeroize for Session {
    fn zeroize(&mut self) {
        self.clear();
    }
}

// implement drop for Session to ensure sensitive data is cleared
impl Drop for Session {
    fn drop(&mut self) {
        self.clear();
    }
}

// Legacy parse function for compatibility (now uses new profile system)
pub fn parse_user_profile(account_data: &str) -> Result<UserProfile, SessionError> {
    log::info!("Starting to parse user profile from account data using new format");
    
    match parse_user_profile_new(account_data) {
        Ok(profile) => {
            log::info!("Successfully parsed new format user profile");
            Ok(profile)
        },
        Err(e) => {
            log::error!("Failed to parse new format user profile: {}", e);
            Err(SessionError::ProfileError(format!("Parse error: {}", e)))
        }
    }
} 