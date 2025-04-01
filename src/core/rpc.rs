use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use js_sys::Promise;
use std::fmt;
use serde_wasm_bindgen::from_value;
use gloo_utils::format::JsValueSerdeExt;
use base64;

// error type
#[derive(Debug, Deserialize)]
pub enum RpcError {
    ConnectionFailed(String),
    InvalidAddress(String),
    TransactionFailed(String),
    Other(String),
}

// implement the display for the rpc error
impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            RpcError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            RpcError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            RpcError::Other(msg) => write!(f, "Error: {}", msg),
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
    const DEFAULT_RPC_ENDPOINT: &'static str = "https://rpc.testnet.x1.xyz";
    
    pub fn new() -> Self {
        Self::with_endpoint(Self::DEFAULT_RPC_ENDPOINT)
    }

    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }

    async fn send_request<T, R>(&self, method: &str, params: T) -> Result<R, RpcError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };

        let mut opts = RequestInit::new();
        opts.method("POST");
        opts.mode(RequestMode::Cors);
        opts.body(Some(&JsValue::from_str(&serde_json::to_string(&request)
            .map_err(|e| RpcError::Other(e.to_string()))?)));

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

        let response: RpcResponse<R> = json.into_serde()
            .map_err(|e| RpcError::Other(format!("Failed to deserialize response: {:?}", e)))?;

        if let Some(error) = response.error {
            return Err(RpcError::Other(error.message));
        }

        Ok(response.result)
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

    pub async fn get_user_profile(&self, pubkey: &str) -> Result<String, RpcError> {
        // Program ID
        const PROGRAM_ID: &str = "TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw";
        
        // calculate PDA
        let seeds = format!("[{{\"pubkey\":\"{}\",\"seeds\":[\"user_profile\",\"{}\"]}}", PROGRAM_ID, pubkey);
        let pda: serde_json::Value = self.send_request("getProgramDerivedAddress", seeds).await?;
        
        // get account info
        let account_info: serde_json::Value = self.send_request("getAccountInfo", vec![pda.to_string()]).await?;
        
        Ok(account_info.to_string())
    }

    pub async fn initialize_user_profile(
        &self, 
        pubkey: &str,
        username: &str, 
        profile_image: &str
    ) -> Result<String, RpcError> {
        // Program ID
        const PROGRAM_ID: &str = "TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw";
        
        // Validate inputs
        if username.len() > 32 {
            return Err(RpcError::Other("Username too long. Maximum length is 32 characters.".to_string()));
        }
        if profile_image.len() > 256 {
            return Err(RpcError::Other("Profile image too long. Maximum length is 256 characters.".to_string()));
        }
        if !profile_image.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(RpcError::Other("Profile image must be a valid hexadecimal string.".to_string()));
        }

        // Get latest blockhash
        let blockhash: serde_json::Value = self.send_request(
            "getLatestBlockhash",
            serde_json::json!([{"commitment": "processed"}])
        ).await?;

        // Construct the instruction data
        let mut instruction_data = Vec::new();
        
        // Add discriminator [192, 144, 204, 140, 113, 25, 59, 102]
        instruction_data.extend_from_slice(&[192, 144, 204, 140, 113, 25, 59, 102]);
        
        // Add username length and bytes
        instruction_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
        instruction_data.extend_from_slice(username.as_bytes());
        
        // Add profile_image length and bytes
        instruction_data.extend_from_slice(&(profile_image.len() as u32).to_le_bytes());
        instruction_data.extend_from_slice(profile_image.as_bytes());

        // Construct the transaction
        let params = serde_json::json!({
            "programId": PROGRAM_ID,
            "data": base64::encode(&instruction_data),
            "recentBlockhash": blockhash["value"]["blockhash"],
            "instructions": [{
                "programId": PROGRAM_ID,
                "accounts": [
                    {
                        "pubkey": pubkey,  // user (signer)
                        "isSigner": true,
                        "isWritable": true
                    },
                    {
                        "pubkey": format!("user_profile_{}", pubkey),  // PDA
                        "isSigner": false,
                        "isWritable": true
                    },
                    {
                        "pubkey": "11111111111111111111111111111111",  // System Program
                        "isSigner": false,
                        "isWritable": false
                    }
                ],
                "data": base64::encode(&instruction_data)
            }]
        });

        // Send transaction
        let result: serde_json::Value = self.send_request("sendTransaction", params).await?;
        
        Ok(result["result"].to_string())
    }
}

// implement the default trait
impl Default for RpcConnection {
    fn default() -> Self {
        Self::new()
    }
}