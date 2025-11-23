use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use std::fmt;
use std::str::FromStr;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::{Date, Math};
use solana_sdk::transaction::Transaction;
use solana_sdk::instruction::Instruction;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use base64;
use bincode;
use super::network_config::{try_get_network_config, get_program_ids};
use super::settings::load_current_network_settings;
use super::constants::*;

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
    // Note: Fields are used by serde for deserialization but not directly accessed
    #[allow(dead_code)]
    code: i64,
    #[allow(dead_code)]
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
    // Note: Fields are used by serde for deserialization but not directly accessed
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    #[allow(dead_code)]
    result: T,
    #[allow(dead_code)]
    #[serde(default)]
    error: Option<RpcResponseError>,
}

impl RpcConnection {
    // Fallback RPC endpoint (used before network is initialized during login)
    const FALLBACK_RPC_ENDPOINT: &'static str = "https://rpc.testnet.x1.xyz";
    
    pub fn new() -> Self {
        let selected_endpoint = Self::select_random_endpoint();
        log::debug!("Selected RPC endpoint: {}", selected_endpoint);
        Self::with_endpoint(&selected_endpoint)
    }

    /// select a random endpoint from the RPC endpoint list
    /// Checks user settings first for custom RPC, then falls back to configured endpoints
    fn select_random_endpoint() -> String {
        // Try to get network configuration endpoints
        let endpoints = if let Some(config) = try_get_network_config() {
            // Check if user has configured a custom RPC endpoint
            if let Some(settings) = load_current_network_settings() {
                if let Some(custom_endpoint) = settings.custom_rpc_endpoint() {
                    log::debug!("Using custom RPC endpoint from settings: {}", custom_endpoint);
                    return custom_endpoint;
                }
            }
            config.rpc_endpoints
        } else {
            // Network not initialized yet (before login), use fallback
            log::debug!("Network not initialized, using fallback endpoint");
            return Self::FALLBACK_RPC_ENDPOINT.to_string();
        };
        
        // if there is only one endpoint, return it directly
        if endpoints.len() == 1 {
            return endpoints[0].to_string();
        }
        
        // use high quality random number generator to select endpoint
        if let Some(random_value) = Self::try_crypto_random() {
            let index = (random_value as usize) % endpoints.len();
            endpoints[index].to_string()
        } else {
            // fallback scheme: use Math.random()
            let random_value = Math::random();
            let index = (random_value * endpoints.len() as f64) as usize;
            endpoints[index.min(endpoints.len() - 1)].to_string()
        }
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
        let request_id = Self::generate_request_id();
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: method.to_string(),
            params,
        };

        // Simple request logging for important operations only
        if method == "sendTransaction" {
            log::debug!("Sending transaction request");
        }
        
        let request_body = serde_json::to_string(&request)
            .map_err(|e| {
                log::error!("Failed to serialize request: {}", e);
                RpcError::Other(e.to_string())
            })?;
        
        // Log request details for debugging (but limit size)
        if method == "sendTransaction" || method == "simulateTransaction" {
            log::debug!("RPC request body (first 200 chars): {}", 
                       if request_body.len() > 200 { &request_body[..200] } else { &request_body });
        } else {
            log::debug!("RPC request body: {}", request_body);
        }

        let opts = RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(RequestMode::Cors);
        opts.set_body(&JsValue::from_str(&request_body));

        let request = Request::new_with_str_and_init(&self.endpoint, &opts)
            .map_err(|e| {
                log::error!("Failed to create HTTP request: {:?}", e);
                RpcError::ConnectionFailed(format!("Failed to create request: {:?}", e))
            })?;

