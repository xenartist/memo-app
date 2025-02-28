use bip39::{Mnemonic, Language};
use rand::rngs::OsRng;
use log::{info, debug};
use crate::storage;
use crate::session;

/// Generates a new BIP39 mnemonic phrase (12 words)
pub fn generate_mnemonic() -> String {
    // create a new random mnemonic phrase (12 words)
    let mut entropy = [0u8; 16]; // 16 bytes = 128 bits = 12 words
    getrandom::getrandom(&mut entropy).expect("Failed to generate random entropy");
    
    // create mnemonic from entropy
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("Failed to create mnemonic");
    
    // convert to string and return
    let phrase = mnemonic.to_string();
    info!("Generated new wallet mnemonic");
    
    phrase
}

/// Validates if a mnemonic phrase is valid
pub fn validate_mnemonic(phrase: &str) -> Result<(), String> {
    match Mnemonic::parse_normalized(phrase) {
        Ok(_) => Ok(()),
        Err(_) => Err("Invalid mnemonic phrase. Please check that you've entered 12 valid words.".to_string()),
    }
}

/// Simple transaction structure for demonstration
#[derive(Debug, Clone)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub memo: Option<String>,
}

/// Signed transaction with signature
#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: String,
    pub public_key: String,
}

/// Sign a transaction using the wallet
pub fn sign_transaction(transaction: &Transaction, password: Option<&str>) -> Result<SignedTransaction, String> {
    // Load the wallet
    let wallet = match storage::load_wallet()? {
        Some(wallet) => wallet,
        None => return Err("No wallet found".to_string()),
    };
    
    // Get the mnemonic (from session if available, otherwise decrypt with password)
    let mnemonic = storage::get_mnemonic_for_signing(&wallet, password)?;
    
    // Derive the private key from mnemonic
    let private_key = derive_private_key_from_mnemonic(&mnemonic)?;
    
    // Sign the transaction (simplified for demonstration)
    let signature = sign_data_with_private_key(&private_key, &format!("{:?}", transaction))?;
    
    // Get the public key
    let public_key = get_public_key_from_private_key(&private_key)?;
    
    Ok(SignedTransaction {
        transaction: transaction.clone(),
        signature,
        public_key,
    })
}

/// Derive private key from mnemonic
fn derive_private_key_from_mnemonic(mnemonic: &str) -> Result<Vec<u8>, String> {
    // This is a simplified version - in a real implementation, you would use proper key derivation
    debug!("Deriving private key from mnemonic");
    
    // Parse the mnemonic
    let mnemonic = Mnemonic::parse_normalized(mnemonic)
        .map_err(|_| "Invalid mnemonic".to_string())?;
    
    // Get the seed
    let seed = mnemonic.to_seed("");
    
    // For demonstration, just return the first 32 bytes of the seed as the private key
    let mut private_key = Vec::new();
    private_key.extend_from_slice(&seed[0..32]);
    
    Ok(private_key)
}

/// Sign data with private key (simplified)
fn sign_data_with_private_key(private_key: &[u8], data: &str) -> Result<String, String> {
    use ed25519_dalek::{Keypair, SecretKey, PublicKey, Signature, Signer};
    use sha2::{Sha512, Digest};
    
    debug!("Signing data with private key");
    
    // Create a secret key from the private key bytes
    let secret = SecretKey::from_bytes(private_key)
        .map_err(|_| "Invalid private key".to_string())?;
    
    // Derive the public key
    let public = PublicKey::from(&secret);
    
    // Create a keypair
    let keypair = Keypair {
        secret,
        public,
    };
    
    // Hash the data
    let mut hasher = Sha512::new();
    hasher.update(data.as_bytes());
    let message = &hasher.finalize()[..];
    
    // Sign the message - using the correct method for ed25519-dalek 1.0.1
    let signature = keypair.sign(message);
    
    // Return the signature as a hex string
    Ok(hex::encode(signature.to_bytes()))
}

/// Get public key from private key
fn get_public_key_from_private_key(private_key: &[u8]) -> Result<String, String> {
    use ed25519_dalek::{SecretKey, PublicKey};
    
    // Create a secret key from the private key bytes
    let secret = SecretKey::from_bytes(private_key)
        .map_err(|_| "Invalid private key".to_string())?;
    
    // Derive the public key
    let public = PublicKey::from(&secret);
    
    // Return the public key as a hex string
    Ok(hex::encode(public.to_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_mnemonic() {
        let mnemonic = generate_mnemonic();
        assert_eq!(mnemonic.split_whitespace().count(), 12);
        assert!(validate_mnemonic(&mnemonic).is_ok());
    }
    
    #[test]
    fn test_sign_transaction() {
        // This test requires a wallet to be set up, so it's more of an integration test
        // In a real application, you would mock the dependencies
    }
} 