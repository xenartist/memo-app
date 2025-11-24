use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::Promise;
use web_sys::window;

#[derive(Debug, Clone)]
pub enum BackpackError {
    ConnectionFailed(String),
    SigningFailed(String),
    JavaScriptError(String),
}

impl std::fmt::Display for BackpackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackpackError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            BackpackError::SigningFailed(msg) => write!(f, "Signing failed: {}", msg),
            BackpackError::JavaScriptError(msg) => write!(f, "JavaScript error: {}", msg),
        }
    }
}

/// Backpack wallet integration module
pub struct BackpackWallet;

impl BackpackWallet {
    /// Check if Backpack wallet is installed in the browser
    pub fn is_installed() -> bool {
        window()
            .and_then(|win| {
                js_sys::Reflect::get(&win, &JsValue::from_str("BackpackWallet"))
                    .ok()
                    .and_then(|wallet_obj| {
                        js_sys::Reflect::get(&wallet_obj, &JsValue::from_str("isInstalled"))
                            .ok()
                            .and_then(|func| {
                                if func.is_function() {
                                    let func = js_sys::Function::from(func);
                                    func.call0(&JsValue::NULL).ok()
                                        .and_then(|result| result.as_bool())
                                } else {
                                    None
                                }
                            })
                    })
            })
            .unwrap_or(false)
    }

    /// Connect to Backpack wallet and return the public key
    pub async fn connect() -> Result<String, BackpackError> {
        let window = window().ok_or(BackpackError::JavaScriptError("No window object".to_string()))?;
        
        let wallet_obj = js_sys::Reflect::get(&window, &JsValue::from_str("BackpackWallet"))
            .map_err(|e| BackpackError::JavaScriptError(format!("Failed to get BackpackWallet: {:?}", e)))?;
        
        let connect_func = js_sys::Reflect::get(&wallet_obj, &JsValue::from_str("connect"))
            .map_err(|e| BackpackError::JavaScriptError(format!("Failed to get connect function: {:?}", e)))?;
        
        if !connect_func.is_function() {
            return Err(BackpackError::JavaScriptError("connect is not a function".to_string()));
        }
        
        let func = js_sys::Function::from(connect_func);
        let promise = func.call0(&JsValue::NULL)
            .map_err(|e| BackpackError::ConnectionFailed(format!("{:?}", e)))?;
        
        let promise = Promise::from(promise);
        let result = JsFuture::from(promise).await
            .map_err(|e| BackpackError::ConnectionFailed(format!("{:?}", e)))?;
        
        result.as_string()
            .ok_or(BackpackError::ConnectionFailed("Public key is not a string".to_string()))
    }

    /// Disconnect from Backpack wallet
    pub async fn disconnect() -> Result<(), BackpackError> {
        let window = window().ok_or(BackpackError::JavaScriptError("No window object".to_string()))?;
        
        let wallet_obj = js_sys::Reflect::get(&window, &JsValue::from_str("BackpackWallet"))
            .map_err(|e| BackpackError::JavaScriptError(format!("Failed to get BackpackWallet: {:?}", e)))?;
        
        let disconnect_func = js_sys::Reflect::get(&wallet_obj, &JsValue::from_str("disconnect"))
            .map_err(|e| BackpackError::JavaScriptError(format!("Failed to get disconnect function: {:?}", e)))?;
        
        if !disconnect_func.is_function() {
            return Ok(()); // If not a function, consider it already disconnected
        }
        
        let func = js_sys::Function::from(disconnect_func);
        let promise = func.call0(&JsValue::NULL)
            .map_err(|e| BackpackError::JavaScriptError(format!("{:?}", e)))?;
        
        if promise.is_object() {
            let promise = Promise::from(promise);
            JsFuture::from(promise).await
                .map_err(|e| BackpackError::JavaScriptError(format!("{:?}", e)))?;
        }
        
        Ok(())
    }

    /// Sign a transaction with Backpack wallet
    /// 
    /// # Parameters
    /// * `transaction_base64` - Base64 encoded unsigned transaction
    /// 
    /// # Returns
    /// Base64 encoded signed transaction
    pub async fn sign_transaction(transaction_base64: &str) -> Result<String, BackpackError> {
        let window = window().ok_or(BackpackError::JavaScriptError("No window object".to_string()))?;
        
        let wallet_obj = js_sys::Reflect::get(&window, &JsValue::from_str("BackpackWallet"))
            .map_err(|e| BackpackError::JavaScriptError(format!("Failed to get BackpackWallet: {:?}", e)))?;
        
        let sign_func = js_sys::Reflect::get(&wallet_obj, &JsValue::from_str("signTransaction"))
            .map_err(|e| BackpackError::JavaScriptError(format!("Failed to get signTransaction function: {:?}", e)))?;
        
        if !sign_func.is_function() {
            return Err(BackpackError::JavaScriptError("signTransaction is not a function".to_string()));
        }
        
        let func = js_sys::Function::from(sign_func);
        let promise = func.call1(&JsValue::NULL, &JsValue::from_str(transaction_base64))
            .map_err(|e| BackpackError::SigningFailed(format!("{:?}", e)))?;
        
        let promise = Promise::from(promise);
        let result = JsFuture::from(promise).await
            .map_err(|e| BackpackError::SigningFailed(format!("{:?}", e)))?;
        
        result.as_string()
            .ok_or(BackpackError::SigningFailed("Signed transaction is not a string".to_string()))
    }
}

