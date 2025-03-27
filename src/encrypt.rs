use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, Params, Version,
};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};

use std::fmt;

// Error types
#[derive(Debug)]
pub enum EncryptError {
    Argon2Error(String),
    ChaChaError(String),
    InvalidData,
}

impl fmt::Display for EncryptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncryptError::Argon2Error(e) => write!(f, "Argon2 error: {}", e),
            EncryptError::ChaChaError(e) => write!(f, "ChaCha20Poly1305 error: {}", e),
            EncryptError::InvalidData => write!(f, "Invalid encrypted data format"),
        }
    }
}

impl std::error::Error for EncryptError {}

// Derive encryption key from password
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], EncryptError> {
    // Use Argon2id algorithm to derive the key
    let argon2 = Argon2::new_with_secret(
        &[],
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        Params::new(
            // These parameters can be adjusted based on security requirements and performance
            32 * 1024, // Memory cost
            3,         // Iterations
            1,         // Parallelism
            Some(32),  // Output length (32 bytes = 256 bits)
        )
        .map_err(|e| EncryptError::Argon2Error(e.to_string()))?,
    )
    .map_err(|e| EncryptError::Argon2Error(e.to_string()))?;

    // Create salt
    let salt = SaltString::encode_b64(salt)
        .map_err(|e| EncryptError::Argon2Error(e.to_string()))?;

    // Derive key
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| EncryptError::Argon2Error(e.to_string()))?;

    // Get hash value
    let hash = password_hash.hash.ok_or_else(|| {
        EncryptError::Argon2Error("Failed to get hash from password hash".to_string())
    })?;

    // Convert to 32-byte array
    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_bytes());

    Ok(key)
}

// Encrypt data
pub fn encrypt(data: &str, password: &str) -> Result<String, EncryptError> {
    // Generate random salt
    let salt = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    // Derive key from password
    let key = derive_key(password, salt.as_slice())?;

    // Create ChaCha20Poly1305 instance
    let cipher = ChaCha20Poly1305::new(&key.into());

    // Generate random nonce
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    // Encrypt data
    let ciphertext = cipher
        .encrypt(&nonce, data.as_bytes())
        .map_err(|e| EncryptError::ChaChaError(e.to_string()))?;

    // Combine salt, nonce, and ciphertext into a string
    // Format: hex(salt) + ":" + hex(nonce) + ":" + hex(ciphertext)
    let result = format!(
        "{}:{}:{}",
        hex::encode(salt),
        hex::encode(nonce),
        hex::encode(ciphertext)
    );

    Ok(result)
}

// Decrypt data
pub fn decrypt(encrypted_data: &str, password: &str) -> Result<String, EncryptError> {
    // Parse encrypted data
    let parts: Vec<&str> = encrypted_data.split(':').collect();
    if parts.len() != 3 {
        return Err(EncryptError::InvalidData);
    }

    // Parse salt, nonce, and ciphertext
    let salt = hex::decode(parts[0]).map_err(|_| EncryptError::InvalidData)?;
    let nonce_bytes = hex::decode(parts[1]).map_err(|_| EncryptError::InvalidData)?;
    let ciphertext = hex::decode(parts[2]).map_err(|_| EncryptError::InvalidData)?;

    // Derive key from password
    let key = derive_key(password, &salt)?;

    // Create ChaCha20Poly1305 instance
    let cipher = ChaCha20Poly1305::new(&key.into());

    // Create nonce
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Decrypt data
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| EncryptError::ChaChaError(e.to_string()))?;

    // Convert plaintext to string
    let result = String::from_utf8(plaintext).map_err(|_| EncryptError::InvalidData)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let data = "this is a test";
        let password = "password123";

        let encrypted = encrypt(data, password).unwrap();
        let decrypted = decrypt(&encrypted, password).unwrap();

        assert_eq!(data, decrypted);
    }

    #[test]
    fn test_wrong_password() {
        let data = "this is a test";
        let password = "password123";
        let wrong_password = "wrong_password";

        let encrypted = encrypt(data, password).unwrap();
        let result = decrypt(&encrypted, wrong_password);

        assert!(result.is_err());
    }
} 