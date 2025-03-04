use hmac::{Hmac, Mac};
use sha2::Sha512;
use ed25519_dalek::SigningKey;
use bs58;
use bip39::{self, Mnemonic};
use rand::{rngs::OsRng, RngCore};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use crate::core::encrypt;

type HmacSha512 = Hmac<Sha512>;

pub struct Wallet {
    // Wallet public key (Solana address)
    pub address: String,
    // Seed phrase (stored temporarily)
    seed_phrase: String,
}

impl Wallet {
    // Create a new wallet from seed phrase
    pub fn new(seed_phrase: &str) -> Result<Self, String> {
        let mut wallet = Self {
            address: String::new(),
            seed_phrase: seed_phrase.to_string(),
        };
        
        // Generate wallet address from seed
        wallet.generate_wallet_address()?;
        
        Ok(wallet)
    }
    
    // Create an empty wallet with a custom address
    pub fn new_with_address(address: &str) -> Self {
        Self {
            address: address.to_string(),
            seed_phrase: String::new(),
        }
    }
    
    // Generate Solana wallet address from seed phrase
    fn generate_wallet_address(&mut self) -> Result<(), String> {
        if self.seed_phrase.is_empty() {
            return Err("Empty seed phrase".to_string());
        }
        
        // Derive the private key using BIP44 for Solana (m/44'/501'/0'/0')
        let private_key = self.derive_private_key()?;
        
        // Convert private key to public key
        let signing_key = SigningKey::from_bytes(&private_key);
        let public_key = signing_key.verifying_key();
        
        // Convert public key to Solana address (base58 encoding of public key bytes)
        let address = bs58::encode(public_key.as_bytes()).into_string();
        self.address = address;
        
        Ok(())
    }
    
    // Derive private key using BIP44 for Solana (m/44'/501'/0'/0')
    pub fn derive_private_key(&self) -> Result<[u8; 32], String> {
        // Convert seed phrase to seed bytes
        let seed = self.seed_to_bytes()?;
        
        // BIP44 path for Solana: m/44'/501'/0'/0'
        let path = "m/44'/501'/0'/0'";
        
        // Derive master key
        let (master_key, chain_code) = self.derive_master_key(&seed)?;
        
        // Derive child keys according to path
        let mut key = master_key;
        let mut code = chain_code;
        
        // Parse path and derive each level
        let path_components: Vec<&str> = path.split('/').collect();
        for &component in path_components.iter().skip(1) { // Skip 'm'
            let hardened = component.ends_with('\'');
            let index_str = if hardened {
                &component[0..component.len()-1]
            } else {
                component
            };
            
            let mut index = index_str.parse::<u32>().map_err(|_| "Invalid path component".to_string())?;
            if hardened {
                index += 0x80000000; // Hardened key
            }
            
            // Derive child key
            let (child_key, child_code) = self.derive_child_key(&key, &code, index)?;
            key = child_key;
            code = child_code;
        }
        
        // Create 32-byte array for the key
        let mut result = [0u8; 32];
        result.copy_from_slice(&key[0..32]);
        
        Ok(result)
    }
    
    // Convert seed phrase to seed bytes
    fn seed_to_bytes(&self) -> Result<Vec<u8>, String> {
        // Use BIP39 to convert mnemonic to seed
        let mnemonic = bip39::Mnemonic::parse_normalized(&self.seed_phrase)
            .map_err(|e| format!("Invalid mnemonic: {}", e))?;
        
        // Generate seed with empty passphrase
        let seed = mnemonic.to_seed("");
        
        Ok(seed.to_vec())
    }
    
    // Derive master key from seed
    fn derive_master_key(&self, seed: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        // HMAC-SHA512 with key "ed25519 seed"
        let mut mac = HmacSha512::new_from_slice(b"ed25519 seed")
            .map_err(|_| "Failed to create HMAC".to_string())?;
        
        mac.update(seed);
        let result = mac.finalize().into_bytes();
        
        // Split result into key and chain code
        let key = result[0..32].to_vec();
        let chain_code = result[32..64].to_vec();
        
        Ok((key, chain_code))
    }
    
    // Derive child key
    fn derive_child_key(&self, key: &[u8], chain_code: &[u8], index: u32) -> Result<(Vec<u8>, Vec<u8>), String> {
        let mut mac = HmacSha512::new_from_slice(chain_code)
            .map_err(|_| "Failed to create HMAC".to_string())?;
        
        // For hardened keys, use 0x00 || key || index
        // For normal keys, use public_key || index
        if index >= 0x80000000 {
            mac.update(&[0x00]);
            mac.update(key);
        } else {
            // For normal derivation, we would use the public key, but Solana uses hardened derivation
            // This branch shouldn't be reached for Solana wallets
            return Err("Non-hardened derivation not supported for Solana".to_string());
        }
        
        // Append index in big-endian
        mac.update(&[(index >> 24) as u8, (index >> 16) as u8, (index >> 8) as u8, index as u8]);
        
        let result = mac.finalize().into_bytes();
        
        // Split result into key and chain code
        let derived_key = result[0..32].to_vec();
        let derived_chain_code = result[32..64].to_vec();
        
        Ok((derived_key, derived_chain_code))
    }
    
