use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::Promise;
use serde::{Serialize, Deserialize};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Debug, Clone)]
pub enum SeedVaultError {
    NotAvailable(String),
    AuthorizationFailed(String),
    SigningFailed(String),
    InvokeError(String),
}

impl std::fmt::Display for SeedVaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedVaultError::NotAvailable(msg) => write!(f, "Seed Vault not available: {}", msg),
            SeedVaultError::AuthorizationFailed(msg) => write!(f, "Authorization failed: {}", msg),
            SeedVaultError::SigningFailed(msg) => write!(f, "Signing failed: {}", msg),
            SeedVaultError::InvokeError(msg) => write!(f, "Invoke error: {}", msg),
        }
    }
}

/// Solana Mobile Seed Vault integration module.
///
/// Communicates with the native Android Seed Vault service
/// through Tauri's plugin invoke mechanism.
pub struct SeedVault;

impl SeedVault {
    /// Check if Seed Vault is available on this device.
    /// Returns true only on Android devices with Seed Vault support.
    pub async fn is_available() -> bool {
        let args = js_sys::Object::new();
        match Self::invoke_plugin("plugin:seed-vault|isAvailable", &args.into()).await {
            Ok(result) => {
                js_sys::Reflect::get(&result, &JsValue::from_str("available"))
                    .ok()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            }
            Err(_) => false,
        }
    }

    /// Request authorization for a seed from the Seed Vault.
    /// This will launch the Seed Vault UI for user approval.
    /// Returns the auth token on success.
    pub async fn authorize_seed() -> Result<i64, SeedVaultError> {
        let args = js_sys::Object::new();
        let result = Self::invoke_plugin("plugin:seed-vault|authorizeSeed", &args.into()).await?;

        js_sys::Reflect::get(&result, &JsValue::from_str("authToken"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as i64)
            .ok_or_else(|| SeedVaultError::AuthorizationFailed("No auth token returned".to_string()))
    }

    /// Get authorized seeds from the Seed Vault.
    /// Returns the first available auth token, or None if no seeds are authorized.
    pub async fn get_authorized_seed_token() -> Result<Option<i64>, SeedVaultError> {
        let args = js_sys::Object::new();
        let result = Self::invoke_plugin("plugin:seed-vault|getAuthorizedSeeds", &args.into()).await?;

        let count = js_sys::Reflect::get(&result, &JsValue::from_str("count"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as i32;

        if count == 0 {
            return Ok(None);
        }

        let auth_token = js_sys::Reflect::get(&result, &JsValue::from_str("authToken"))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|v| v as i64);

        Ok(auth_token)
    }

    /// Get the public key for the default derivation path of an authorized seed.
    /// Returns the base58 encoded public key.
    pub async fn get_public_key(auth_token: i64) -> Result<String, SeedVaultError> {
        let args = js_sys::Object::new();
        js_sys::Reflect::set(&args, &JsValue::from_str("authToken"), &JsValue::from_f64(auth_token as f64))
            .map_err(|e| SeedVaultError::InvokeError(format!("{:?}", e)))?;

        let result = Self::invoke_plugin("plugin:seed-vault|getAccounts", &args.into()).await?;

        js_sys::Reflect::get(&result, &JsValue::from_str("publicKey"))
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| SeedVaultError::AuthorizationFailed("No public key found for this seed".to_string()))
    }

    /// Connect to Seed Vault: authorize a seed and retrieve the public key.
    /// This is the main entry point for login flow.
    /// Returns (auth_token, public_key_base58).
    pub async fn connect() -> Result<(i64, String), SeedVaultError> {
        // First check if we already have an authorized seed
        if let Ok(Some(auth_token)) = Self::get_authorized_seed_token().await {
            // Try to get public key from existing authorization
            match Self::get_public_key(auth_token).await {
                Ok(pubkey) => return Ok((auth_token, pubkey)),
                Err(_) => {
                    // Fall through to request new authorization
                    log::warn!("Existing seed auth token failed, requesting new authorization");
                }
            }
        }

        // Request new seed authorization
        let auth_token = Self::authorize_seed().await?;

        // Get the public key for this seed
        let pubkey = Self::get_public_key(auth_token).await?;

        Ok((auth_token, pubkey))
    }

    /// Sign a transaction using Seed Vault.
    /// Launches the Seed Vault signing UI for user approval.
    ///
    /// # Parameters
    /// * `auth_token` - The auth token from seed authorization
    /// * `transaction_base64` - Base64 encoded unsigned transaction
    ///
    /// # Returns
    /// Base64 encoded signature
    pub async fn sign_transaction(auth_token: i64, transaction_base64: &str) -> Result<String, SeedVaultError> {
        let args = js_sys::Object::new();
        js_sys::Reflect::set(&args, &JsValue::from_str("authToken"), &JsValue::from_f64(auth_token as f64))
            .map_err(|e| SeedVaultError::InvokeError(format!("{:?}", e)))?;
        js_sys::Reflect::set(&args, &JsValue::from_str("transaction"), &JsValue::from_str(transaction_base64))
            .map_err(|e| SeedVaultError::InvokeError(format!("{:?}", e)))?;

        let result = Self::invoke_plugin("plugin:seed-vault|signTransaction", &args.into()).await?;

        js_sys::Reflect::get(&result, &JsValue::from_str("signature"))
            .ok()
            .and_then(|v| v.as_string())
            .ok_or_else(|| SeedVaultError::SigningFailed("No signature returned".to_string()))
    }

    /// Helper to invoke Tauri commands (not plugin commands).
    async fn invoke_plugin(cmd: &str, args: &JsValue) -> Result<JsValue, SeedVaultError> {
        // Extract just the command name from "plugin:seed-vault|commandName" format
        let command_name = if cmd.contains('|') {
            cmd.split('|').nth(1).unwrap_or(cmd)
        } else {
            cmd
        };

        // Call as a regular Tauri command under the plugin namespace
        let full_cmd = format!("plugin:seed-vault|{}", command_name);
        let result = invoke(&full_cmd, args.clone()).await;

        // Check if the result is an error
        if result.is_undefined() || result.is_null() {
            return Err(SeedVaultError::InvokeError("Command returned null/undefined".to_string()));
        }

        // Check for error field in result
        if let Ok(error) = js_sys::Reflect::get(&result, &JsValue::from_str("error")) {
            if let Some(error_msg) = error.as_string() {
                if !error_msg.is_empty() {
                    return Err(SeedVaultError::InvokeError(error_msg));
                }
            }
        }

        // Extract data field from response
        if let Ok(data) = js_sys::Reflect::get(&result, &JsValue::from_str("data")) {
            Ok(data)
        } else {
            Ok(result)
        }
    }
}
