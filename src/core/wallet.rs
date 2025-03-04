use bs58;
use bip39::{self, Mnemonic};
use rand::{rngs::OsRng, RngCore};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use crate::core::encrypt;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::signer::keypair::keypair_from_seed;

#[derive(Clone)]
pub struct Wallet {
    // Wallet public key (Solana address)
    pub address: String,
}

impl Wallet {
    // Create a new wallet from seed phrase
    pub fn new(seed_phrase: &str) -> Result<Self, String> {
        if seed_phrase.is_empty() {
            return Err("Empty seed phrase".to_string());
        }
        
        // Convert seed phrase to seed bytes
        let seed = Self::seed_to_bytes(seed_phrase)?;
        
        // Create keypair from seed
        let keypair = keypair_from_seed(&seed[..32])
            .map_err(|e| format!("Failed to create keypair: {}", e))?;
        
        // Get public key as Solana address
        let address = bs58::encode(keypair.pubkey().to_bytes()).into_string();
        
        Ok(Self {
            address,
        })
    }
    
    // Create an empty wallet with a custom address
    pub fn new_with_address(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }
    
    // Convert seed phrase to seed bytes
    fn seed_to_bytes(seed_phrase: &str) -> Result<Vec<u8>, String> {
        // Use BIP39 to convert mnemonic to seed
        let mnemonic = bip39::Mnemonic::parse_normalized(seed_phrase)
            .map_err(|e| format!("Invalid mnemonic: {}", e))?;
        
        // Generate seed with empty passphrase
        let seed = mnemonic.to_seed("");
        
        Ok(seed.to_vec())
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

    // Get keypair using password
    pub fn get_keypair_with_password(&self, password: &str) -> Result<Keypair, String> {
        // Load the encrypted wallet file
        let wallet_path = Self::get_wallet_file_path()?;
        
        // Read encrypted data from file
        let encrypted_data = fs::read_to_string(wallet_path)
            .map_err(|e| format!("Failed to read wallet file: {}", e))?;
        
        // Decrypt data to get seed phrase
        let seed_phrase = encrypt::decrypt(&encrypted_data, password)
            .map_err(|e| format!("Decryption error: {}", e))?;
        
        // Convert seed phrase to seed bytes
        let seed = Self::seed_to_bytes(&seed_phrase)?;
        
        // Create keypair from seed
        let keypair = keypair_from_seed(&seed[..32])
            .map_err(|e| format!("Failed to create keypair: {}", e))?;
        
        // Verify that the address matches
        let derived_address = bs58::encode(keypair.pubkey().to_bytes()).into_string();
        if derived_address != self.address {
            return Err("Address mismatch. The provided password may be incorrect.".to_string());
        }
        
        Ok(keypair)
    }
} 