        request.headers().set("Content-Type", "application/json")
            .map_err(|e| {
                log::error!("Failed to set HTTP headers: {:?}", e);
                RpcError::ConnectionFailed(format!("Failed to set headers: {:?}", e))
            })?;

        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| {
                log::error!("HTTP request failed: {:?}", e);
                RpcError::ConnectionFailed(format!("Failed to send request: {:?}", e))
            })?;

        let resp: Response = resp_value.dyn_into()
            .map_err(|e| {
                log::error!("Failed to convert response: {:?}", e);
                RpcError::Other(format!("Failed to convert response: {:?}", e))
            })?;

        // Check HTTP status
        if !resp.ok() {
            log::error!("HTTP error: status={}, status_text={}", resp.status(), resp.status_text());
            return Err(RpcError::ConnectionFailed(format!("HTTP {} {}", resp.status(), resp.status_text())));
        }

        let json = JsFuture::from(resp.json().map_err(|e| {
            log::error!("Failed to get JSON from response: {:?}", e);
            RpcError::Other(format!("Failed to get JSON: {:?}", e))
        })?)
            .await
            .map_err(|e| {
                log::error!("Failed to parse JSON: {:?}", e);
                RpcError::Other(format!("Failed to parse JSON: {:?}", e))
            })?;

        // first try to parse as Value, so we can check for errors
        let value: serde_json::Value = json.into_serde()
            .map_err(|e| {
                log::error!("Failed to parse response as JSON Value: {:?}", e);
                RpcError::Other(format!("Failed to parse response as JSON: {:?}", e))
            })?;

        // Simplified response logging - only log errors
        if let Some(error) = value.get("error") {
            log::error!("RPC error for {}: {}", method, error.to_string());
            
            if let Some(error_obj) = error.as_object() {
                let code = error_obj.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                let message = error_obj.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
                
                // Extract specific error details from transaction logs
                let mut specific_error = None;
                if let Some(data) = error_obj.get("data") {
                    // Check for specific Solana contract errors
                    if let Some(err_info) = data.get("err") {
                        if let Some(custom) = err_info.get("InstructionError").and_then(|e| e.as_array()) {
                            if custom.len() >= 2 {
                                if let Some(custom_error) = custom[1].get("Custom") {
                                    let error_code = custom_error.as_i64().unwrap_or(0);
                                    log::error!("Contract error code: {}", error_code);
                                    
                                    // Extract specific error message from logs
                                    if let Some(logs) = data.get("logs").and_then(|l| l.as_array()) {
                                        for log_entry in logs {
                                            if let Some(log_str) = log_entry.as_str() {
                                                if log_str.contains("Error Message:") {
                                                    // Extract the error message after "Error Message:"
                                                    if let Some(msg_start) = log_str.find("Error Message:") {
                                                        let error_msg = &log_str[msg_start + 14..].trim();
                                                        specific_error = Some(error_msg.to_string());
                                                        log::error!("Extracted error message: {}", error_msg);
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Create error message with specific details if available
                let error_message = if let Some(specific_msg) = specific_error {
                    format!("Code {}: {} - {}", code, message, specific_msg)
                } else {
                    format!("Code {}: {}", code, message)
                };
                
                return Err(RpcError::SolanaRpcError(error_message));
            } else {
                return Err(RpcError::Other(error.to_string()));
            }
        }

        // if there is no error, try to get the result
        if let Some(result) = value.get("result") {
            log::debug!("RPC request {} completed successfully", method);
            // convert result to target type
            serde_json::from_value(result.clone())
                .map_err(|e| {
                    log::error!("Failed to deserialize result for method {}: {:?}", method, e);
                    RpcError::Other(format!("Failed to deserialize result: {:?}", e))
                })
        } else {
            log::error!("RPC response missing result field for method {}", method);
            Err(RpcError::Other("Response missing result field".to_string()))
        }
    }

    pub async fn get_balance(&self, pubkey: &str) -> Result<String, RpcError> {
        let result: serde_json::Value = self.send_request("getBalance", vec![pubkey]).await?;
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

    // ============ Common Transaction Utilities ============

    /// Get the latest blockhash from the network
    /// 
    /// This is a common utility method used by all transaction builders.
    /// 
    /// # Returns
    /// The recent blockhash as a Hash
    pub async fn get_latest_blockhash(&self) -> Result<solana_sdk::hash::Hash, RpcError> {
        let blockhash: serde_json::Value = self.send_request(
            "getLatestBlockhash",
            serde_json::json!([{
                "commitment": "confirmed",
                "minContextSlot": 0
            }])
        ).await?;
        
        let recent_blockhash = blockhash["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get blockhash".to_string()))?;
        
        solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))
    }

    /// Send a signed transaction to the network
    /// 
    /// This is a common utility method used by all modules after signing transactions.
    /// 
    /// # Parameters
    /// * `transaction` - The signed transaction to send
    /// 
    /// # Returns
    /// Transaction signature on success
    pub async fn send_signed_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<String, RpcError> {
        // Serialize transaction
        let serialized_tx = base64::encode(bincode::serialize(transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize transaction: {}", e)))?);
        
        let params = serde_json::json!([
            serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "confirmed",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);
        
        log::info!("Sending signed transaction...");
        let result = self.send_request("sendTransaction", params).await?;
        log::info!("Transaction sent successfully: {}", result);
        
        Ok(result)
    }

    // ============ End Transaction Utilities ============

    /// Apply compute budget instructions based on user settings
    /// 
    /// This method adds compute budget instructions to the transaction based on:
    /// 1. Simulated compute units (with optional buffer from settings)
    /// 2. Optional compute unit price for priority fees
    /// 
    /// # Parameters
    /// * `simulated_cu` - The compute units consumed in simulation
    /// * `default_multiplier` - Default multiplier if no settings exist (usually 1.0)
    /// 
    /// # Returns
    /// A vector of compute budget instructions to prepend to the transaction
    pub fn build_compute_budget_instructions(
        simulated_cu: u64,
        default_multiplier: f64,
    ) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        
        // Load user settings
        let user_settings = load_current_network_settings();
        
        // Calculate final compute unit limit
        let cu_multiplier = user_settings
            .as_ref()
            .map(|s| s.get_cu_buffer_multiplier())
            .unwrap_or(default_multiplier);
        
        let final_cu = ((simulated_cu as f64) * cu_multiplier).ceil() as u64;
        let final_cu_u32 = final_cu.min(u32::MAX as u64) as u32;
        
        log::info!(
            "Compute budget: simulated={} CU, multiplier={:.2}, final={} CU",
            simulated_cu,
            cu_multiplier,
            final_cu_u32
        );
        
        // Add compute unit limit instruction
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(final_cu_u32));
        
        // Add compute unit price instruction if user has set a priority fee
        if let Some(settings) = user_settings {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                log::info!("Setting compute unit price: {} micro-lamports", price);
                instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
            }
        }
        
        instructions
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

    /// Helper function to read a String from account data
    pub fn read_string_from_data(&self, data: &[u8], offset: usize) -> Result<(String, usize), RpcError> {
        if data.len() < offset + 4 {
            return Err(RpcError::Other("Data too short for string length".to_string()));
        }
        
        let len = u32::from_le_bytes(
            data[offset..offset + 4].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse string length: {:?}", e)))?
        ) as usize;
        let new_offset = offset + 4;
        
        if data.len() < new_offset + len {
            return Err(RpcError::Other("Data too short for string content".to_string()));
        }
        
        let string_data = &data[new_offset..new_offset + len];
        let string = String::from_utf8(string_data.to_vec())
            .map_err(|e| RpcError::Other(format!("Failed to parse string as UTF-8: {}", e)))?;
        
        Ok((string, new_offset + len))
    }
    
    /// Helper function to read a Vec<String> from account data
    pub fn read_string_vec_from_data(&self, data: &[u8], offset: usize) -> Result<(Vec<String>, usize), RpcError> {
        if data.len() < offset + 4 {
            return Err(RpcError::Other("Data too short for vec length".to_string()));
        }
        
        let vec_len = u32::from_le_bytes(
            data[offset..offset + 4].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse vec length: {:?}", e)))?
        ) as usize;
        let mut new_offset = offset + 4;
        let mut strings = Vec::new();
        
        for _ in 0..vec_len {
            let (string, next_offset) = self.read_string_from_data(data, new_offset)?;
            strings.push(string);
            new_offset = next_offset;
        }
        
        Ok((strings, new_offset))
    }
}

// implement the default trait
impl Default for RpcConnection {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Shared Helper Functions
// ============================================================================
// These functions are commonly used across multiple RPC modules

/// Get the token mint address from network configuration
pub fn get_token_mint() -> Result<Pubkey, RpcError> {
    let program_ids = get_program_ids();
    Pubkey::from_str(program_ids.token_mint)
        .map_err(|e| RpcError::InvalidAddress(format!("Invalid token mint: {}", e)))
}

/// Get the Token 2022 program ID from network configuration
pub fn get_token_2022_program_id() -> Result<Pubkey, RpcError> {
    let program_ids = get_program_ids();
    Pubkey::from_str(program_ids.token_2022_program_id)
        .map_err(|e| RpcError::InvalidAddress(format!("Invalid Token 2022 program ID: {}", e)))
}

/// Validate memo string length (for &str input)
pub fn validate_memo_length_str(memo: &str) -> Result<(), RpcError> {
    let len = memo.len();
    if len < MIN_MEMO_LENGTH {
        return Err(RpcError::Other(format!("Memo too short: {} bytes (min: {})", len, MIN_MEMO_LENGTH)));
    }
    if len > MAX_MEMO_LENGTH {
        return Err(RpcError::Other(format!("Memo too long: {} bytes (max: {})", len, MAX_MEMO_LENGTH)));
    }
    Ok(())
}

/// Validate memo data length (for &[u8] input)
pub fn validate_memo_length_bytes(memo_data: &[u8]) -> Result<(), RpcError> {
    let len = memo_data.len();
    if len < MIN_MEMO_LENGTH {
        return Err(RpcError::Other(format!("Memo data too short: {} bytes (min: {})", len, MIN_MEMO_LENGTH)));
    }
    if len > MAX_MEMO_LENGTH {
        return Err(RpcError::Other(format!("Memo data too long: {} bytes (max: {})", len, MAX_MEMO_LENGTH)));
    }
    Ok(())
}
 