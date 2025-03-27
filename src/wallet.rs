use bip39::{Mnemonic, MnemonicType, Language};
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use web_sys::{window, Storage};
use wasm_bindgen::JsValue;

// wallet config
#[derive(Serialize, Deserialize)]
pub struct WalletConfig {
    encrypted_mnemonic: String,
    // can add other config, like path etc.
}

#[derive(Debug)]
pub enum WalletError {
    MnemonicGeneration,
    Encryption(String),
    Storage(String),
}

// generate new mnemonic
pub fn generate_mnemonic(word_count: u32) -> Result<String, WalletError> {
    let mnemonic_type = match word_count {
        12 => MnemonicType::Words12,
        24 => MnemonicType::Words24,
        _ => return Err(WalletError::MnemonicGeneration),
    };

    let mnemonic = Mnemonic::new(mnemonic_type, Language::English);
    Ok(mnemonic.phrase().to_string())
}

// verify mnemonic
pub fn verify_mnemonic(mnemonic: &str) -> bool {
    Mnemonic::from_phrase(mnemonic, Language::English).is_ok()
}

// store encrypted mnemonic
pub async fn store_encrypted_mnemonic(
    mnemonic: &str, 
    password: &str,
) -> Result<(), WalletError> {
    let encrypted = crate::encrypt::encrypt(mnemonic, password)
        .map_err(|e| WalletError::Encryption(e.to_string()))?;

    let config = WalletConfig {
        encrypted_mnemonic: encrypted,
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

pub enum StorageType {
    Browser,
    Desktop,
    Mobile,
} 