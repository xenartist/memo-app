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

#[derive(Serialize, Deserialize)]
pub struct Wallet {
    encrypted_seed: String,
}

#[derive(Debug)]
pub enum WalletError {
    MnemonicGeneration,
    SeedGeneration,
    KeypairGeneration,
    Encryption,
    Storage,
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
    
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|_| WalletError::MnemonicGeneration)?;
    
    Ok(mnemonic.to_string())
}

// generate seed from mnemonic
pub fn generate_seed_from_mnemonic(
    mnemonic: &str, 
    passphrase: Option<&str>
) -> Result<[u8; 64], WalletError> {
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, mnemonic)
        .map_err(|_| WalletError::SeedGeneration)?;
    
    let salt = format!("mnemonic{}", passphrase.unwrap_or(""));
    let mut seed = [0u8; 64];
    pbkdf2::<Hmac<Sha512>>(
        mnemonic.to_string().as_bytes(),
        salt.as_bytes(),
        2048,
        &mut seed
    );

    Ok(seed)
}

// verify if a mnemonic phrase is valid
pub fn verify_mnemonic(mnemonic: &str) -> bool {
    // Try to parse the mnemonic using BIP39 English wordlist
    match Mnemonic::parse_in_normalized(Language::English, mnemonic) {
        Ok(_) => true,
        Err(_) => false,
    }
}

// derive keypair from seed
pub fn derive_keypair_from_seed(
    seed: &[u8; 64],
    path: &str,
) -> Result<(Keypair, String), WalletError> {
    let derivation_path = DerivationPath::from_absolute_path_str(path)
        .map_err(|_| WalletError::KeypairGeneration)?;
    
    let keypair = keypair_from_seed_and_derivation_path(seed, Some(derivation_path))
        .map_err(|_| WalletError::KeypairGeneration)?;
    
    let pubkey = keypair.pubkey().to_string();

    Ok((keypair, pubkey))
}

// store encrypted seed
pub async fn store_encrypted_seed(
    seed: &[u8; 64], 
    password: &str,
) -> Result<(), WalletError> {
    let encrypted = crate::core::encrypt::encrypt(&hex::encode(seed), password)
        .map_err(|_| WalletError::Encryption)?;

    let config = Wallet {
        encrypted_seed: encrypted,
    };

    if let Some(window) = window() {
        let storage: Storage = window
            .local_storage()
            .map_err(|_| WalletError::Storage)?
            .ok_or_else(|| WalletError::Storage)?;

        let json = serde_json::to_string(&config)
            .map_err(|_| WalletError::Storage)?;
        
        storage.set_item("wallet", &json)
            .map_err(|_| WalletError::Storage)?;
    }

    Ok(())
}

// get default solana derivation path
pub fn get_default_derivation_path() -> &'static str {
    "m/44'/501'/0'/0'"
}

impl Wallet {
    // get the encrypted seed
    pub fn get_encrypted_seed(&self) -> &str {
        &self.encrypted_seed
    }

