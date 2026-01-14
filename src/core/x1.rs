use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use js_sys::Promise;
use web_sys::window;

#[derive(Debug, Clone)]
pub enum X1Error {
    ConnectionFailed(String),
    SigningFailed(String),
    JavaScriptError(String),
}

impl std::fmt::Display for X1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X1Error::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            X1Error::SigningFailed(msg) => write!(f, "Signing failed: {}", msg),
            X1Error::JavaScriptError(msg) => write!(f, "JavaScript error: {}", msg),
        }
    }
}

/// X1 wallet integration module
/// 
/// X1 wallet is injected via `window.x1` (similar to Backpack's `window.backpack`)
/// The provider follows the Solana wallet standard interface.
pub struct X1Wallet;

impl X1Wallet {
    /// Check if X1 wallet is installed in the browser
    /// 
    /// X1 wallet is detected via `window.x1` object existence
    /// The provider is injected by the X1 wallet extension
    pub fn is_installed() -> bool {
        window()
            .and_then(|win| {
                js_sys::Reflect::get(&win, &JsValue::from_str("x1"))
                    .ok()
                    .map(|wallet_obj| {
                        // Check if x1 object exists and is not null/undefined
                        !wallet_obj.is_null() && !wallet_obj.is_undefined()
                    })
            })
            .unwrap_or(false)
    }

    /// Connect to X1 wallet and return the public key
    /// 
    /// This calls `window.x1.connect()` which returns a Promise
    /// that resolves to the connected wallet info.
    pub async fn connect() -> Result<String, X1Error> {
        let window = window().ok_or(X1Error::JavaScriptError("No window object".to_string()))?;
        
        let wallet_obj = js_sys::Reflect::get(&window, &JsValue::from_str("x1"))
            .map_err(|e| X1Error::JavaScriptError(format!("Failed to get x1: {:?}", e)))?;
        
        if wallet_obj.is_null() || wallet_obj.is_undefined() {
            return Err(X1Error::JavaScriptError("X1 wallet not found".to_string()));
        }
        
        let connect_func = js_sys::Reflect::get(&wallet_obj, &JsValue::from_str("connect"))
            .map_err(|e| X1Error::JavaScriptError(format!("Failed to get connect function: {:?}", e)))?;
        
        if !connect_func.is_function() {
            return Err(X1Error::JavaScriptError("connect is not a function".to_string()));
        }
        
        let func = js_sys::Function::from(connect_func);
        let promise = func.call0(&wallet_obj)
            .map_err(|e| X1Error::ConnectionFailed(format!("{:?}", e)))?;
        
        let promise = Promise::from(promise);
        let result = JsFuture::from(promise).await
            .map_err(|e| X1Error::ConnectionFailed(format!("{:?}", e)))?;
        
        // The result might be an object with publicKey property or a string directly
        // Try to get publicKey from result object first
        if let Ok(pubkey_val) = js_sys::Reflect::get(&result, &JsValue::from_str("publicKey")) {
            // publicKey might be an object with toString() method
            if let Ok(to_string_func) = js_sys::Reflect::get(&pubkey_val, &JsValue::from_str("toString")) {
                if to_string_func.is_function() {
                    let func = js_sys::Function::from(to_string_func);
                    if let Ok(str_result) = func.call0(&pubkey_val) {
                        if let Some(pubkey_str) = str_result.as_string() {
                            return Ok(pubkey_str);
                        }
                    }
                }
            }
            // Try as string directly
            if let Some(pubkey_str) = pubkey_val.as_string() {
                return Ok(pubkey_str);
            }
        }
        
        // Try result as string directly
        if let Some(pubkey_str) = result.as_string() {
            return Ok(pubkey_str);
        }
        
        // Try result.toString()
        if let Ok(to_string_func) = js_sys::Reflect::get(&result, &JsValue::from_str("toString")) {
            if to_string_func.is_function() {
                let func = js_sys::Function::from(to_string_func);
                if let Ok(str_result) = func.call0(&result) {
                    if let Some(pubkey_str) = str_result.as_string() {
                        return Ok(pubkey_str);
                    }
                }
            }
        }
        
        Err(X1Error::ConnectionFailed("Failed to extract public key from connect result".to_string()))
    }

    /// Disconnect from X1 wallet
    pub async fn disconnect() -> Result<(), X1Error> {
        let window = window().ok_or(X1Error::JavaScriptError("No window object".to_string()))?;
        
        let wallet_obj = js_sys::Reflect::get(&window, &JsValue::from_str("x1"))
            .map_err(|e| X1Error::JavaScriptError(format!("Failed to get x1: {:?}", e)))?;
        
        if wallet_obj.is_null() || wallet_obj.is_undefined() {
            return Ok(()); // Already disconnected if wallet not found
        }
        
        let disconnect_func = js_sys::Reflect::get(&wallet_obj, &JsValue::from_str("disconnect"))
            .map_err(|e| X1Error::JavaScriptError(format!("Failed to get disconnect function: {:?}", e)))?;
        
        if !disconnect_func.is_function() {
            return Ok(()); // If not a function, consider it already disconnected
        }
        
        let func = js_sys::Function::from(disconnect_func);
        let promise = func.call0(&wallet_obj)
            .map_err(|e| X1Error::JavaScriptError(format!("{:?}", e)))?;
        
        if promise.is_object() && !promise.is_null() && !promise.is_undefined() {
            // Check if it's a Promise (has then method)
            if let Ok(then_func) = js_sys::Reflect::get(&promise, &JsValue::from_str("then")) {
                if then_func.is_function() {
                    let promise = Promise::from(promise);
                    JsFuture::from(promise).await
                        .map_err(|e| X1Error::JavaScriptError(format!("{:?}", e)))?;
                }
            }
        }
        
        Ok(())
    }

