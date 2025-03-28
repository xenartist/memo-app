use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};
use crate::core::encrypt;

#[derive(Debug, Clone)]
pub enum SessionError {
    Encryption(String),
    Expired,
    InvalidPassword,
    NotInitialized,
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
    start_time: SystemTime,
    // encrypted seed
    encrypted_seed: Option<String>,
    // session key
    session_key: Option<String>,
}

impl Session {
    pub fn new(config: Option<SessionConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            start_time: SystemTime::now(),
            encrypted_seed: None,
            session_key: None,
        }
    }

    // initialize session, decrypt seed using user password and re-encrypt using session key
    pub fn initialize(&mut self, encrypted_seed: &str, password: &str) -> Result<(), SessionError> {
        // decrypt original seed
        let seed = encrypt::decrypt(encrypted_seed, password)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        // generate new session key
        let session_key = encrypt::generate_random_key();

        // re-encrypt seed using session key
        let session_encrypted_seed = encrypt::encrypt(&seed, &session_key)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        // save session info
        self.session_key = Some(session_key);
        self.encrypted_seed = Some(session_encrypted_seed);
        self.start_time = SystemTime::now();

        Ok(())
    }

    // check if session is expired
    pub fn is_expired(&self) -> bool {
        if let Ok(elapsed) = self.start_time.elapsed() {
            elapsed > Duration::from_secs(self.config.timeout_minutes as u64 * 60)
        } else {
            true
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

        encrypt::decrypt(encrypted_seed, session_key)
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
        self.start_time = SystemTime::now();
    }

    // clear session data
    pub fn clear(&mut self) {
        self.encrypted_seed = None;
        self.session_key = None;
    }

    // update config
    pub fn update_config(&mut self, config: SessionConfig) {
        self.config = config;
    }
}

// implement Drop trait to ensure session data is properly cleaned up
impl Drop for Session {
    fn drop(&mut self) {
        self.clear();
    }
} 