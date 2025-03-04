use hmac::{Hmac, Mac};
use sha2::Sha512;
use ed25519_dalek::SigningKey;
use bs58;
use bip39;

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
} 