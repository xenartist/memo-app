use serde::{Deserialize, Serialize};
use log::info;
use crate::encrypt;

// wallet data structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletData {
    pub encrypted_mnemonic: String,  // Encrypted mnemonic phrase
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

// create new wallet and save it with encryption
pub fn create_and_save_wallet(mnemonic: String, password: &str) -> Result<WalletData, String> {
    // 加密助记词
    let encrypted_mnemonic = encrypt::encrypt(&mnemonic, password)
        .map_err(|e| format!("Failed to encrypt mnemonic: {}", e))?;
    
    // Generate a wallet address from the mnemonic
    let address = generate_solana_address_from_mnemonic(&mnemonic);
    
    // Get current timestamp in a way that works on both native and web platforms
    let now = get_current_timestamp();
        
    let wallet = WalletData {
        encrypted_mnemonic,
        address,
        created_at: now,
    };
    
    let storage = get_storage();
    storage.save_wallet(&wallet)?;
    
    Ok(wallet)
}

// Decrypt mnemonic phrase
pub fn decrypt_mnemonic(wallet: &WalletData, password: &str) -> Result<String, String> {
    encrypt::decrypt(&wallet.encrypted_mnemonic, password)
        .map_err(|e| format!("Failed to decrypt mnemonic: {}", e))
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
// Using standard BIP44 derivation path: m/44'/501'/0'/0'
fn generate_solana_address_from_mnemonic(mnemonic: &str) -> String {
    use bip39::{Mnemonic, Language};
    use sha2::{Sha512, Digest};
    use hmac::{Hmac, Mac};
    use pbkdf2::pbkdf2;
    use ed25519_dalek::{PublicKey, SecretKey};
    
    type HmacSha512 = Hmac<Sha512>;
    
    // Parse the mnemonic
    let mnemonic = Mnemonic::parse_normalized(mnemonic)
        .expect("Invalid mnemonic");
    
    // Get the seed from the mnemonic (with empty passphrase)
    let seed = mnemonic.to_seed("");
    
    // BIP44 path for Solana: m/44'/501'/0'/0'
    let path = "m/44'/501'/0'/0'";
    
    // Derive the key using BIP32/BIP44
    let derived_key = derive_key_from_path(&seed, path)
        .expect("Failed to derive key");
    
    // Get the public key from the derived private key
    let secret_key = SecretKey::from_bytes(&derived_key)
        .expect("Invalid secret key");
    let public_key = PublicKey::from(&secret_key);
    
    // Encode the public key in base58 (Solana format)
    let address = bs58::encode(public_key.as_bytes()).into_string();
    
    log::info!("Generated Solana address using BIP44 path: {}", path);
    address
}

// Helper function to derive a key from a seed using a BIP32/BIP44 path
fn derive_key_from_path(seed: &[u8], path: &str) -> Result<[u8; 32], String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha512;
    
    type HmacSha512 = Hmac<Sha512>;
    
    // Parse the path
    let path_components = parse_derivation_path(path)?;
    
    // Start with the master key
    let mut key = [0u8; 64]; // 512 bits
    
    // Derive the master key from the seed
    let mut hmac = HmacSha512::new_from_slice(b"ed25519 seed")
        .map_err(|_| "HMAC error".to_string())?;
    hmac.update(seed);
    let result = hmac.finalize().into_bytes();
    key.copy_from_slice(&result[..64]);
    
    // Derive child keys according to the path
    for component in path_components {
        // Prepare the data for HMAC
        let mut data = vec![0u8]; // Version byte
        data.extend_from_slice(&key[32..64]); // Chain code
        
        // Add the index with hardened bit if needed
        let mut index_bytes = [0u8; 4];
        let index = if component.hardened {
            component.index | 0x80000000
        } else {
            component.index
        };
        index_bytes[0] = ((index >> 24) & 0xFF) as u8;
        index_bytes[1] = ((index >> 16) & 0xFF) as u8;
        index_bytes[2] = ((index >> 8) & 0xFF) as u8;
        index_bytes[3] = (index & 0xFF) as u8;
        data.extend_from_slice(&index_bytes);
        
        // Compute the HMAC
        let mut hmac = HmacSha512::new_from_slice(&key[..32])
            .map_err(|_| "HMAC error".to_string())?;
        hmac.update(&data);
        let result = hmac.finalize().into_bytes();
        
        // Update the key
        key.copy_from_slice(&result[..64]);
    }
    
    // Return the first 32 bytes as the private key
    let mut private_key = [0u8; 32];
    private_key.copy_from_slice(&key[..32]);
    Ok(private_key)
}

// Helper struct for path components
struct PathComponent {
    index: u32,
    hardened: bool,
}

// Parse a BIP32/BIP44 derivation path
fn parse_derivation_path(path: &str) -> Result<Vec<PathComponent>, String> {
    let mut components = Vec::new();
    
    // Split the path by '/'
    let parts: Vec<&str> = path.split('/').collect();
    
    // Check if the path starts with 'm'
    if parts.is_empty() || parts[0] != "m" {
        return Err("Path must start with 'm'".to_string());
    }
    
    // Parse each component
    for part in &parts[1..] {
        let hardened = part.ends_with('\'');
        let index_str = if hardened {
            &part[..part.len() - 1]
        } else {
            part
        };
        
        let index = index_str.parse::<u32>()
            .map_err(|_| format!("Invalid path component: {}", part))?;
        
        components.push(PathComponent {
            index,
            hardened,
        });
    }
    
    Ok(components)
} 