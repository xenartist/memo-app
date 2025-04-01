use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};
use crate::core::encrypt;
use crate::core::rpc::RpcConnection;
use web_sys::js_sys::Date;
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;
use hex;
use solana_sdk::pubkey::Pubkey;
use serde_json;
use base64;
use std::fmt;

#[derive(Clone, Debug)]
pub struct UserProfile {
    pub pubkey: String,           // pubkey
    pub username: String,         // username
    pub total_minted: u64,        // total minted
    pub total_burned: u64,        // total burned
    pub mint_count: u64,          // mint count
    pub burn_count: u64,          // burn count
    pub profile_image: String,    // profile image
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

        // save session info
        self.session_key = Some(session_key);
        self.encrypted_seed = Some(session_encrypted_seed);
        self.start_time = Date::now();

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
    }

    // update config
    pub fn update_config(&mut self, config: SessionConfig) {
        self.config = config;
    }

    pub fn get_public_key(&self) -> Result<String, SessionError> {
        let seed_hex = self.get_seed()?;
        
        let seed_bytes = hex::decode(&seed_hex)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;
        
        let seed: [u8; 64] = seed_bytes.try_into()
            .map_err(|_| SessionError::Encryption("Invalid seed length".to_string()))?;

        let (_, pubkey) = crate::core::wallet::derive_keypair_from_seed(
            &seed,
            crate::core::wallet::get_default_derivation_path()
        ).map_err(|e| SessionError::Encryption("Failed to derive keypair".to_string()))?;

        Ok(pubkey)
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
}

// implement Drop trait to ensure session data is properly cleaned up
impl Drop for Session {
    fn drop(&mut self) {
        self.clear();
    }
}

pub fn parse_user_profile(account_data: &str) -> Result<UserProfile, SessionError> {
    let value: serde_json::Value = serde_json::from_str(account_data)
        .map_err(|e| SessionError::InvalidData(e.to_string()))?;

    // get data from JSON
    if let Some(data) = value.get("value").and_then(|v| v.get("data")) {
        if let Some(data_str) = data.as_str() {
            // decode base64 data to bytes
            let data_bytes = base64::decode(data_str)
                .map_err(|e| SessionError::InvalidData(format!("Failed to decode base64: {}", e)))?;
            
            // ensure data length is enough
            if data_bytes.len() < 8 {
                return Err(SessionError::InvalidData("Data too short".to_string()));
            }
            
            // skip discriminator
            let mut data = &data_bytes[8..];
            
            // read pubkey
            if data.len() < 32 {
                return Err(SessionError::InvalidData("Invalid pubkey length".to_string()));
            }
            let pubkey = Pubkey::new_from_array(data[..32].try_into().unwrap()).to_string();
            data = &data[32..];
            
            // read username length
            if data.len() < 4 {
                return Err(SessionError::InvalidData("Invalid username length field".to_string()));
            }
            let username_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
            data = &data[4..];
            
            // read username
            if data.len() < username_len {
                return Err(SessionError::InvalidData("Invalid username data".to_string()));
            }
            let username = String::from_utf8(data[..username_len].to_vec())
                .map_err(|e| SessionError::InvalidData(format!("Invalid username UTF-8: {}", e)))?;
            data = &data[username_len..];
            
            // read stats data
            if data.len() < 32 {  // 4 u64, each 8 bytes
                return Err(SessionError::InvalidData("Invalid stats data".to_string()));
            }
            let total_minted = u64::from_le_bytes(data[..8].try_into().unwrap());
            let total_burned = u64::from_le_bytes(data[8..16].try_into().unwrap());
            let mint_count = u64::from_le_bytes(data[16..24].try_into().unwrap());
            let burn_count = u64::from_le_bytes(data[24..32].try_into().unwrap());
            data = &data[32..];
            
            // read profile_image length
            if data.len() < 4 {
                return Err(SessionError::InvalidData("Invalid profile image length field".to_string()));
            }
            let profile_image_len = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
            data = &data[4..];
            
            // read profile_image
            if data.len() < profile_image_len {
                return Err(SessionError::InvalidData("Invalid profile image data".to_string()));
            }
            let profile_image = String::from_utf8(data[..profile_image_len].to_vec())
                .map_err(|e| SessionError::InvalidData(format!("Invalid profile image UTF-8: {}", e)))?;
            data = &data[profile_image_len..];
            
            // read timestamp
            if data.len() < 16 {  // 2 i64, each 8 bytes
                return Err(SessionError::InvalidData("Invalid timestamp data".to_string()));
            }
            let created_at = i64::from_le_bytes(data[..8].try_into().unwrap());
            let last_updated = i64::from_le_bytes(data[8..16].try_into().unwrap());

            Ok(UserProfile {
                pubkey,
                username,
                total_minted,
                total_burned,
                mint_count,
                burn_count,
                profile_image,
                created_at,
                last_updated,
            })
        } else {
            Err(SessionError::InvalidData("Data field is not a string".to_string()))
        }
    } else {
        Err(SessionError::InvalidData("Invalid account data format".to_string()))
    }
} 