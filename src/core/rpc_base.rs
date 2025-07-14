use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use std::fmt;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::{Date, Math};

// error type
#[derive(Debug, Deserialize)]
pub enum RpcError {
    ConnectionFailed(String),
    InvalidAddress(String),
    TransactionFailed(String),
    Other(String),
    InvalidParameter(String),
    SolanaRpcError(String),
}

// implement the display for the rpc error
impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            RpcError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            RpcError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            RpcError::Other(msg) => write!(f, "Error: {}", msg),
            RpcError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            RpcError::SolanaRpcError(msg) => write!(f, "Solana RPC error: {}", msg),
        }
    }
}

// define the rpc response error structure
#[derive(Deserialize, Debug)]
struct RpcResponseError {
    code: i64,
    message: String,
}

pub struct RpcConnection {
    endpoint: String,
}

#[derive(Serialize)]
struct RpcRequest<T> {
    jsonrpc: String,
    id: u64,
    method: String,
    params: T,
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    jsonrpc: String,
    id: u64,
    result: T,
    #[serde(default)]
    error: Option<RpcResponseError>,
}

impl RpcConnection {
    // X1 testnet RPC endpoint
    const DEFAULT_RPC_ENDPOINT: &'static str = "https://rpc-testnet.x1.wiki";
    
