use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use once_cell::sync::Lazy;
use log::{debug, info};
use ed25519_dalek::{Keypair, SecretKey, PublicKey};

// Default session timeout in seconds (5 minutes)
const DEFAULT_SESSION_TIMEOUT: u64 = 300;

/// Wallet session to temporarily store decrypted mnemonic and keys
pub struct WalletSession {
    decrypted_mnemonic: Option<String>,
    keypair: Option<Keypair>,
    public_key: Option<PublicKey>,
    last_activity: u64,  // Unix timestamp
    timeout_seconds: u64, // Session timeout in seconds
}

// Global session instance using thread-safe singleton pattern
static GLOBAL_SESSION: Lazy<Arc<Mutex<WalletSession>>> = Lazy::new(|| {
    Arc::new(Mutex::new(WalletSession::new(DEFAULT_SESSION_TIMEOUT)))
});

impl WalletSession {
    /// Create a new wallet session with specified timeout
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            decrypted_mnemonic: None,
            keypair: None,
            public_key: None,
            last_activity: get_current_timestamp(),
            timeout_seconds,
        }
    }
    
    /// Set the decrypted mnemonic and derive keys in the session
    pub fn set_mnemonic_and_keys(&mut self, mnemonic: String, keypair: Keypair) {
        self.decrypted_mnemonic = Some(mnemonic);
        self.public_key = Some(keypair.public);
        self.keypair = Some(keypair);
        self.last_activity = get_current_timestamp();
        debug!("Mnemonic and keys stored in session");
    }
    
    /// Get the decrypted mnemonic if session is still valid
    pub fn get_mnemonic(&mut self) -> Option<String> {
        let now = get_current_timestamp();
        
        // Check if session has timed out
        if now - self.last_activity > self.timeout_seconds {
            debug!("Session expired, clearing mnemonic and keys");
            self.clear();
            return None;
        }
        
        // Update last activity time
        self.last_activity = now;
        self.decrypted_mnemonic.clone()
    }
    
    /// Get the keypair if session is still valid
    pub fn get_keypair(&mut self) -> Option<Keypair> {
        let now = get_current_timestamp();
        
        // Check if session has timed out
        if now - self.last_activity > self.timeout_seconds {
            debug!("Session expired, clearing mnemonic and keys");
            self.clear();
            return None;
        }
        
        // Update last activity time
        self.last_activity = now;
        
        // Clone the keypair if it exists
        self.keypair.as_ref().map(|kp| {
            Keypair {
                public: kp.public,
                secret: SecretKey::from_bytes(kp.secret.as_bytes()).unwrap(),
            }
        })
    }
    
    /// Get the public key if session is still valid
    pub fn get_public_key(&mut self) -> Option<PublicKey> {
        let now = get_current_timestamp();
        
        // Check if session has timed out
        if now - self.last_activity > self.timeout_seconds {
            debug!("Session expired, clearing mnemonic and keys");
            self.clear();
            return None;
        }
        
        // Update last activity time
        self.last_activity = now;
        self.public_key
    }
    
    /// Clear the session data
    pub fn clear(&mut self) {
        self.decrypted_mnemonic = None;
        self.keypair = None;
        self.public_key = None;
        debug!("Session cleared");
    }
    
    /// Check if the session is active (has a stored mnemonic and not expired)
    pub fn is_active(&mut self) -> bool {
        self.get_mnemonic().is_some()
    }
    
    /// Get the remaining time in seconds before session expires
    pub fn time_remaining(&self) -> u64 {
        let now = get_current_timestamp();
        let elapsed = now - self.last_activity;
        
        if elapsed >= self.timeout_seconds {
            0
        } else {
            self.timeout_seconds - elapsed
        }
    }
}

/// Get the global session instance
pub fn get_session() -> Arc<Mutex<WalletSession>> {
    GLOBAL_SESSION.clone()
}

/// Store mnemonic and keys in the global session
pub fn store_mnemonic_and_keys(mnemonic: String, keypair: Keypair) {
    if let Ok(mut session) = GLOBAL_SESSION.lock() {
        session.set_mnemonic_and_keys(mnemonic, keypair);
        info!("Mnemonic and keys stored in global session");
    }
}

/// Retrieve mnemonic from the global session
pub fn retrieve_mnemonic() -> Option<String> {
    if let Ok(mut session) = GLOBAL_SESSION.lock() {
        session.get_mnemonic()
    } else {
        None
    }
}

/// Retrieve keypair from the global session
pub fn retrieve_keypair() -> Option<Keypair> {
    if let Ok(mut session) = GLOBAL_SESSION.lock() {
        session.get_keypair()
    } else {
        None
    }
}

/// Retrieve public key from the global session
pub fn retrieve_public_key() -> Option<PublicKey> {
    if let Ok(mut session) = GLOBAL_SESSION.lock() {
        session.get_public_key()
    } else {
        None
    }
}

/// Clear the global session
pub fn clear_session() {
    if let Ok(mut session) = GLOBAL_SESSION.lock() {
        session.clear();
        info!("Global session cleared");
    }
}

/// Check if the global session is active
pub fn is_session_active() -> bool {
    if let Ok(mut session) = GLOBAL_SESSION.lock() {
        session.is_active()
    } else {
        false
    }
}

/// Get current timestamp in seconds
fn get_current_timestamp() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        
        // Use JavaScript's Date.now() for web
        (js_sys::Date::now() / 1000.0) as u64 // Convert from milliseconds to seconds
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    
    #[test]
    fn test_session_timeout() {
        // Create a session with a very short timeout (1 second)
        let mut session = WalletSession::new(1);
        
        // Set a mnemonic
        session.set_mnemonic_and_keys("test mnemonic".to_string(), Keypair::from_bytes(&[0; 32]).unwrap());
        
        // Should be able to retrieve it immediately
        assert_eq!(session.get_mnemonic(), Some("test mnemonic".to_string()));
        
        // Wait for the session to expire
        sleep(Duration::from_secs(2));
        
        // Should return None after timeout
        assert_eq!(session.get_mnemonic(), None);
    }
    
    #[test]
    fn test_session_activity_extension() {
        // Create a session with a 2 second timeout
        let mut session = WalletSession::new(2);
        
        // Set a mnemonic
        session.set_mnemonic_and_keys("test mnemonic".to_string(), Keypair::from_bytes(&[0; 32]).unwrap());
        
        // Access it after 1 second (should extend the session)
        sleep(Duration::from_secs(1));
        assert_eq!(session.get_mnemonic(), Some("test mnemonic".to_string()));
        
        // Access it after another 1 second (should still be valid)
        sleep(Duration::from_secs(1));
        assert_eq!(session.get_mnemonic(), Some("test mnemonic".to_string()));
        
        // Wait for 2 seconds without access (should expire)
        sleep(Duration::from_secs(2));
        assert_eq!(session.get_mnemonic(), None);
    }
} 