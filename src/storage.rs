use serde::{Deserialize, Serialize};
use log::info;

// wallet data structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletData {
    pub mnemonic: String,
    pub address: String,
    pub created_at: u64, // Unix timestamp
}

// storage interface
pub trait Storage {
    fn save_wallet(&self, wallet: &WalletData) -> Result<(), String>;
    fn load_wallet(&self) -> Result<Option<WalletData>, String>;
}

// Web storage implementation
#[cfg(target_arch = "wasm32")]
pub mod web_storage {
    use super::*;
    use wasm_bindgen::prelude::*;
    use web_sys::{window, Storage as WebStorage};
    
    const WALLET_KEY: &str = "wallet_data";
    
    pub struct WebLocalStorage;
    
    impl WebLocalStorage {
        pub fn new() -> Self {
            Self {}
        }
        
        fn get_storage(&self) -> Result<WebStorage, String> {
            window()
                .ok_or_else(|| "No window object found".to_string())?
                .local_storage()
                .map_err(|e| format!("Failed to access localStorage: {:?}", e))?
                .ok_or_else(|| "localStorage not available".to_string())
        }
    }
    
    impl Storage for WebLocalStorage {
        fn save_wallet(&self, wallet: &WalletData) -> Result<(), String> {
            let storage = self.get_storage()?;
            
            let json = serde_json::to_string(wallet)
                .map_err(|e| format!("Failed to serialize wallet: {}", e))?;
                
            storage.set_item(WALLET_KEY, &json)
                .map_err(|e| format!("Failed to save to localStorage: {:?}", e))?;
                
            info!("Wallet saved to browser localStorage");
            Ok(())
        }
        
        fn load_wallet(&self) -> Result<Option<WalletData>, String> {
            let storage = self.get_storage()?;
            
            let json = match storage.get_item(WALLET_KEY)
                .map_err(|e| format!("Failed to read from localStorage: {:?}", e))? {
                Some(data) => data,
                None => return Ok(None),
            };
            
            let wallet = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize wallet: {}", e))?;
                
            info!("Wallet loaded from browser localStorage");
            Ok(Some(wallet))
        }
    }
}

// desktop/mobile storage implementation
#[cfg(not(target_arch = "wasm32"))]
pub mod file_storage {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    
    pub struct FileStorage {
        file_path: PathBuf,
    }
    
    impl FileStorage {
        pub fn new() -> Self {
            let mut path = dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."));
                
            path.push("my_cross_platform_app");
            fs::create_dir_all(&path).ok();
            path.push("wallet.json");
            
            Self { file_path: path }
        }
    }
    
    impl Storage for FileStorage {
        fn save_wallet(&self, wallet: &WalletData) -> Result<(), String> {
            let json = serde_json::to_string_pretty(wallet)
                .map_err(|e| format!("Failed to serialize wallet: {}", e))?;
                
            fs::write(&self.file_path, json)
                .map_err(|e| format!("Failed to write wallet file: {}", e))?;
                
            info!("Wallet saved to file: {:?}", self.file_path);
            Ok(())
        }
        
        fn load_wallet(&self) -> Result<Option<WalletData>, String> {
            if !self.file_path.exists() {
                return Ok(None);
            }
            
            let json = fs::read_to_string(&self.file_path)
                .map_err(|e| format!("Failed to read wallet file: {}", e))?;
                
            let wallet = serde_json::from_str(&json)
                .map_err(|e| format!("Failed to deserialize wallet: {}", e))?;
                
            info!("Wallet loaded from file: {:?}", self.file_path);
            Ok(Some(wallet))
        }
    }
}

// factory function, returns appropriate storage implementation based on platform
pub fn get_storage() -> Box<dyn Storage> {
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(web_storage::WebLocalStorage::new())
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::new(file_storage::FileStorage::new())
    }
}

// create new wallet and save it
pub fn create_and_save_wallet(mnemonic: String) -> Result<WalletData, String> {
    // Generate a wallet address from the mnemonic
    let address = generate_solana_address_from_mnemonic(&mnemonic);
    
    // Get current timestamp in a way that works on both native and web platforms
    let now = get_current_timestamp();
        
    let wallet = WalletData {
        mnemonic,
        address,
        created_at: now,
    };
    
    let storage = get_storage();
    storage.save_wallet(&wallet)?;
    
    Ok(wallet)
}

// Get current timestamp in a cross-platform way
fn get_current_timestamp() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        
        // Use JavaScript's Date.now() for web
        let timestamp = js_sys::Date::now() as u64 / 1000; // Convert from milliseconds to seconds
        timestamp
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        // Use Rust's SystemTime for native platforms
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

// Load wallet from storage
pub fn load_wallet() -> Result<Option<WalletData>, String> {
    let storage = get_storage();
    storage.load_wallet()
}

// Generate a Solana-style wallet address from mnemonic
// This is a simplified implementation for demo purposes
fn generate_solana_address_from_mnemonic(mnemonic: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // Create multiple hashes to get more bytes
    let mut bytes = Vec::with_capacity(32);
    
    // Use different seeds to generate different hashes
    for i in 0..4 {
        let mut hasher = DefaultHasher::new();
        mnemonic.hash(&mut hasher);
        i.hash(&mut hasher); // Add a seed to get different hashes
        let hash = hasher.finish();
        let hash_bytes = hash.to_be_bytes();
        bytes.extend_from_slice(&hash_bytes);
    }
    
    // Ensure we have exactly 32 bytes (Solana public key size)
    bytes.truncate(32);
    
    // Use base58 encoding (Solana addresses are base58 encoded)
    // In a real app, you would use proper Ed25519 key derivation
    bs58::encode(bytes).into_string()
} 