    pub fn new() -> Self {
        Self::with_endpoint(Self::DEFAULT_RPC_ENDPOINT)
    }

    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }

    /// generate unique request id, use crypto random number first, time stamp as fallback
    fn generate_request_id() -> u64 {
        // try to use crypto API
        if let Some(crypto_id) = Self::try_crypto_random() {
            crypto_id
        } else {
            // fallback to time stamp scheme
            Self::fallback_timestamp_random()
        }
    }
    
    /// use crypto.getRandomValues to generate high quality random number
    fn try_crypto_random() -> Option<u64> {
        let window = web_sys::window()?;
        let crypto = window.crypto().ok()?;
        
        // create 8 byte array to store random number
        let mut buffer = [0u8; 8];
        
        // use get_random_values_with_u8_array, pass mutable reference
        if crypto.get_random_values_with_u8_array(&mut buffer).is_ok() {
            // convert 8 bytes to u64
            let mut result = 0u64;
            for &byte in buffer.iter() {
                result = (result << 8) | (byte as u64);
            }
            
            // ensure it is a positive number (remove the highest bit sign)
            Some(result & 0x7FFFFFFFFFFFFFFF)
        } else {
            None
        }
    }
    
    /// fallback scheme: time stamp + Math.random()
    fn fallback_timestamp_random() -> u64 {
        let timestamp = Date::now() as u64;
        let random_part = (Math::random() * 10000.0) as u64;
        let timestamp_part = timestamp % 10_000_000_000;
        timestamp_part * 10000 + random_part
    }

    pub async fn send_request<T, R>(&self, method: &str, params: T) -> Result<R, RpcError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Self::generate_request_id(),
            method: method.to_string(),
            params,
        };

        let mut opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        opts.set_body(&JsValue::from_str(&serde_json::to_string(&request)
            .map_err(|e| RpcError::Other(e.to_string()))?));

        let request = Request::new_with_str_and_init(&self.endpoint, &opts)
            .map_err(|e| RpcError::ConnectionFailed(format!("Failed to create request: {:?}", e)))?;

        request.headers().set("Content-Type", "application/json")
            .map_err(|e| RpcError::ConnectionFailed(format!("Failed to set headers: {:?}", e)))?;

        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| RpcError::ConnectionFailed(format!("Failed to send request: {:?}", e)))?;

        let resp: Response = resp_value.dyn_into()
            .map_err(|e| RpcError::Other(format!("Failed to convert response: {:?}", e)))?;

        let json = JsFuture::from(resp.json().map_err(|e| RpcError::Other(format!("Failed to get JSON: {:?}", e)))?)
            .await
            .map_err(|e| RpcError::Other(format!("Failed to parse JSON: {:?}", e)))?;

        // first try to parse as Value, so we can check for errors
        let value: serde_json::Value = json.into_serde()
            .map_err(|e| RpcError::Other(format!("Failed to parse response as JSON: {:?}", e)))?;

        // check if there is an error
        if let Some(error) = value.get("error") {
            return Err(RpcError::Other(error.to_string()));
        }

        // if there is no error, try to get the result
        if let Some(result) = value.get("result") {
            // convert result to target type
            serde_json::from_value(result.clone())
                .map_err(|e| RpcError::Other(format!("Failed to deserialize result: {:?}", e)))
        } else {
            Err(RpcError::Other("Response missing result field".to_string()))
        }
    }

    pub async fn get_balance(&self, pubkey: &str) -> Result<String, RpcError> {
        let result: serde_json::Value = self.send_request("getBalance", vec![pubkey]).await?;
        Ok(result.to_string())
    }

    pub async fn get_latest_blockhash(&self) -> Result<String, RpcError> {
        let result: serde_json::Value = self.send_request("getLatestBlockhash", Vec::<String>::new()).await?;
        Ok(result.to_string())
    }

    pub async fn send_transaction(&self, serialized_tx: &str) -> Result<String, RpcError> {
        self.send_request("sendTransaction", vec![serialized_tx]).await
    }

    pub async fn get_transaction_status(&self, signature: &str) -> Result<String, RpcError> {
        let result: serde_json::Value = self.send_request("getSignatureStatuses", vec![vec![signature]]).await?;
        Ok(result.to_string())
    }

    pub async fn get_version(&self) -> Result<String, RpcError> {
        let result: serde_json::Value = self.send_request("getVersion", Vec::<String>::new()).await?;
        Ok(result.to_string())
    }

    pub async fn get_token_balance(&self, owner: &str, token_mint: &str) -> Result<String, RpcError> {
        let params = serde_json::json!([
            owner,
            {
                "mint": token_mint
            },
            {
                "encoding": "jsonParsed"
            }
        ]);
        
        let result: serde_json::Value = self.send_request("getTokenAccountsByOwner", params).await?;
        Ok(result.to_string())
    }

    pub async fn get_account_info(&self, pubkey: &str, encoding: Option<&str>) -> Result<String, RpcError> {
        let params = if let Some(enc) = encoding {
            serde_json::json!([pubkey, {"encoding": enc}])
        } else {
            serde_json::json!([pubkey])
        };
        
        let result: serde_json::Value = self.send_request("getAccountInfo", params).await?;
        Ok(result.to_string())
    }

    pub async fn simulate_transaction(&self, serialized_tx: &str, options: Option<serde_json::Value>) -> Result<String, RpcError> {
        let params = if let Some(opts) = options {
            serde_json::json!([serialized_tx, opts])
        } else {
            serde_json::json!([serialized_tx])
        };
        
        let result: serde_json::Value = self.send_request("simulateTransaction", params).await?;
        Ok(result.to_string())
    }

    /// interface: get transaction details by signature
    /// return full transaction information, including meta, transaction, etc.
    pub async fn get_transaction_details(&self, signature: &str) -> Result<String, RpcError> {
        let params = serde_json::json!([
            signature,
            {
                "encoding": "json",
                "maxSupportedTransactionVersion": 0
            }
        ]);
        
        let result: serde_json::Value = self.send_request("getTransaction", params).await?;
        Ok(result.to_string())
    }

    /// interface: get signatures for address
    /// returns confirmed transaction signatures that include the given address
    pub async fn get_signatures_for_address(&self, address: &str, options: Option<serde_json::Value>) -> Result<String, RpcError> {
        let params = if let Some(opts) = options {
            serde_json::json!([address, opts])
        } else {
            serde_json::json!([address])
        };
        
        let result: serde_json::Value = self.send_request("getSignaturesForAddress", params).await?;
        Ok(result.to_string())
    }
}

// implement the default trait
impl Default for RpcConnection {
    fn default() -> Self {
        Self::new()
    }
} 