    // Format wallet address with mask (e.g., "abcd****wxyz")
    pub fn format_masked_address(&self) -> String {
        if self.address.len() < 8 {
            return self.address.clone();
        }
        
        let prefix = &self.address[..4];
        let suffix = &self.address[self.address.len() - 4..];
        format!("{}****{}", prefix, suffix)
    }
    
    // Get the signing key from the wallet
    pub fn get_signing_key(&self) -> Result<SigningKey, String> {
        let private_key = self.derive_private_key()?;
        Ok(SigningKey::from_bytes(&private_key))
    }
    
    // Get the seed phrase
    pub fn get_seed_phrase(&self) -> &str {
        &self.seed_phrase
    }
    
    // Generate new BIP39 mnemonic seed words
    pub fn generate_seed_words(word_count: usize) -> Result<Vec<String>, String> {
        // Determine entropy size based on word count
        // 16 bytes (128 bits) for 12 words, 32 bytes (256 bits) for 24 words
        let entropy_size = match word_count {
            12 => 16,
            24 => 32,
            _ => return Err(format!("Unsupported word count: {}", word_count)),
        };
        
        // Generate random entropy
        let mut entropy = vec![0u8; entropy_size];
        OsRng.fill_bytes(&mut entropy);
        
        // Create mnemonic from entropy
        let mnemonic = Mnemonic::from_entropy(&entropy)
            .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;
        
        // Get the phrase as a string and split into words
        let phrase = mnemonic.to_string();
        let seed_words = phrase.split_whitespace().map(String::from).collect();
        
        Ok(seed_words)
    }
    
    // Validate a mnemonic phrase
    pub fn validate_mnemonic(seed_words: &[String]) -> Result<String, String> {
        // Join the seed words
        let phrase = seed_words.join(" ");
        
        // Check if any words are empty
        if seed_words.iter().any(|word| word.trim().is_empty()) {
            return Err("All seed words must be filled".to_string());
        }
        
        // Validate the mnemonic
        match Mnemonic::parse_normalized(&phrase) {
            Ok(_) => Ok(phrase),
            Err(e) => Err(format!("Invalid mnemonic: {}", e)),
        }
    }
    
    // Get the wallet file path
    pub fn get_wallet_file_path() -> Result<PathBuf, String> {
        // Get the executable directory
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;
        let exe_dir = exe_path.parent()
            .ok_or_else(|| "Failed to get executable directory".to_string())?;
        
        // Create wallets directory path
        let wallets_dir = exe_dir.join("wallets");
        
        // Return wallet file path
        Ok(wallets_dir.join("memo-encrypted.wallet"))
    }
    
    // Check if wallet file exists
    pub fn wallet_file_exists() -> bool {
        if let Ok(wallet_path) = Self::get_wallet_file_path() {
            wallet_path.exists()
        } else {
            false
        }
    }
    
    // Save wallet to file
    pub fn save_wallet(seed_phrase: &str, password: &str) -> Result<(), String> {
        // Encrypt the seed phrase
        let encrypted_data = encrypt::encrypt(seed_phrase, password)
            .map_err(|e| format!("Encryption error: {}", e))?;
        
        // Get wallet file path
        let wallet_path = Self::get_wallet_file_path()?;
        
        // Create wallets directory if it doesn't exist
        if let Some(parent) = wallet_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create wallets directory: {}", e))?;
        }
        
        // Create wallet file
        let mut file = File::create(&wallet_path)
            .map_err(|e| format!("Failed to create wallet file: {}", e))?;
        
        // Write encrypted data to file
        file.write_all(encrypted_data.as_bytes())
            .map_err(|e| format!("Failed to write to wallet file: {}", e))?;
        
        Ok(())
    }
    
    // Load wallet from file
    pub fn load_wallet(password: &str) -> Result<Self, String> {
        // Get wallet file path
        let wallet_path = Self::get_wallet_file_path()?;
        
        // Read encrypted data from file
        let encrypted_data = fs::read_to_string(wallet_path)
            .map_err(|e| format!("Failed to read wallet file: {}", e))?;
        
        // Decrypt data
        let seed_phrase = encrypt::decrypt(&encrypted_data, password)
            .map_err(|e| format!("Decryption error: {}", e))?;
        
        // Create wallet from seed phrase
        Self::new(&seed_phrase)
    }
} 