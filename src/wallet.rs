use bip39::{Mnemonic, Language};
use serde::{Serialize, Deserialize};
use web_sys::{window, Storage};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha2::Sha512;
use solana_sdk::{
    derivation_path::DerivationPath,
    signature::{Keypair, keypair_from_seed_and_derivation_path, Signer},
};

// wallet config
#[derive(Serialize, Deserialize)]
pub struct WalletConfig {
    encrypted_keypair: String,  // store encrypted main private key
}

#[derive(Debug)]
pub enum WalletError {
    MnemonicGeneration,
    KeypairGeneration(String),
    Encryption(String),
    Storage(String),
}

// generate mnemonic
pub fn generate_mnemonic(word_count: u32) -> Result<String, WalletError> {
    let entropy_bytes = match word_count {
        12 => 16, // 128 bits
        24 => 32, // 256 bits
        _ => return Err(WalletError::MnemonicGeneration),
    };

    let mut entropy = vec![0u8; entropy_bytes];
    getrandom::getrandom(&mut entropy).map_err(|_| WalletError::MnemonicGeneration)?;
    
    // 新版本 bip39 的用法
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|_| WalletError::MnemonicGeneration)?;
    
    Ok(mnemonic.to_string())
}

// generate keypair from mnemonic
pub fn generate_keypair_from_mnemonic(
    mnemonic: &str, 
    passphrase: Option<&str>
) -> Result<(Keypair, String), WalletError> {
    // 1. parse mnemonic
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, mnemonic)
        .map_err(|_| WalletError::KeypairGeneration("Invalid mnemonic".to_string()))?;
    
    // 2. generate seed
    let salt = format!("mnemonic{}", passphrase.unwrap_or(""));
    let mut seed = [0u8; 64];
    pbkdf2::<Hmac<Sha512>>(
        mnemonic.to_string().as_bytes(),
        salt.as_bytes(),
        2048,
        &mut seed
    );

    // 3. generate keypair
    let path = "m/44'/501'/0'/0'";
    let derivation_path = DerivationPath::from_absolute_path_str(path)
        .map_err(|_| WalletError::KeypairGeneration("Invalid derivation path".to_string()))?;
    
    let keypair = keypair_from_seed_and_derivation_path(&seed, Some(derivation_path))
        .map_err(|_| WalletError::KeypairGeneration("Failed to derive keypair".to_string()))?;
    
    // 4. get pubkey address
    let pubkey = keypair.pubkey().to_string();

    Ok((keypair, pubkey))
}

// store encrypted main private key
pub async fn store_encrypted_keypair(
    keypair: &Keypair, 
    password: &str,
) -> Result<(), WalletError> {
    let keypair_bytes = keypair.to_bytes();
    let encrypted = crate::encrypt::encrypt(&hex::encode(keypair_bytes), password)
        .map_err(|e| WalletError::Encryption(e.to_string()))?;

    let config = WalletConfig {
        encrypted_keypair: encrypted,
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