    // check if wallet exists
    pub async fn exists() -> bool {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(_)) = storage.get_item("wallet") {
                    return true;
                }
            }
        }
        false
    }

    // load wallet from storage
    pub async fn load() -> Result<Self, WalletError> {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(json)) = storage.get_item("wallet") {
                    return serde_json::from_str(&json)
                        .map_err(|_| WalletError::Storage);
                }
            }
        }
        Err(WalletError::Storage)
    }

    // get encrypted seed from storage without loading the entire wallet
    pub async fn get_encrypted_seed_from_storage() -> Result<String, WalletError> {
        let wallet = Self::load().await?;
        Ok(wallet.encrypted_seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create test mnemonic
    fn create_test_mnemonic() -> String {
        // Use a known valid mnemonic for testing
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_string()
    }

    #[test]
    fn test_generate_mnemonic() {
        // Test 12 words mnemonic
        let mnemonic_12 = generate_mnemonic(12).unwrap();
        assert_eq!(mnemonic_12.split_whitespace().count(), 12);
        assert!(verify_mnemonic(&mnemonic_12));

        // Test 24 words mnemonic
        let mnemonic_24 = generate_mnemonic(24).unwrap();
        assert_eq!(mnemonic_24.split_whitespace().count(), 24);
        assert!(verify_mnemonic(&mnemonic_24));

        // Test invalid word count
        let result = generate_mnemonic(15);
        assert!(matches!(result, Err(WalletError::MnemonicGeneration)));
    }

    #[test]
    fn test_verify_mnemonic() {
        // Test valid mnemonic
        let valid_mnemonic = create_test_mnemonic();
        assert!(verify_mnemonic(&valid_mnemonic));

        // Test invalid mnemonic
        let invalid_mnemonic = "invalid mnemonic phrase test";
        assert!(!verify_mnemonic(invalid_mnemonic));

        // Test empty mnemonic
        assert!(!verify_mnemonic(""));

        // Test mnemonic with invalid words
        let invalid_words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon invalid";
        assert!(!verify_mnemonic(invalid_words));
    }

    #[test]
    fn test_generate_seed_from_mnemonic() {
        let mnemonic = create_test_mnemonic();

        // Test without passphrase
        let seed_result = generate_seed_from_mnemonic(&mnemonic, None);
        assert!(seed_result.is_ok());
        let seed = seed_result.unwrap();
        assert_eq!(seed.len(), 64);

        // Test with passphrase
        let seed_with_pass = generate_seed_from_mnemonic(&mnemonic, Some("passphrase")).unwrap();
        assert_ne!(seed, seed_with_pass); // Seeds should be different with different passphrases

        // Test invalid mnemonic
        let result = generate_seed_from_mnemonic("invalid mnemonic", None);
        assert!(matches!(result, Err(WalletError::SeedGeneration(_))));
    }

    #[test]
    fn test_derive_keypair_from_seed() {
        // Create a known seed
        let seed = [1u8; 64];

        // Test with default derivation path
        let path = get_default_derivation_path();
        let result = derive_keypair_from_seed(&seed, path);
        assert!(result.is_ok());
        let (keypair, pubkey) = result.unwrap();
        assert_eq!(keypair.pubkey().to_string(), pubkey);

        // Test with invalid derivation path
        let invalid_path = "m/44'/0'/0'/0/";
        let result = derive_keypair_from_seed(&seed, invalid_path);
        assert!(matches!(result, Err(WalletError::KeypairGeneration(_))));

        // Test with different seeds produce different keypairs
        let seed2 = [2u8; 64];
        let (keypair2, pubkey2) = derive_keypair_from_seed(&seed2, path).unwrap();
        assert_ne!(keypair.pubkey(), keypair2.pubkey());
        assert_ne!(pubkey, pubkey2);
    }

    #[test]
    fn test_default_derivation_path() {
        let path = get_default_derivation_path();
        assert_eq!(path, "m/44'/501'/0'/0'");
    }

    #[test]
    fn test_wallet_struct() {
        let encrypted_seed = "test_encrypted_seed".to_string();
        let wallet = Wallet { encrypted_seed: encrypted_seed.clone() };
        
        assert_eq!(wallet.get_encrypted_seed(), encrypted_seed);
    }

    #[test]
    fn test_wallet_error_variants() {
        // Test error variants construction
        let mnemonic_err = WalletError::MnemonicGeneration;
        let seed_err = WalletError::SeedGeneration;
        let keypair_err = WalletError::KeypairGeneration;
        let encryption_err = WalletError::Encryption;
        let storage_err = WalletError::Storage;

        // Verify each error can be matched
        assert!(matches!(mnemonic_err, WalletError::MnemonicGeneration));
        assert!(matches!(seed_err, WalletError::SeedGeneration));
        assert!(matches!(keypair_err, WalletError::KeypairGeneration));
        assert!(matches!(encryption_err, WalletError::Encryption));
        assert!(matches!(storage_err, WalletError::Storage));
    }

    #[test]
    fn test_mnemonic_to_keypair_flow() {
        // Test the complete flow from mnemonic to keypair
        let mnemonic = create_test_mnemonic();
        
        // Generate seed
        let seed = generate_seed_from_mnemonic(&mnemonic, None).unwrap();
        
        // Derive keypair
        let path = get_default_derivation_path();
        let (keypair, pubkey) = derive_keypair_from_seed(&seed, path).unwrap();
        
        // Verify the keypair matches the pubkey
        assert_eq!(keypair.pubkey().to_string(), pubkey);
    }

    #[test]
    fn test_seed_consistency() {
        // Test that the same mnemonic + passphrase always generates the same seed
        let mnemonic = create_test_mnemonic();
        let passphrase = Some("test_passphrase");

        let seed1 = generate_seed_from_mnemonic(&mnemonic, passphrase).unwrap();
        let seed2 = generate_seed_from_mnemonic(&mnemonic, passphrase).unwrap();

        assert_eq!(seed1, seed2);

        // Test different passphrases generate different seeds
        let seed3 = generate_seed_from_mnemonic(&mnemonic, Some("different_passphrase")).unwrap();
        assert_ne!(seed1, seed3);
    }

    #[test]
    fn test_wallet_error_handling() {
        // Test error handling in the mnemonic generation flow
        let invalid_word_count = generate_mnemonic(16);
        assert!(matches!(invalid_word_count, Err(WalletError::MnemonicGeneration)));

        // Test error handling in the seed generation flow
        let invalid_mnemonic = "invalid mnemonic";
        let seed_result = generate_seed_from_mnemonic(invalid_mnemonic, None);
        assert!(matches!(seed_result, Err(WalletError::SeedGeneration)));

        // Test error handling in the keypair derivation flow
        let seed = [0u8; 64];
        let invalid_path = "invalid/path";
        let keypair_result = derive_keypair_from_seed(&seed, invalid_path);
        assert!(matches!(keypair_result, Err(WalletError::KeypairGeneration)));
    }
}