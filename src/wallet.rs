use bip39::{Mnemonic, Language, MnemonicType};
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use web_sys::{window, Storage};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha2::Sha512;
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::str::FromStr;
use bip32::{XPrv, DerivationPath};
use rand::{RngCore, rngs::OsRng};
use bs58;

// 钱包配置
#[derive(Serialize, Deserialize)]
pub struct WalletConfig {
    encrypted_seed: String,  // 改为存储加密后的种子
}

#[derive(Debug)]
pub enum WalletError {
    MnemonicGeneration,
    SeedGeneration,
    Encryption(String),
    Storage(String),
    KeyDerivation,
}

// Generate a new mnemonic phrase
pub fn generate_mnemonic(word_count: u32) -> Result<String, WalletError> {
    let mtype = match word_count {
        12 => MnemonicType::Words12,
        15 => MnemonicType::Words15,
        18 => MnemonicType::Words18,
        21 => MnemonicType::Words21,
        24 => MnemonicType::Words24,
        _ => return Err(WalletError::MnemonicGeneration),
    };

    let mut entropy = vec![0u8; mtype.entropy_bits() / 8];
    OsRng.fill_bytes(&mut entropy);
    
    let mnemonic = Mnemonic::from_entropy(&entropy, Language::English)
        .map_err(|_| WalletError::MnemonicGeneration)?;
    
    Ok(mnemonic.phrase().to_string())
}

// 生成种子
pub fn generate_seed(mnemonic: &str, passphrase: Option<&str>) -> Result<[u8; 64], WalletError> {
    let mnemonic = Mnemonic::from_phrase(mnemonic, Language::English)
        .map_err(|_| WalletError::SeedGeneration)?;
    
    let salt = format!("mnemonic{}", passphrase.unwrap_or(""));
    let mut seed = [0u8; 64];
    
    pbkdf2::<Hmac<Sha512>>(
        mnemonic.phrase().as_bytes(),
        salt.as_bytes(),
        2048,
        &mut seed,
    );
    
    Ok(seed)
}

// Derive Solana address from seed
pub fn derive_solana_address(seed: &[u8; 64]) -> Result<String, WalletError> {
    // Solana's BIP44 path: m/44'/501'/0'/0'
    let path = DerivationPath::from_str("m/44'/501'/0'/0'")
        .map_err(|_| WalletError::KeyDerivation)?;
    
    let master_key = XPrv::new(seed)
        .map_err(|_| WalletError::KeyDerivation)?;
    
    // Derive each child key in sequence
    let mut derived_key = master_key;
    for child_number in path.into_iter() {
        derived_key = derived_key.derive_child(child_number)
            .map_err(|_| WalletError::KeyDerivation)?;
    }
    
    // Create signing key from the derived private key bytes
    let key_bytes: [u8; 32] = derived_key.to_bytes()[..32]
        .try_into()
        .map_err(|_| WalletError::KeyDerivation)?;
    
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let verifying_key = signing_key.verifying_key();
    
    // Convert the verifying key to Base58 string format
    Ok(bs58::encode(verifying_key.as_bytes()).into_string())
}

// 存储加密后的种子
pub async fn store_encrypted_seed(
    seed: &[u8; 64], 
    password: &str,
) -> Result<(), WalletError> {
    let encrypted = crate::encrypt::encrypt(&hex::encode(seed), password)
        .map_err(|e| WalletError::Encryption(e.to_string()))?;

    let config = WalletConfig {
        encrypted_seed: encrypted,
    };

    if let Some(window) = window() {
        let storage: Storage = window
            .local_storage()
            .map_err(|_| WalletError::Storage("Failed to get localStorage".to_string()))?
            .ok_or_else(|| WalletError::Storage("localStorage not available".to_string()))?;

        let json = serde_json::to_string(&config)
            .map_err(|e| WalletError::Storage(e.to_string()))?;
        
        storage.set_item("wallet_config", &json)
            .map_err(|_| WalletError::Storage("Failed to store data".to_string()))?;
    }

    Ok(())
}