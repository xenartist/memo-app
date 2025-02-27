use bip39::{Mnemonic, Language};
use rand::rngs::OsRng;
use log::info;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_mnemonic() {
        let mnemonic = generate_mnemonic();
        assert_eq!(mnemonic.split_whitespace().count(), 12);
        assert!(validate_mnemonic(&mnemonic).is_ok());
    }
} 