    /// Sign a transaction with X1 wallet
    /// 
    /// # Parameters
    /// * `transaction_base64` - Base64 encoded unsigned transaction
    /// 
    /// # Returns
    /// Base64 encoded signed transaction
    pub async fn sign_transaction(transaction_base64: &str) -> Result<String, X1Error> {
        let window = window().ok_or(X1Error::JavaScriptError("No window object".to_string()))?;
        
        let wallet_obj = js_sys::Reflect::get(&window, &JsValue::from_str("x1"))
            .map_err(|e| X1Error::JavaScriptError(format!("Failed to get x1: {:?}", e)))?;
        
        if wallet_obj.is_null() || wallet_obj.is_undefined() {
            return Err(X1Error::JavaScriptError("X1 wallet not found".to_string()));
        }
        
        let sign_func = js_sys::Reflect::get(&wallet_obj, &JsValue::from_str("signTransaction"))
            .map_err(|e| X1Error::JavaScriptError(format!("Failed to get signTransaction function: {:?}", e)))?;
        
        if !sign_func.is_function() {
            return Err(X1Error::JavaScriptError("signTransaction is not a function".to_string()));
        }
        
        let func = js_sys::Function::from(sign_func);
        let promise = func.call1(&wallet_obj, &JsValue::from_str(transaction_base64))
            .map_err(|e| X1Error::SigningFailed(format!("{:?}", e)))?;
        
        let promise = Promise::from(promise);
        let result = JsFuture::from(promise).await
            .map_err(|e| X1Error::SigningFailed(format!("{:?}", e)))?;
        
        // Try result as string directly (base64 encoded)
        if let Some(signed_tx_str) = result.as_string() {
            return Ok(signed_tx_str);
        }
        
        // Try to handle _signedBytes property (X1 wallet specific format)
        if let Ok(signed_bytes_val) = js_sys::Reflect::get(&result, &JsValue::from_str("_signedBytes")) {
            // Get the length by iterating until we can't find a key
            let mut bytes = Vec::new();
            let mut index = 0;
            
            loop {
                let key = JsValue::from_str(&index.to_string());
                match js_sys::Reflect::get(&signed_bytes_val, &key) {
                    Ok(val) if !val.is_undefined() => {
                        if let Some(byte_num) = val.as_f64() {
                            bytes.push(byte_num as u8);
                            index += 1;
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            
            if !bytes.is_empty() {
                let base64_str = base64::encode(&bytes);
                return Ok(base64_str);
            }
        }
        
        // Try to handle Uint8Array or byte array
        if let Ok(array) = js_sys::Uint8Array::new(&result).dyn_into::<js_sys::Uint8Array>() {
            let bytes = array.to_vec();
            let base64_str = base64::encode(&bytes);
            return Ok(base64_str);
        }
        
        // Try to get transaction property which might be Uint8Array
        if let Ok(tx_val) = js_sys::Reflect::get(&result, &JsValue::from_str("transaction")) {
            // Check if it's a string
            if let Some(tx_str) = tx_val.as_string() {
                return Ok(tx_str);
            }
            // Check if it's a Uint8Array
            if let Ok(array) = js_sys::Uint8Array::new(&tx_val).dyn_into::<js_sys::Uint8Array>() {
                let bytes = array.to_vec();
                let base64_str = base64::encode(&bytes);
                return Ok(base64_str);
            }
        }
        
        // Try to get signedTransaction property
        if let Ok(signed_tx_val) = js_sys::Reflect::get(&result, &JsValue::from_str("signedTransaction")) {
            // Check if it's a string
            if let Some(tx_str) = signed_tx_val.as_string() {
                return Ok(tx_str);
            }
            // Check if it's a Uint8Array
            if let Ok(array) = js_sys::Uint8Array::new(&signed_tx_val).dyn_into::<js_sys::Uint8Array>() {
                let bytes = array.to_vec();
                let base64_str = base64::encode(&bytes);
                return Ok(base64_str);
            }
        }
        
        // Try result.toString()
        if let Ok(to_string_func) = js_sys::Reflect::get(&result, &JsValue::from_str("toString")) {
            if to_string_func.is_function() {
                let func = js_sys::Function::from(to_string_func);
                if let Ok(str_result) = func.call0(&result) {
                    if let Some(tx_str) = str_result.as_string() {
                        return Ok(tx_str);
                    }
                }
            }
        }
        
        Err(X1Error::SigningFailed("Signed transaction is not a string or Uint8Array".to_string()))
    }
}
