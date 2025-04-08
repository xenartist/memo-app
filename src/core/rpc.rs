use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use js_sys::Promise;
use std::fmt;
use serde_wasm_bindgen::from_value;
use gloo_utils::format::JsValueSerdeExt;
use base64;
use bs58;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use flate2::read::DeflateDecoder;
use std::io::Read;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
    message::Message,
};
use bincode;
use gloo_timers;

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

    pub async fn get_user_profile(&self, pubkey: &str) -> Result<String, RpcError> {
        // Program ID
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))?;
        
        let target_pubkey = Pubkey::from_str(pubkey)
            .map_err(|e| RpcError::Other(format!("Invalid public key: {}", e)))?;

        // Calculate user profile PDA
        let (user_profile_pda, _) = Pubkey::find_program_address(
            &[b"user_profile", target_pubkey.as_ref()],
            &program_id
        );

        // get account info, using base64 encoding
        let params = serde_json::json!([
            user_profile_pda.to_string(),
            {"encoding": "base64"}
        ]);

        // get raw account data and return directly
        let result: serde_json::Value = self.send_request("getAccountInfo", params).await?;
        Ok(result.to_string())
    }

    pub async fn initialize_user_profile(
        &self, 
        pubkey: &str,
        username: &str, 
        profile_image: &str,
        keypair_bytes: &[u8]
    ) -> Result<String, RpcError> {
        use solana_sdk::{
            signature::{Keypair, Signer},
            instruction::{AccountMeta, Instruction},
            transaction::Transaction,
            message::Message,
        };

        // Program ID
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))?;
        
        let target_pubkey = Pubkey::from_str(pubkey)
            .map_err(|e| RpcError::Other(format!("Invalid public key: {}", e)))?;

        // Validate inputs
        if username.len() > 32 {
            return Err(RpcError::Other("Username too long. Maximum length is 32 characters.".to_string()));
        }
        if profile_image.len() > 256 {
            return Err(RpcError::Other("Profile image too long. Maximum length is 256 characters.".to_string()));
        }

        // Create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;

        // Calculate user profile PDA
        let (user_profile_pda, _) = Pubkey::find_program_address(
            &[b"user_profile", target_pubkey.as_ref()],
            &program_id
        );

        // Get latest blockhash with specific commitment
        let blockhash: serde_json::Value = self.send_request(
            "getLatestBlockhash",
            serde_json::json!([{
                "commitment": "finalized",
                "minContextSlot": 0
            }])
        ).await?;

        let recent_blockhash = blockhash["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get blockhash".to_string()))?;

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

        // Create the instruction
        let instruction = Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(target_pubkey, true),     // user (signer)
                AccountMeta::new(user_profile_pda, false), // user_profile PDA
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // System Program
            ],
        );

        // Create the message
        let message = Message::new(
            &[instruction],
            Some(&target_pubkey), // fee payer
        );

        // Create and sign transaction
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        transaction.sign(&[&keypair], transaction.message.recent_blockhash);

        // Serialize the transaction to base64
        let serialized_tx = base64::encode(bincode::serialize(&transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize transaction: {}", e)))?);

        // Send transaction with preflight checks and specific commitment
        let params = serde_json::json!([
            serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "finalized",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        let result: serde_json::Value = self.send_request("sendTransaction", params).await?;
        Ok(result.to_string())
    }

    pub async fn close_user_profile(&self, pubkey: &str, keypair_bytes: &[u8]) -> Result<String, RpcError> {
        use solana_sdk::{
            signature::{Keypair, Signer},
            instruction::{AccountMeta, Instruction},
            transaction::Transaction,
            message::Message,
        };

        // Program ID
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))?;
        
        let target_pubkey = Pubkey::from_str(pubkey)
            .map_err(|e| RpcError::Other(format!("Invalid public key: {}", e)))?;

        // Create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;

        // Calculate user profile PDA
        let (user_profile_pda, _) = Pubkey::find_program_address(
            &[b"user_profile", target_pubkey.as_ref()],
            &program_id
        );

        // Get latest blockhash with specific commitment
        let blockhash: serde_json::Value = self.send_request(
            "getLatestBlockhash",
            serde_json::json!([{
                "commitment": "finalized",
                "minContextSlot": 0
            }])
        ).await?;

        let recent_blockhash = blockhash["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get blockhash".to_string()))?;

        // Create instruction data with discriminator
        let instruction_data = vec![242, 80, 248, 79, 81, 251, 65, 113]; // close_user_profile discriminator

        // Create the instruction
        let instruction = Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(target_pubkey, true),     // user (signer)
                AccountMeta::new(user_profile_pda, false), // user_profile PDA
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // System Program
            ],
        );

        // Create the message
        let message = Message::new(
            &[instruction],
            Some(&target_pubkey), // fee payer
        );

        // Create and sign transaction
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        transaction.sign(&[&keypair], transaction.message.recent_blockhash);

        // Serialize the transaction to base64
        let serialized_tx = base64::encode(bincode::serialize(&transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize transaction: {}", e)))?);

        // Send transaction with preflight checks and specific commitment
        let params = serde_json::json!([
            serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "finalized",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        let result: serde_json::Value = self.send_request("sendTransaction", params).await?;
        Ok(result.to_string())
    }

    pub async fn update_user_profile(
        &self,
        pubkey: &str,
        username: Option<String>,
        profile_image: Option<String>,
        keypair_bytes: &[u8]
    ) -> Result<String, RpcError> {
        use solana_sdk::{
            signature::{Keypair, Signer},
            instruction::{AccountMeta, Instruction},
            transaction::Transaction,
            message::Message,
        };

        // Program ID
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))?;
        
        let target_pubkey = Pubkey::from_str(pubkey)
            .map_err(|e| RpcError::Other(format!("Invalid public key: {}", e)))?;

        // Validate inputs
        if let Some(ref username) = username {
            if username.len() > 32 {
                return Err(RpcError::Other("Username too long. Maximum length is 32 characters.".to_string()));
            }
        }
        if let Some(ref profile_image) = profile_image {
            if profile_image.len() > 256 {
                return Err(RpcError::Other("Profile image too long. Maximum length is 256 characters.".to_string()));
            }
            if !profile_image.starts_with("n:") && !profile_image.starts_with("c:") {
                return Err(RpcError::Other("Profile image must start with 'n:' or 'c:' prefix.".to_string()));
            }
        }

        // Create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;

        // Calculate user profile PDA
        let (user_profile_pda, _) = Pubkey::find_program_address(
            &[b"user_profile", target_pubkey.as_ref()],
            &program_id
        );

        // Get latest blockhash with specific commitment
        let blockhash: serde_json::Value = self.send_request(
            "getLatestBlockhash",
            serde_json::json!([{
                "commitment": "finalized",
                "minContextSlot": 0
            }])
        ).await?;

        let recent_blockhash = blockhash["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get blockhash".to_string()))?;

        // Construct the instruction data
        let mut instruction_data = Vec::new();
        
        // Add discriminator [79, 75, 114, 130, 68, 123, 180, 11]
        instruction_data.extend_from_slice(&[79, 75, 114, 130, 68, 123, 180, 11]);
        
        // Add username option
        if let Some(username) = username {
            instruction_data.push(1); // Some variant
            instruction_data.extend_from_slice(&(username.len() as u32).to_le_bytes());
            instruction_data.extend_from_slice(username.as_bytes());
        } else {
            instruction_data.push(0); // None variant
        }
        
        // Add profile_image option
        if let Some(profile_image) = profile_image {
            instruction_data.push(1); // Some variant
            instruction_data.extend_from_slice(&(profile_image.len() as u32).to_le_bytes());
            instruction_data.extend_from_slice(profile_image.as_bytes());
        } else {
            instruction_data.push(0); // None variant
        }

        // Create the instruction
        let instruction = Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(target_pubkey, true),     // user (signer)
                AccountMeta::new(user_profile_pda, false), // user_profile PDA
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // System Program
            ],
        );

        // Create the message
        let message = Message::new(
            &[instruction],
            Some(&target_pubkey), // fee payer
        );

        // Create and sign transaction
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        transaction.sign(&[&keypair], transaction.message.recent_blockhash);

        // Serialize the transaction to base64
        let serialized_tx = base64::encode(bincode::serialize(&transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize transaction: {}", e)))?);

        // Send transaction with preflight checks and specific commitment
        let params = serde_json::json!([
            serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "finalized",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        let result: serde_json::Value = self.send_request("sendTransaction", params).await?;
        Ok(result.to_string())
    }
}

// implement the default trait
impl Default for RpcConnection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    use wasm_bindgen_test::console_log;

    wasm_bindgen_test_configure!(run_in_browser);

    fn log_info(msg: &str) {
        console_log!("ℹ️  {}", msg);
    }

    fn log_error(msg: &str) {
        console_log!("❌ {}", msg);
    }

    fn log_success(msg: &str) {
        console_log!("✅ {}", msg);
    }

    fn log_json(prefix: &str, value: &serde_json::Value) {
        console_log!("📄 {}:", prefix);
        match serde_json::to_string_pretty(value) {
            Ok(pretty) => {
                for line in pretty.lines() {
                    console_log!("   {}", line);
                }
            }
            Err(e) => log_error(&format!("Failed to format JSON: {}", e))
        }
    }

    fn print_separator() {
        console_log!("\n----------------------------------------");
    }

    fn load_test_wallet() -> Result<(String, Vec<u8>), RpcError> {
        let keypair_json = include_str!("../../test-keypair/memo-test-keypair.json");
        
        let keypair_bytes: Vec<u8> = serde_json::from_str(keypair_json)
            .map_err(|e| RpcError::Other(format!("Failed to parse keypair JSON: {}", e)))?;
        
        let pubkey = bs58::encode(&keypair_bytes[32..64]).into_string();
        log_info(&format!("Successfully loaded wallet from embedded keypair file"));
        Ok((pubkey, keypair_bytes))
    }

    #[wasm_bindgen_test]
    async fn test_get_version() {
        print_separator();
        log_info("Starting version test");
        
        let rpc = RpcConnection::new();
        log_info(&format!("Using RPC endpoint: {}", RpcConnection::DEFAULT_RPC_ENDPOINT));
        
        match rpc.get_version().await {
            Ok(version) => {
                print_separator();
                
                let version_value: serde_json::Value = serde_json::from_str(&version)
                    .expect("Failed to parse version JSON");
                
                log_json("RPC Version Response", &version_value);
                
                assert!(version_value.is_object(), "Version response should be an object");
                assert!(version_value.get("solana-core").is_some(), "Should contain solana-core version");
                
                print_separator();
                log_success("Version test completed successfully");
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Version test failed: {}", e));
                panic!("Test failed");
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_get_balance() {
        print_separator();
        log_info("Starting balance test");

        match load_test_wallet() {
            Ok((pubkey, _)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", RpcConnection::DEFAULT_RPC_ENDPOINT));

                match rpc.get_balance(&pubkey).await {
                    Ok(balance_response) => {
                        print_separator();
                        
                        let balance_value: serde_json::Value = serde_json::from_str(&balance_response)
                            .expect("Failed to parse balance JSON");
                        
                        log_json("Balance Response", &balance_value);

                        assert!(balance_value.get("value").is_some(), "Response should contain 'value' field");

                        if let Some(lamports) = balance_value.get("value").and_then(|v| v.as_u64()) {
                            let sol = lamports as f64 / 1_000_000_000.0;
                            log_info(&format!("\nWallet balance: {} SOL ({} lamports)", sol, lamports));
                        }

                        print_separator();
                        log_success("Balance test completed successfully");
                    },
                    Err(e) => {
                        print_separator();
                        log_error(&format!("Failed to get balance: {}", e));
                        panic!("Balance test failed");
                    }
                }
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_a3_get_user_profile() {
        print_separator();
        log_info("Starting user profile test sequence (3/4): Get Profile");

        // using load_test_wallet to get test wallet pubkey
        match load_test_wallet() {
            Ok((pubkey, _)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", RpcConnection::DEFAULT_RPC_ENDPOINT));

                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        print_separator();
                        log_info("Raw account info received:");
                        log_info(&account_info_str);

                        // parse account info JSON
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        // get base64 encoded data
                        if let Some(data) = account_info["value"]["data"].get(0).and_then(|v| v.as_str()) {
                            // decode base64 data
                            let decoded = base64::decode(data)
                                .expect("Failed to decode base64 data");

                            // start parsing data structure (simulate UI behavior)
                            let mut data = &decoded[8..]; // Skip discriminator

                            // Read pubkey
                            let mut pubkey_bytes = [0u8; 32];
                            pubkey_bytes.copy_from_slice(&data[..32]);
                            let account_pubkey = Pubkey::new_from_array(pubkey_bytes);
                            data = &data[32..];

                            // Read username
                            let username_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
                            data = &data[4..];
                            let username = String::from_utf8(data[..username_len].to_vec())
                                .expect("Invalid username encoding");
                            data = &data[username_len..];

                            // Read stats
                            let total_minted = u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            let total_burned = u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            let mint_count = u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            let burn_count = u64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            // Read profile image
                            let profile_image_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
                            data = &data[4..];
                            let profile_image = String::from_utf8(data[..profile_image_len].to_vec())
                                .expect("Invalid profile image encoding");
                            data = &data[profile_image_len..];

                            // Read timestamps
                            let created_at = i64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);
                            data = &data[8..];

                            let last_updated = i64::from_le_bytes([
                                data[0], data[1], data[2], data[3], 
                                data[4], data[5], data[6], data[7]
                            ]);

                            // display parsed user info
                            print_separator();
                            log_info("==== USER PROFILE ====");
                            log_info(&format!("Username: {}", username));
                            log_info(&format!("Profile Image: {}", 
                                if profile_image.is_empty() { "None" } else { &profile_image }));
                            
                            log_info("\n==== TOKEN STATISTICS ====");
                            log_info(&format!("Total Minted: {} tokens", total_minted));
                            log_info(&format!("Total Burned: {} tokens", total_burned));
                            log_info(&format!("Net Balance: {} tokens", 
                                (total_minted as i64 - total_burned as i64)));
                            log_info(&format!("Mint Operations: {}", mint_count));
                            log_info(&format!("Burn Operations: {}", burn_count));
                            
                            log_info("\n==== ACCOUNT INFO ====");
                            log_info(&format!("Owner: {}", account_pubkey));
                            log_info(&format!("Created: {}", format_timestamp(created_at)));
                            log_info(&format!("Last Updated: {}", format_timestamp(last_updated)));

                            if !profile_image.is_empty() {
                                log_info("\n==== PIXEL ART ====");
                                display_pixel_art(&profile_image);
                            }
                        } else {
                            log_error("No account data found");
                        }

                        print_separator();
                        log_success("User profile test completed successfully");
                    },
                    Err(e) => {
                        print_separator();
                        log_error(&format!("Failed to get user profile: {}", e));
                        log_error("Error occurred during RPC call");
                        panic!("User profile test failed");
                    }
                }
            },
            Err(e) => {
                print_separator();
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_a4_close_user_profile() {
        print_separator();
        log_info("Starting user profile test sequence (4/4): Close Profile");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", RpcConnection::DEFAULT_RPC_ENDPOINT));

                // first check if user profile exists
                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        if account_info["value"].is_null() {
                            log_info("No user profile found to close");
                            return;
                        }

                        // if account exists, attempt to close it
                        log_info("Found existing user profile, attempting to close...");
                        
                        match rpc.close_user_profile(&pubkey, &keypair_bytes).await {
                            Ok(response) => {
                                // print raw response
                                print_separator();
                                log_info("Raw Close Profile Response:");
                                log_info(&response);
                                print_separator();

                                // try to parse response as JSON (but don't let parsing fail affect test flow)
                                match serde_json::from_str::<serde_json::Value>(&response) {
                                    Ok(json_response) => {
                                        log_json("Parsed Close Profile Response", &json_response);
                                    }
                                    Err(e) => {
                                        log_error(&format!("Failed to parse response as JSON: {}", e));
                                    }
                                }

                                // wait for transaction confirmation
                                log_info("Waiting for transaction confirmation...");
                                
                                // try 10 times, 10 seconds interval
                                for i in 1..=10 {
                                    // use gloo_timers future version for delay
                                    gloo_timers::future::TimeoutFuture::new(10_000).await;
                                    
                                    log_info(&format!("Checking account status (attempt {}/10)...", i));
                                    
                                    match rpc.get_user_profile(&pubkey).await {
                                        Ok(verify_info_str) => {
                                            let verify_info: serde_json::Value = serde_json::from_str(&verify_info_str)
                                                .expect("Failed to parse verification info JSON");

                                            if verify_info["value"].is_null() {
                                                log_success("User profile successfully closed and removed");
                                                print_separator();
                                                log_success("Close user profile test completed");
                                                return;
                                            } else {
                                                log_info("Profile still exists, waiting for confirmation...");
                                            }
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to verify account closure: {}", e));
                                        }
                                    }
                                }

                                // if all attempts fail
                                log_error("Profile still exists after maximum retries");
                                panic!("Close operation failed - account still exists after timeout");
                            },
                            Err(e) => {
                                log_error(&format!("Failed to close user profile: {}", e));
                                panic!("Close operation failed");
                            }
                        }
                    },
                    Err(e) => {
                        log_error(&format!("Failed to check user profile: {}", e));
                        panic!("Failed to check user profile existence");
                    }
                }
            },
            Err(e) => {
                print_separator();  
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_a1_initialize_user_profile() {
        print_separator();
        log_info("Starting user profile test sequence (1/4): Initialize Profile");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", RpcConnection::DEFAULT_RPC_ENDPOINT));

                // First check if user profile already exists
                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        if !account_info["value"].is_null() {
                            log_info("User profile already exists, skipping initialization");
                            return;
                        }

                        // If account doesn't exist, proceed with initialization
                        log_info("No existing user profile found, proceeding with initialization...");
                        
                        // Generate a simple profile image
                        let profile_image = "n:3UZcHVQ0*UD`75D)/9W9[@$E#F#+ddL^$7+a/AVJ7R7SKW?0$V@<3DaVT'(V?VHKB=N-%K3bJ^BH-cdGP33]cB9I`&KH*D)X#XF#V$S[VH%CI_=P--_]*T&]^`?>N?.aNJ)V8.W8Z&V/DZ9I+0?0BbD^VV]/0aGa=,G6d456c`#";
                        
                        match rpc.initialize_user_profile(&pubkey, "TestUser", profile_image, &keypair_bytes).await {
                            Ok(response) => {
                                // Print raw response
                                print_separator();
                                log_info("Raw Initialize Profile Response:");
                                log_info(&response);
                                print_separator();

                                // Try to parse response as JSON (but don't let parsing fail affect test flow)
                                match serde_json::from_str::<serde_json::Value>(&response) {
                                    Ok(json_response) => {
                                        log_json("Parsed Initialize Profile Response", &json_response);
                                    }
                                    Err(e) => {
                                        log_error(&format!("Failed to parse response as JSON: {}", e));
                                    }
                                }

                                // Wait for transaction confirmation
                                log_info("Waiting for transaction confirmation...");
                                
                                // Try 10 times, 10 seconds interval
                                for i in 1..=10 {
                                    // Use gloo_timers future version for delay
                                    gloo_timers::future::TimeoutFuture::new(10_000).await;
                                    
                                    log_info(&format!("Checking account status (attempt {}/10)...", i));
                                    
                                    match rpc.get_user_profile(&pubkey).await {
                                        Ok(verify_info_str) => {
                                            let verify_info: serde_json::Value = serde_json::from_str(&verify_info_str)
                                                .expect("Failed to parse verification info JSON");

                                            if !verify_info["value"].is_null() {
                                                log_success("User profile successfully initialized");
                                                print_separator();
                                                log_success("Initialize user profile test completed");
                                                return;
                                            } else {
                                                log_info("Profile not yet created, waiting for confirmation...");
                                            }
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to verify account creation: {}", e));
                                        }
                                    }
                                }

                                // If all attempts fail
                                log_error("Profile not created after maximum retries");
                                panic!("Initialize operation failed - account not created after timeout");
                            },
                            Err(e) => {
                                log_error(&format!("Failed to initialize user profile: {}", e));
                                panic!("Initialize operation failed");
                            }
                        }
                    },
                    Err(e) => {
                        log_error(&format!("Failed to check user profile: {}", e));
                        panic!("Failed to check user profile existence");
                    }
                }
            },
            Err(e) => {
                print_separator();  
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    #[wasm_bindgen_test]
    async fn test_a2_update_user_profile() {
        print_separator();
        log_info("Starting user profile test sequence (2/4): Update Profile");

        match load_test_wallet() {
            Ok((pubkey, keypair_bytes)) => {
                log_info(&format!("Test wallet public key: {}", pubkey));

                let rpc = RpcConnection::new();
                log_info(&format!("Using RPC endpoint: {}", RpcConnection::DEFAULT_RPC_ENDPOINT));

                // First check if user profile exists
                match rpc.get_user_profile(&pubkey).await {
                    Ok(account_info_str) => {
                        let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                            .expect("Failed to parse account info JSON");

                        if account_info["value"].is_null() {
                            log_info("No user profile found to update");
                            return;
                        }

                        // If account exists, proceed with update
                        log_info("Found existing user profile, attempting to update...");
                        
                        // Update username only
                        let new_username = Some("UpdatedUser".to_string());
                        let profile_image = None;
                        
                        match rpc.update_user_profile(&pubkey, new_username, profile_image, &keypair_bytes).await {
                            Ok(response) => {
                                // Print raw response
                                print_separator();
                                log_info("Raw Update Profile Response:");
                                log_info(&response);
                                print_separator();

                                // Try to parse response as JSON
                                match serde_json::from_str::<serde_json::Value>(&response) {
                                    Ok(json_response) => {
                                        log_json("Parsed Update Profile Response", &json_response);
                                    }
                                    Err(e) => {
                                        log_error(&format!("Failed to parse response as JSON: {}", e));
                                    }
                                }

                                // Wait for transaction confirmation
                                log_info("Waiting for transaction confirmation...");
                                
                                // Try 10 times, 10 seconds interval
                                for i in 1..=10 {
                                    gloo_timers::future::TimeoutFuture::new(10_000).await;
                                    
                                    log_info(&format!("Checking account status (attempt {}/10)...", i));
                                    
                                    match rpc.get_user_profile(&pubkey).await {
                                        Ok(verify_info_str) => {
                                            let verify_info: serde_json::Value = serde_json::from_str(&verify_info_str)
                                                .expect("Failed to parse verification info JSON");

                                            if !verify_info["value"].is_null() {
                                                log_success("User profile successfully updated");
                                                print_separator();
                                                log_success("Update user profile test completed");
                                                return;
                                            } else {
                                                log_info("Profile update not yet confirmed, waiting...");
                                            }
                                        },
                                        Err(e) => {
                                            log_error(&format!("Failed to verify account update: {}", e));
                                        }
                                    }
                                }

                                // If all attempts fail
                                log_error("Profile update not confirmed after maximum retries");
                                panic!("Update operation failed - changes not confirmed after timeout");
                            },
                            Err(e) => {
                                log_error(&format!("Failed to update user profile: {}", e));
                                panic!("Update operation failed");
                            }
                        }
                    },
                    Err(e) => {
                        log_error(&format!("Failed to check user profile: {}", e));
                        panic!("Failed to check user profile existence");
                    }
                }
            },
            Err(e) => {
                print_separator();  
                log_error(&format!("Failed to load test wallet: {}", e));
                panic!("Failed to load wallet");
            }
        }
    }

    // Helper functions for the test
    fn format_timestamp(timestamp: i64) -> String {
        let secs = timestamp as u64;
        let days = secs / 86400;
        let hours = (secs % 86400) / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;
        
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            1970 + (days / 365),
            ((days % 365) / 30) + 1,
            ((days % 365) % 30) + 1,
            hours,
            minutes,
            seconds
        )
    }

    fn display_pixel_art(profile_image: &str) {
        if profile_image.is_empty() {
            return;
        }

        // parse prefix and data
        let (prefix, data) = match profile_image.split_once(':') {
            Some(("c", compressed)) => {
                // handle compressed data
                match decompress_with_deflate(compressed) {
                    Ok(decompressed) => ("n", decompressed),
                    Err(e) => {
                        log_error(&format!("Error decompressing profile image: {}", e));
                        return;
                    }
                }
            },
            Some(("n", uncompressed)) => ("n", uncompressed.to_string()),
            _ => {
                log_error("Invalid profile image format");
                return;
            }
        };

        // display pixel art
        log_info("\nPixel Art Representation:");
        let mut bit_count = 0;

        for c in data.chars() {
            if let Some(value) = map_from_safe_char(c) {
                for i in (0..6).rev() {
                    let bit = (value & (1 << i)) != 0;
                    console_log!("{}", if bit { "⬛" } else { "⬜" });
                    bit_count += 1;
                    
                    if bit_count % 32 == 0 {
                        console_log!("\n");
                    }
                }
            }
        }
        console_log!("\n");
    }

    fn map_from_safe_char(c: char) -> Option<u8> {
        let ascii = c as u8;
        
        if c == ':' || c == '\\' || c == '"' {
            return None;
        }
        
        if ascii < 35 || ascii > 126 {
            return None;
        }
        
        let mut value = ascii - 35;
        if ascii > 92 { value -= 1; }  // adjust '\'
        if ascii > 58 { value -= 1; }  // adjust ':'
        
        if value >= 64 {
            return None;
        }
        
        Some(value)
    }

    fn decompress_with_deflate(input: &str) -> Result<String, String> {
        let bytes = base64::decode(input)
            .map_err(|e| format!("Base64 decode error: {}", e))?;
            
        let mut decoder = DeflateDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| format!("Decompression error: {}", e))?;
            
        let result: String = decompressed.into_iter()
            .map(|b| b as char)
            .collect();
            
        Ok(result)
    }
}