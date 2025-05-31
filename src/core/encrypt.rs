use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, Params, Version,
};
use chacha20poly1305::{
    aead::{Aead, NewAead, generic_array::GenericArray},
    ChaCha20Poly1305,
};
use zeroize::{Zeroize, Zeroizing};
use secrecy::{Secret, ExposeSecret};
use rand::{rngs::OsRng, RngCore};

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
    let mut salt = [0u8; 12];
    OsRng.fill_bytes(&mut salt);

    // Derive key from password
    let key = derive_key(password, &salt)?;

    // Create ChaCha20Poly1305 instance
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&key));

    // Generate random nonce
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let nonce = GenericArray::from_slice(&nonce);

    // Encrypt data
    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
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
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&key));

    // Create nonce
    let nonce = GenericArray::from_slice(&nonce_bytes);

    // Decrypt data
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| EncryptError::ChaChaError(e.to_string()))?;

    // Convert plaintext to string
    let result = String::from_utf8(plaintext).map_err(|_| EncryptError::InvalidData)?;

    Ok(result)
}

// Add this new async version
pub async fn decrypt_async(encrypted_data: &str, password: &str) -> Result<String, EncryptError> {
    use gloo_timers::future::sleep;
    use std::time::Duration;
    
    // Parse encrypted data
    let parts: Vec<&str> = encrypted_data.split(':').collect();
    if parts.len() != 3 {
        return Err(EncryptError::InvalidData);
    }

    // Parse salt, nonce, and ciphertext
    let salt = hex::decode(parts[0]).map_err(|_| EncryptError::InvalidData)?;
    let nonce_bytes = hex::decode(parts[1]).map_err(|_| EncryptError::InvalidData)?;
    let ciphertext = hex::decode(parts[2]).map_err(|_| EncryptError::InvalidData)?;

    // Give UI a chance to update before CPU-intensive operation
    sleep(Duration::from_millis(10)).await;

    // Derive key from password (this is the CPU-intensive part)
    let key = derive_key(password, &salt)?;

    // Give UI another chance to update
    sleep(Duration::from_millis(10)).await;

    // Create ChaCha20Poly1305 instance
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&key));

    // Create nonce
    let nonce = GenericArray::from_slice(&nonce_bytes);

    // Decrypt data
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| EncryptError::ChaChaError(e.to_string()))?;

    // Convert plaintext to string
    let result = String::from_utf8(plaintext).map_err(|_| EncryptError::InvalidData)?;

    Ok(result)
}

pub fn generate_random_key() -> Secret<String> {
    // create a buffer that can be securely cleared
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).expect("Failed to generate random key");
    
    // convert to hex string and ensure it will be cleared
    let hex_string = Zeroizing::new(hex::encode(key));
    
    // clear original byte array
    key.zeroize();
    
    // convert Zeroizing<String> to Secret<String>
    Secret::new(hex_string.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Existing tests
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

    // 1. Input validation tests
    #[test]
    fn test_input_validation() {
        let password = "password123";

        // Test empty string
        let empty = "";
        let encrypted_empty = encrypt(empty, password).unwrap();
        let decrypted_empty = decrypt(&encrypted_empty, password).unwrap();
        assert_eq!(empty, decrypted_empty);

        // Test long string
        let long_string = "a".repeat(10000);
        let encrypted_long = encrypt(&long_string, password).unwrap();
        let decrypted_long = decrypt(&encrypted_long, password).unwrap();
        assert_eq!(long_string, decrypted_long);

        // Test special characters
        let special_chars = "!@#$%^&*()_+-=[]{}|;:'\",.<>?`~";
        let encrypted_special = encrypt(special_chars, password).unwrap();
        let decrypted_special = decrypt(&encrypted_special, password).unwrap();
        assert_eq!(special_chars, decrypted_special);

        // Test Unicode characters
        let unicode = "Hello, 世界! 🌍";
        let encrypted_unicode = encrypt(unicode, password).unwrap();
        let decrypted_unicode = decrypt(&encrypted_unicode, password).unwrap();
        assert_eq!(unicode, decrypted_unicode);
    }

    // 2. Password validation tests
    #[test]
    fn test_password_validation() {
        let data = "test data";

        // Test empty password
        let empty_pass = "";
        let encrypted_empty = encrypt(data, empty_pass).unwrap();
        let decrypted_empty = decrypt(&encrypted_empty, empty_pass).unwrap();
        assert_eq!(data, decrypted_empty);

        // Test long password
        let long_pass = "a".repeat(1000);
        let encrypted_long = encrypt(data, &long_pass).unwrap();
        let decrypted_long = decrypt(&encrypted_long, &long_pass).unwrap();
        assert_eq!(data, decrypted_long);

        // Test password with special characters
        let special_pass = "!@#$%^&*()_+-=[]{}|;:'\",.<>?`~";
        let encrypted_special = encrypt(data, special_pass).unwrap();
        let decrypted_special = decrypt(&encrypted_special, special_pass).unwrap();
        assert_eq!(data, decrypted_special);
    }

    // 3. Error handling tests
    #[test]
    fn test_error_handling() {
        let password = "password123";

        // Test invalid encrypted data format
        assert!(matches!(
            decrypt("invalid_data", password),
            Err(EncryptError::InvalidData)
        ));

        // Test missing separators
        assert!(matches!(
            decrypt("abc123", password),
            Err(EncryptError::InvalidData)
        ));

        // Test invalid hex encoding
        assert!(matches!(
            decrypt("ZZ:11:22", password),
            Err(EncryptError::InvalidData)
        ));

        // Test wrong number of parts
        assert!(matches!(
            decrypt("11:22", password),
            Err(EncryptError::InvalidData)
        ));
    }

    // 4. Random key generation tests
    #[test]
    fn test_random_key_generation() {
        // Test key length
        let key = generate_random_key();
        assert_eq!(key.expose_secret().len(), 64); // 32 bytes = 64 hex chars

        // Test hex format
        assert!(key.expose_secret().chars().all(|c| c.is_ascii_hexdigit()));

        // Test uniqueness
        let key2 = generate_random_key();
        assert_ne!(key.expose_secret(), key2.expose_secret());
    }

    // 5. Edge cases tests
    #[test]
    fn test_edge_cases() {
        let password = "password123";
        
        // Test various input lengths around block boundaries
        for len in [1, 15, 16, 17, 31, 32, 33, 63, 64, 65] {
            let data = "a".repeat(len);
            let encrypted = encrypt(&data, password).unwrap();
            let decrypted = decrypt(&encrypted, password).unwrap();
            assert_eq!(data, decrypted);
        }
    }
} 