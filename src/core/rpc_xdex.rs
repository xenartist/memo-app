use serde::{Serialize, Deserialize};
use super::rpc_base::{RpcError, RpcConnection};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::instruction::{Instruction, AccountMeta};
use solana_sdk::transaction::Transaction;
use solana_sdk::message::Message;
use solana_sdk::system_instruction;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use std::str::FromStr;

/// xDEX Program ID (from IDL metadata)
pub const XDEX_PROGRAM_ID: &str = "sEsYH97wqmfnkzHedjNcw3zyJdPvUmsa9AixhS4b4fN";

/// USDC mint address (price anchor = 1 USD)
pub const USDC_MINT: &str = "B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq";

/// Native XNT address (43个1)
pub const NATIVE_XNT_MINT: &str = "So11111111111111111111111111111111111111111";

/// Wrapped XNT address (42个1 + 2)
pub const WRAPPED_XNT_MINT: &str = "So11111111111111111111111111111111111111112";

// Compute unit buffer for xDEX operations (default 10% buffer)
const COMPUTE_UNIT_BUFFER: f64 = 1.10;

/// Get well-known token metadata
fn get_well_known_token_metadata(mint: &str) -> Option<TokenMetadata> {
    match mint {
        // Native XNT (43个1)
        "So11111111111111111111111111111111111111111" => Some(TokenMetadata {
            mint: mint.to_string(),
            name: Some("X1 Native Token".to_string()),
            symbol: Some("XNT".to_string()),
            logo_uri: Some("https://app.xdex.xyz/assets/images/tokens/x1.webp".to_string()),
        }),
        // Wrapped XNT (42个1 + 2) - Display as XNT to users
        "So11111111111111111111111111111111111111112" => Some(TokenMetadata {
            mint: mint.to_string(),
            name: Some("X1 Native Token".to_string()),  // 显示为XNT而非WXNT
            symbol: Some("XNT".to_string()),             // 显示为XNT而非WXNT
            logo_uri: Some("https://app.xdex.xyz/assets/images/tokens/x1.webp".to_string()),
        }),
        // USDC.X
        "B69chRzqzDCmdB5WYB8NRu5Yv5ZA95ABiZcdzCgGm9Tq" => Some(TokenMetadata {
            mint: mint.to_string(),
            name: Some("USDC.X".to_string()),
            symbol: Some("USDC.X".to_string()),
            logo_uri: Some("https://x1logos.s3.us-east-1.amazonaws.com/48-usdcx.png".to_string()),
        }),
        "pXNTyoqQsskHdZ7Q1rnP25FEyHHjissbs7n6RRN2nP5" => Some(TokenMetadata {
            mint: mint.to_string(),
            name: Some("Pooled XNT".to_string()),
            symbol: Some("pXNT".to_string()),
            logo_uri: Some("https://x1logos.s3.us-east-1.amazonaws.com/48-pxnt.png".to_string()),
        }),
        "7jP6rm8zEd2kLgt1vRTzmA11MmbhypHRdU1VyZPripp" => Some(TokenMetadata {
            mint: mint.to_string(),
            name: Some("RipperPool XNT".to_string()),
            symbol: Some("rXNT".to_string()),
            logo_uri: Some("https://ipfs.io/ipfs/bafkreia6ukbearwhr7wq3ijooxdfudtqrg6vbme5ebaqgubfulefs3lisu".to_string()),
        }),
        "XBLKLmxhADMVX3DsdwymvHyYbBYfKa5eKhtpiQ2kj7T" => Some(TokenMetadata {
            mint: mint.to_string(),
            name: Some("XenBlocks".to_string()),
            symbol: Some("XBLK".to_string()),
            logo_uri: Some("https://explorer.xenblocks.io/tokens/xblk.png".to_string()),
        }),
        _ => None,
    }
}

/// Pool state account structure (from IDL)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PoolState {
    /// Which config the pool belongs to
    pub amm_config: String,
    /// Pool creator
    pub pool_creator: String,
    /// Token A vault
    pub token_0_vault: String,
    /// Token B vault
    pub token_1_vault: String,
    /// LP token mint
    pub lp_mint: String,
    /// Token A mint
    pub token_0_mint: String,
    /// Token B mint
    pub token_1_mint: String,
    /// Token 0 program
    pub token_0_program: String,
    /// Token 1 program
    pub token_1_program: String,
    /// Observation account
    pub observation_key: String,
    /// Auth bump
    pub auth_bump: u8,
    /// Pool status (bitwise flags)
    pub status: u8,
    /// LP mint decimals
    pub lp_mint_decimals: u8,
    /// Token 0 decimals
    pub mint_0_decimals: u8,
    /// Token 1 decimals
    pub mint_1_decimals: u8,
    /// LP supply
    pub lp_supply: u64,
    /// Protocol fees for token 0
    pub protocol_fees_token_0: u64,
    /// Protocol fees for token 1
    pub protocol_fees_token_1: u64,
    /// Fund fees for token 0
    pub fund_fees_token_0: u64,
    /// Fund fees for token 1
    pub fund_fees_token_1: u64,
    /// Open time timestamp
    pub open_time: u64,
    /// Recent epoch
    pub recent_epoch: u64,
}

/// Pool information with parsed data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    /// Pool account address
    pub address: String,
    /// Token A mint address
    pub token_0_mint: String,
    /// Token B mint address
    pub token_1_mint: String,
    /// Token A decimals
    pub token_0_decimals: u8,
    /// Token B decimals
    pub token_1_decimals: u8,
    /// Token A vault address
    pub token_0_vault: String,
    /// Token B vault address
    pub token_1_vault: String,
    /// LP supply
    pub lp_supply: u64,
    /// Pool status (0=all enabled, 1=deposit disabled, 2=withdraw disabled, 4=swap disabled)
    pub status: u8,
    /// Pool creator
    pub pool_creator: String,
}

/// Pool price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPrice {
    /// Pool address
    pub pool_address: String,
    /// Token 0 reserve (in tokens, not lamports)
    pub reserve_0: f64,
    /// Token 1 reserve (in tokens, not lamports)
    pub reserve_1: f64,
    /// Price of token1 in terms of token0
    pub price: f64,
    /// USD price of token0 (if available)
    pub token_0_usd_price: Option<f64>,
    /// USD price of token1 (if available)
    pub token_1_usd_price: Option<f64>,
}

/// Token metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    /// Token mint address
    pub mint: String,
    /// Token name
    pub name: Option<String>,
    /// Token symbol
    pub symbol: Option<String>,
    /// Token logo URI
    pub logo_uri: Option<String>,
}

/// xDEX connection for direct contract interaction
pub struct XDexConnection {
    rpc: RpcConnection,
    program_id: Pubkey,
}

impl XDexConnection {
    /// Create a new xDEX connection
    pub fn new() -> Self {
        Self {
            rpc: RpcConnection::new(),
            program_id: Pubkey::from_str(XDEX_PROGRAM_ID)
                .expect("Invalid xDEX program ID"),
        }
    }

    /// Get all pools from the xDEX program
    /// 
    /// This queries all PoolState accounts owned by the xDEX program
    pub async fn get_all_pools(&self) -> Result<Vec<PoolInfo>, RpcError> {
        // Use getProgramAccounts to fetch all pool accounts
        let params = serde_json::json!([
            XDEX_PROGRAM_ID,
            {
                "encoding": "base64"
            }
        ]);
        
        let result: serde_json::Value = self.rpc.send_request("getProgramAccounts", params).await?;
        
        let mut pools = Vec::new();
        
        // Parse the accounts
        if let Some(accounts) = result.as_array() {
            for account in accounts.iter() {
                if let Some(pubkey) = account.get("pubkey").and_then(|p| p.as_str()) {
                    if let Some(account_data) = account.get("account") {
                        if let Some(data_array) = account_data.get("data").and_then(|d| d.as_array()) {
                            if data_array.len() >= 1 {
                                if let Some(data_str) = data_array[0].as_str() {
                                    // Decode base64 data
                                    match base64::decode(data_str) {
                                        Ok(decoded) => {
                                            // Parse pool state from raw bytes
                                            if let Ok(pool_info) = Self::parse_pool_state(pubkey, &decoded) {
                                                pools.push(pool_info);
                                            }
                                        },
                                        Err(e) => {
                                            log::error!("Failed to decode base64: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(pools)
    }

    /// Parse pool state from raw account data
    fn parse_pool_state(address: &str, data: &[u8]) -> Result<PoolInfo, RpcError> {
        // Ensure we have enough data (at least basic structure)
        if data.len() < 400 {
            return Err(RpcError::Other(format!("Data too short: {} bytes (need at least 400)", data.len())));
        }

        // Parse pool state structure (offsets based on IDL)
        // Note: This is a simplified parser. For production, consider using borsh deserialization
        
        // Skip discriminator (8 bytes) - Anchor uses 8-byte discriminator
        let offset = 8;
        
        // ammConfig: PublicKey (32 bytes) at offset 8
        let _amm_config = Self::read_pubkey(data, offset)?;
        
        // poolCreator: PublicKey (32 bytes) at offset 40
        let pool_creator = Self::read_pubkey(data, 40)?;
        
        // token0Vault: PublicKey (32 bytes) at offset 72
        let token_0_vault = Self::read_pubkey(data, 72)?;
        
        // token1Vault: PublicKey (32 bytes) at offset 104
        let token_1_vault = Self::read_pubkey(data, 104)?;
        
        // lpMint: PublicKey (32 bytes) at offset 136
        let _lp_mint = Self::read_pubkey(data, 136)?;
        
        // token0Mint: PublicKey (32 bytes) at offset 168
        let token_0_mint = Self::read_pubkey(data, 168)?;
        
        // token1Mint: PublicKey (32 bytes) at offset 200
        let token_1_mint = Self::read_pubkey(data, 200)?;
        
        // token0Program: PublicKey (32 bytes) at offset 232
        // token1Program: PublicKey (32 bytes) at offset 264
        // observationKey: PublicKey (32 bytes) at offset 296
        
        // authBump: u8 at offset 328
        // status: u8 at offset 329
        let status = if data.len() > 329 { data[329] } else { 0 };
        
        // lpMintDecimals: u8 at offset 330
        // mint0Decimals: u8 at offset 331
        let token_0_decimals = if data.len() > 331 { data[331] } else { 0 };
        
        // mint1Decimals: u8 at offset 332
        let token_1_decimals = if data.len() > 332 { data[332] } else { 0 };
        
        // lpSupply: u64 at offset 333
        let lp_supply = if data.len() >= 341 {
            u64::from_le_bytes(
                data[333..341].try_into()
                    .map_err(|_| RpcError::Other("Failed to parse lp_supply".to_string()))?
            )
        } else {
            0
        };
        
        Ok(PoolInfo {
            address: address.to_string(),
            token_0_mint,
            token_1_mint,
            token_0_decimals,
            token_1_decimals,
            token_0_vault,
            token_1_vault,
            lp_supply,
            status,
            pool_creator,
        })
    }

    /// Helper to read a PublicKey from raw bytes
    fn read_pubkey(data: &[u8], offset: usize) -> Result<String, RpcError> {
        if data.len() < offset + 32 {
            return Err(RpcError::Other("Insufficient data to read pubkey".to_string()));
        }
        
        let pubkey_bytes = &data[offset..offset + 32];
        let pubkey = Pubkey::new_from_array(
            pubkey_bytes.try_into()
                .map_err(|_| RpcError::Other("Invalid pubkey bytes".to_string()))?
        );
        
        Ok(pubkey.to_string())
    }

    /// Find pools for a specific token pair
    pub async fn find_pools_for_pair(
        &self,
        token_a: &str,
        token_b: &str,
    ) -> Result<Vec<PoolInfo>, RpcError> {
        let all_pools = self.get_all_pools().await?;
        
        let filtered_pools: Vec<PoolInfo> = all_pools
            .into_iter()
            .filter(|pool| {
                (pool.token_0_mint == token_a && pool.token_1_mint == token_b)
                    || (pool.token_0_mint == token_b && pool.token_1_mint == token_a)
            })
            .collect();
        
        Ok(filtered_pools)
    }

    /// Get token account balance
    async fn get_token_account_balance(&self, account_address: &str) -> Result<u64, RpcError> {
        let params = serde_json::json!([
            account_address,
            {
                "encoding": "jsonParsed"
            }
        ]);
        
        let result: serde_json::Value = self.rpc.send_request("getAccountInfo", params).await?;
        
        // Extract balance from jsonParsed format
        if let Some(value) = result.get("value") {
            // Check if account exists
            if value.is_null() {
                return Err(RpcError::Other(format!("Account {} does not exist", account_address)));
            }
            
            if let Some(data) = value.get("data") {
                if let Some(parsed) = data.get("parsed") {
                    if let Some(info) = parsed.get("info") {
                        if let Some(token_amount) = info.get("tokenAmount") {
                            if let Some(amount_str) = token_amount.get("amount").and_then(|a| a.as_str()) {
                                let balance = amount_str.parse::<u64>()
                                    .map_err(|e| RpcError::Other(format!("Failed to parse balance: {}", e)))?;
                                return Ok(balance);
                            }
                        }
                    }
                }
            }
        }
        
        Err(RpcError::Other(format!("Failed to extract token balance from account info for {}", account_address)))
    }

    /// Get pool price information
    /// 
    /// Returns the current price in the pool by querying vault balances
    pub async fn get_pool_price(&self, pool_info: &PoolInfo) -> Result<PoolPrice, RpcError> {
        // Get vault balances
        let vault_0_balance = self.get_token_account_balance(&pool_info.token_0_vault).await?;
        let vault_1_balance = self.get_token_account_balance(&pool_info.token_1_vault).await?;
        
        // Convert to actual token amounts (considering decimals)
        let reserve_0 = vault_0_balance as f64 / 10_f64.powi(pool_info.token_0_decimals as i32);
        let reserve_1 = vault_1_balance as f64 / 10_f64.powi(pool_info.token_1_decimals as i32);
        
        // Calculate price (token0 per token1)
        // price = reserve_0 / reserve_1
        // 表示：1个token1值多少个token0
        let price = if reserve_1 > 0.0 {
            reserve_0 / reserve_1
        } else {
            0.0
        };
        
        // Check if either token is USDC to calculate USD prices
        let (token_0_usd_price, token_1_usd_price) = if pool_info.token_0_mint == USDC_MINT {
            // token_0 is USDC
            (Some(1.0), Some(price))
        } else if pool_info.token_1_mint == USDC_MINT {
            // token_1 is USDC
            (Some(1.0 / price), Some(1.0))
        } else {
            // Neither is USDC, can't determine USD price directly
            (None, None)
        };
        
        Ok(PoolPrice {
            pool_address: pool_info.address.clone(),
            reserve_0,
            reserve_1,
            price,
            token_0_usd_price,
            token_1_usd_price,
        })
    }

    /// Get USD price for a token by finding its USDC pool
    /// 
    /// Returns the USD price if a USDC pool exists for the token
    pub async fn get_token_usd_price(&self, token_mint: &str) -> Result<f64, RpcError> {
                
        // If it's USDC itself, return 1.0
        if token_mint == USDC_MINT {
            return Ok(1.0);
        }
        
        // Find USDC pool for this token
        let pools = self.find_pools_for_pair(token_mint, USDC_MINT).await?;
        
        if pools.is_empty() {
            return Err(RpcError::Other(format!("No USDC pool found for token {}", token_mint)));
        }
        
        // Use the first pool (in production, you might want to check liquidity)
        let pool = &pools[0];
        let pool_price = self.get_pool_price(pool).await?;
        
        // Determine which side is the token and which is USDC
        let usd_price = if pool.token_0_mint == token_mint {
            pool_price.token_0_usd_price.unwrap_or(0.0)
        } else {
            pool_price.token_1_usd_price.unwrap_or(0.0)
        };
        
                
        Ok(usd_price)
    }

    /// Fetch token metadata JSON from URI
    async fn fetch_metadata_json(&self, uri: &str) -> Result<serde_json::Value, RpcError> {
                
        let window = web_sys::window()
            .ok_or_else(|| RpcError::Other("No window object".to_string()))?;
        
        let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(uri))
            .await
            .map_err(|e| RpcError::Other(format!("Failed to fetch metadata JSON: {:?}", e)))?;
        
        use wasm_bindgen::JsCast;
        let resp: web_sys::Response = resp_value.dyn_into()
            .map_err(|_| RpcError::Other("Failed to cast to Response".to_string()))?;
        
        if !resp.ok() {
            return Err(RpcError::Other(format!("HTTP error: {}", resp.status())));
        }
        
        let json = wasm_bindgen_futures::JsFuture::from(
            resp.json().map_err(|e| RpcError::Other(format!("Failed to parse JSON: {:?}", e)))?
        )
        .await
        .map_err(|e| RpcError::Other(format!("Failed to get JSON: {:?}", e)))?;
        
        let json_str = js_sys::JSON::stringify(&json)
            .map_err(|_| RpcError::Other("Failed to stringify JSON".to_string()))?
            .as_string()
            .ok_or_else(|| RpcError::Other("Failed to convert to string".to_string()))?;
        
        let data: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| RpcError::Other(format!("Failed to parse JSON: {}", e)))?;
        
        Ok(data)
    }

    /// Get token metadata (name, symbol, logo)
    /// 
    /// Queries token metadata from chain (supports both SPL Token and Token-2022)
    pub async fn get_token_metadata(&self, token_mint: &str) -> Result<TokenMetadata, RpcError> {
                
        // First, check if it's a well-known token
        if let Some(well_known) = get_well_known_token_metadata(token_mint) {
                        return Ok(well_known);
        }
        
        // Otherwise, try to get mint account info to check for Token-2022 metadata extension
        let params = serde_json::json!([
            token_mint,
            {
                "encoding": "jsonParsed"
            }
        ]);
        
        let result: serde_json::Value = self.rpc.send_request("getAccountInfo", params).await?;
        
                
        let mut metadata = TokenMetadata {
            mint: token_mint.to_string(),
            name: None,
            symbol: None,
            logo_uri: None,
        };
        
        // Try to extract metadata from Token-2022 extensions
        if let Some(value) = result.get("value") {
            if !value.is_null() {
                if let Some(data) = value.get("data") {
                    if let Some(parsed) = data.get("parsed") {
                        if let Some(info) = parsed.get("info") {
                            // Check for metadata extension in Token-2022
                            if let Some(extensions) = info.get("extensions") {
                                if let Some(extensions_array) = extensions.as_array() {
                                                                        for ext in extensions_array {
                                                                                if let Some(ext_obj) = ext.as_object() {
                                            if let Some(extension_type) = ext_obj.get("extension") {
                                                                                                if extension_type.as_str() == Some("tokenMetadata") {
                                                    if let Some(state) = ext_obj.get("state") {
                                                        metadata.name = state.get("name")
                                                            .and_then(|n| n.as_str())
                                                            .map(|s| s.trim_matches('\0').to_string());
                                                        metadata.symbol = state.get("symbol")
                                                            .and_then(|s| s.as_str())
                                                            .map(|s| s.trim_matches('\0').to_string());
                                                        
                                                        let uri = state.get("uri")
                                                            .and_then(|u| u.as_str())
                                                            .map(|s| s.trim_matches('\0').to_string());
                                                        
                                                                                                                
                                                        // If we have a URI, fetch it to get the logo
                                                        if let Some(uri_str) = &uri {
                                                            if !uri_str.is_empty() {
                                                                                                                                match self.fetch_metadata_json(uri_str).await {
                                                                    Ok(json_data) => {
                                                                                                                                                
                                                                        // Extract image/logo from JSON
                                                                        metadata.logo_uri = json_data.get("image")
                                                                            .and_then(|v| v.as_str())
                                                                            .map(|s| s.to_string())
                                                                            .or_else(|| {
                                                                                json_data.get("logo")
                                                                                    .and_then(|v| v.as_str())
                                                                                    .map(|s| s.to_string())
                                                                            });
                                                                        
                                                                                                                                            },
                                                                    Err(e) => {
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
                            }
                        }
                    }
                }
                
                // If we found metadata, return it
                if metadata.symbol.is_some() {
                                        return Ok(metadata);
                }
                
                // If no Token-2022 metadata, try Metaplex metadata (for standard SPL tokens)
                                
                // Calculate Metaplex metadata PDA
                // PDA = findProgramAddress([b"metadata", metadata_program_id, mint_pubkey], metadata_program_id)
                let metadata_program_id = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
                let mint_pubkey = Pubkey::from_str(token_mint)
                    .map_err(|e| RpcError::Other(format!("Invalid mint address: {}", e)))?;
                let metadata_program_pubkey = Pubkey::from_str(metadata_program_id)
                    .map_err(|e| RpcError::Other(format!("Invalid metadata program ID: {}", e)))?;
                
                let seeds = &[
                    b"metadata".as_ref(),
                    metadata_program_pubkey.as_ref(),
                    mint_pubkey.as_ref(),
                ];
                
                let (metadata_pda, _bump) = Pubkey::find_program_address(seeds, &metadata_program_pubkey);
                let metadata_address = metadata_pda.to_string();
                
                                
                // Query the metadata account
                let metadata_params = serde_json::json!([
                    metadata_address,
                    {
                        "encoding": "base64"
                    }
                ]);
                
                match self.rpc.send_request::<_, serde_json::Value>("getAccountInfo", metadata_params).await {
                    Ok(metadata_result) => {
                                                
                        // Parse Metaplex metadata (this is complex, would need full borsh deserialization)
                        // For now, return empty metadata
                                            },
                    Err(e) => {
                                            }
                }
            }
        }
        
                
        Ok(metadata)
    }
    
    /// Build swap transaction using swapBaseInput instruction
    /// 
    /// # Arguments
    /// * `pool_address` - Pool account address
    /// * `user_pubkey` - User's public key
    /// * `input_mint` - Input token mint address
    /// * `output_mint` - Output token mint address
    /// * `amount_in_lamports` - Amount to swap (in lamports/smallest unit)
    /// * `slippage_pct` - Slippage tolerance in percentage (e.g., 1.0 for 1%)
    /// * `input_decimals` - Input token decimals
    /// * `output_decimals` - Output token decimals
    /// 
    /// Returns an unsigned transaction ready to be signed
    pub async fn build_swap_transaction(
        &self,
        pool_address: &str,
        user_pubkey: &Pubkey,
        input_mint: &str,
        output_mint: &str,
        amount_in_lamports: u64,
        slippage_pct: f64,
        _input_decimals: u8,
        output_decimals: u8,
    ) -> Result<Transaction, RpcError> {
                                                        
        // Parse public keys
        let pool_pubkey = Pubkey::from_str(pool_address)
            .map_err(|e| RpcError::Other(format!("Invalid pool address: {}", e)))?;
        let input_mint_pk = Pubkey::from_str(input_mint)
            .map_err(|e| RpcError::Other(format!("Invalid input mint: {}", e)))?;
        let output_mint_pk = Pubkey::from_str(output_mint)
            .map_err(|e| RpcError::Other(format!("Invalid output mint: {}", e)))?;
        
        // Get pool information to find vaults and AMM config
                let pool_params = serde_json::json!([
            pool_address,
            {
                "encoding": "base64"
            }
        ]);
        
        let pool_result: serde_json::Value = self.rpc.send_request("getAccountInfo", pool_params).await?;
        
        // Parse pool data
        let pool_data_str = pool_result
            .get("value")
            .and_then(|v| v.get("data"))
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|s| s.as_str())
            .ok_or_else(|| RpcError::Other("Failed to get pool data".to_string()))?;
        
        let pool_data = base64::decode(pool_data_str)
            .map_err(|e| RpcError::Other(format!("Failed to decode pool data: {}", e)))?;
        
        // Parse pool state to get vaults and other info
        let pool_info = Self::parse_pool_state(pool_address, &pool_data)?;
        
                        
        // Get AMM config address from pool data (offset 8, first 32 bytes after discriminator)
        let amm_config_str = Self::read_pubkey(&pool_data, 8)?;
        let amm_config = Pubkey::from_str(&amm_config_str)
            .map_err(|e| RpcError::Other(format!("Invalid AMM config: {}", e)))?;
                
        // Determine which is input and which is output
        let (input_vault, output_vault, input_token_program_str, output_token_program_str) = 
            if pool_info.token_0_mint == input_mint {
                // Input is token0, output is token1
                                (
                    pool_info.token_0_vault.clone(),
                    pool_info.token_1_vault.clone(),
                    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb", // Token-2022 program
                    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
                )
            } else {
                // Input is token1, output is token0
                                (
                    pool_info.token_1_vault.clone(),
                    pool_info.token_0_vault.clone(),
                    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
                    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
                )
            };
        
        let input_vault_pk = Pubkey::from_str(&input_vault)
            .map_err(|e| RpcError::Other(format!("Invalid input vault: {}", e)))?;
        let output_vault_pk = Pubkey::from_str(&output_vault)
            .map_err(|e| RpcError::Other(format!("Invalid output vault: {}", e)))?;
        
        // 获取当前池子价格来估算最小输出
                let pool_price = self.get_pool_price(&pool_info).await?;
        
        // 根据价格和滑点计算最小输出
        // pool_price.price = reserve_0 / reserve_1
        // 表示：1个token1 = price个token0（或者 1个token0 = 1/price个token1）
        let amount_in_tokens = amount_in_lamports as f64 / 10_f64.powi(pool_info.token_0_decimals as i32);
        
        let estimated_output_tokens = if pool_info.token_0_mint == input_mint {
            // 输入是token0, 输出是token1
            // price = reserve_0 / reserve_1
            // 要算：amount个token0可以换多少token1
            // output = amount / price
            amount_in_tokens / pool_price.price
        } else {
            // 输入是token1, 输出是token0
            // output = amount * price
            amount_in_tokens * pool_price.price
        };
        
                
        let minimum_output_tokens = estimated_output_tokens * (1.0 - slippage_pct / 100.0);
        let minimum_amount_out = (minimum_output_tokens * 10_f64.powi(output_decimals as i32)) as u64;
        
                
        // 查找用户的ATA (Associated Token Account)
                
        let associated_token_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
            .map_err(|e| RpcError::Other(format!("Invalid associated token program: {}", e)))?;
        
        let input_token_program_pk = Pubkey::from_str(input_token_program_str)
            .map_err(|e| RpcError::Other(format!("Invalid input token program: {}", e)))?;
        let output_token_program_pk = Pubkey::from_str(output_token_program_str)
            .map_err(|e| RpcError::Other(format!("Invalid output token program: {}", e)))?;
        
        let (input_token_account, _) = Pubkey::find_program_address(
            &[
                user_pubkey.as_ref(),
                input_token_program_pk.as_ref(),
                input_mint_pk.as_ref(),
            ],
            &associated_token_program,
        );
        
        let (output_token_account, _) = Pubkey::find_program_address(
            &[
                user_pubkey.as_ref(),
                output_token_program_pk.as_ref(),
                output_mint_pk.as_ref(),
            ],
            &associated_token_program,
        );
        
                        
        // Check if token accounts exist, and create them if needed
        let mut pre_instructions = Vec::new();
        
        // Check input token account
        let input_ata_params = serde_json::json!([
            input_token_account.to_string(),
            {"encoding": "jsonParsed"}
        ]);
        
        let input_ata_exists = match self.rpc.send_request::<_, serde_json::Value>("getAccountInfo", input_ata_params).await {
            Ok(result) => !result.get("value").map(|v| v.is_null()).unwrap_or(true),
            Err(_) => false,
        };
        
        if !input_ata_exists {
                        
            // Create input ATA instruction
            let create_input_ata_ix = Instruction {
                program_id: associated_token_program,
                accounts: vec![
                    AccountMeta::new(*user_pubkey, true),           // payer (signer)
                    AccountMeta::new(input_token_account, false),   // associated token account
                    AccountMeta::new_readonly(*user_pubkey, false), // owner
                    AccountMeta::new_readonly(input_mint_pk, false), // mint
                    AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system program
                    AccountMeta::new_readonly(input_token_program_pk, false), // token program
                ],
                data: vec![], // Create instruction has no data
            };
            
            pre_instructions.push(create_input_ata_ix);
        } else {
                    }
        
        // Check output token account
        let output_ata_params = serde_json::json!([
            output_token_account.to_string(),
            {"encoding": "jsonParsed"}
        ]);
        
        let output_ata_exists = match self.rpc.send_request::<_, serde_json::Value>("getAccountInfo", output_ata_params).await {
            Ok(result) => !result.get("value").map(|v| v.is_null()).unwrap_or(true),
            Err(_) => false,
        };
        
        if !output_ata_exists {
                        
            // Create output ATA instruction
            let create_output_ata_ix = Instruction {
                program_id: associated_token_program,
                accounts: vec![
                    AccountMeta::new(*user_pubkey, true),            // payer (signer)
                    AccountMeta::new(output_token_account, false),   // associated token account
                    AccountMeta::new_readonly(*user_pubkey, false),  // owner
                    AccountMeta::new_readonly(output_mint_pk, false), // mint
                    AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system program
                    AccountMeta::new_readonly(output_token_program_pk, false), // token program
                ],
                data: vec![], // Create instruction has no data
            };
            
            pre_instructions.push(create_output_ata_ix);
        } else {
                    }
        
        // Get observation account address (offset 296 in pool state, after token programs)
        let observation_key_str = Self::read_pubkey(&pool_data, 296)?;
        let observation_key = Pubkey::from_str(&observation_key_str)
            .map_err(|e| RpcError::Other(format!("Invalid observation key: {}", e)))?;
                
        // Derive authority PDA (pool authority)
        let (authority, _) = Pubkey::find_program_address(
            &[b"vault_and_lp_mint_auth_seed"],
            &self.program_id,
        );
                
        // Build swapBaseInput instruction
        // Discriminator for swapBaseInput: [143, 190, 90, 218, 196, 30, 51, 222]
        let discriminator: [u8; 8] = [143, 190, 90, 218, 196, 30, 51, 222];
        
        // Instruction data: discriminator + amountIn (u64) + minimumAmountOut (u64)
        let mut instruction_data = Vec::new();
        instruction_data.extend_from_slice(&discriminator);
        instruction_data.extend_from_slice(&amount_in_lamports.to_le_bytes());
        instruction_data.extend_from_slice(&minimum_amount_out.to_le_bytes());
        
                
        // Build accounts for swapBaseInput
        let accounts = vec![
            AccountMeta::new_readonly(*user_pubkey, true),         // payer (signer)
            AccountMeta::new_readonly(authority, false),           // authority
            AccountMeta::new_readonly(amm_config, false),          // ammConfig
            AccountMeta::new(pool_pubkey, false),                  // poolState
            AccountMeta::new(input_token_account, false),          // inputTokenAccount
            AccountMeta::new(output_token_account, false),         // outputTokenAccount
            AccountMeta::new(input_vault_pk, false),               // inputVault
            AccountMeta::new(output_vault_pk, false),              // outputVault
            AccountMeta::new_readonly(input_token_program_pk, false), // inputTokenProgram
            AccountMeta::new_readonly(output_token_program_pk, false), // outputTokenProgram
            AccountMeta::new_readonly(input_mint_pk, false),       // inputTokenMint
            AccountMeta::new_readonly(output_mint_pk, false),      // outputTokenMint
            AccountMeta::new(observation_key, false),              // observationState
        ];
        
        let swap_instruction = Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data,
        };
        
                
        // Combine all instructions
        let mut all_instructions = pre_instructions;
        all_instructions.push(swap_instruction);
        
                
        // Get recent blockhash
        let blockhash = self.rpc.get_latest_blockhash().await?;
        
        // Build transaction
        let message = Message::new(&all_instructions, Some(user_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        
                
        Ok(transaction)
    }
    
    /// Build wrap native XNT to WXNT transaction
    /// 
    /// This creates the necessary instructions to wrap native XNT into WXNT (SPL Token)
    /// Note: WXNT uses standard SPL Token program, not Token-2022
    /// 
    /// # Arguments
    /// * `user_pubkey` - User's public key
    /// * `amount_lamports` - Amount of native XNT to wrap (in lamports)
    /// 
    /// Returns an unsigned transaction ready to be signed
    pub async fn build_wrap_xnt_transaction(
        &self,
        user_pubkey: &Pubkey,
        amount_lamports: u64,
    ) -> Result<Transaction, RpcError> {
                                
        let wxnt_mint = Pubkey::from_str(WRAPPED_XNT_MINT)
            .map_err(|e| RpcError::Other(format!("Invalid WXNT mint: {}", e)))?;
        
        // Standard SPL Token program (NOT Token-2022!)
        let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
            .map_err(|e| RpcError::Other(format!("Invalid token program: {}", e)))?;
        
        // Associated Token Program
        let associated_token_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
            .map_err(|e| RpcError::Other(format!("Invalid associated token program: {}", e)))?;
        
        // Derive WXNT associated token account
        let (wxnt_ata, _) = Pubkey::find_program_address(
            &[
                user_pubkey.as_ref(),
                token_program.as_ref(),
                wxnt_mint.as_ref(),
            ],
            &associated_token_program,
        );
        
                
        let mut instructions = Vec::new();
        
        // Check if WXNT ATA exists
        let ata_info_params = serde_json::json!([
            wxnt_ata.to_string(),
            {"encoding": "jsonParsed"}
        ]);
        
        let ata_exists = match self.rpc.send_request::<_, serde_json::Value>("getAccountInfo", ata_info_params).await {
            Ok(result) => {
                !result.get("value").map(|v| v.is_null()).unwrap_or(true)
            },
            Err(_) => false,
        };
        
        if !ata_exists {
                        
            // Create Associated Token Account instruction
            let create_ata_ix = Instruction {
                program_id: associated_token_program,
                accounts: vec![
                    AccountMeta::new(*user_pubkey, true),           // payer (signer)
                    AccountMeta::new(wxnt_ata, false),              // associated token account
                    AccountMeta::new_readonly(*user_pubkey, false), // owner
                    AccountMeta::new_readonly(wxnt_mint, false),    // mint
                    AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system program
                    AccountMeta::new_readonly(token_program, false), // token program
                ],
                data: vec![], // Create instruction has no data
            };
            
            instructions.push(create_ata_ix);
        } else {
                    }
        
        // Transfer native XNT to WXNT ATA
        let transfer_ix = system_instruction::transfer(
            user_pubkey,
            &wxnt_ata,
            amount_lamports,
        );
        instructions.push(transfer_ix);
        
        // SyncNative instruction to update WXNT balance
        // For Token-2022, syncNative discriminator is [17] (instruction index)
        let sync_native_ix = Instruction {
            program_id: token_program,
            accounts: vec![
                AccountMeta::new(wxnt_ata, false), // token account
            ],
            data: vec![17], // SyncNative instruction discriminator
        };
        instructions.push(sync_native_ix);
        
                
        // Get recent blockhash
        let blockhash = self.rpc.get_latest_blockhash().await?;
        
        // Build transaction
        let message = Message::new(&instructions, Some(user_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        
                
        Ok(transaction)
    }
    
    /// Build wrap + swap transaction in one atomic operation
    /// 
    /// This combines wrap and swap into a single transaction,
    /// following the project's pattern of simulating first, then adding compute budget
    /// 
    /// # Arguments
    /// * `pool_address` - Pool account address
    /// * `user_pubkey` - User's public key
    /// * `amount_xnt_lamports` - Amount of native XNT to wrap and swap (in lamports)
    /// * `slippage_pct` - Slippage tolerance in percentage
    /// * `output_mint` - Output token mint (e.g., MEMO)
    /// * `output_decimals` - Output token decimals
    /// 
    /// Returns an unsigned transaction ready to be signed
    pub async fn build_wrap_and_swap_transaction(
        &self,
        pool_address: &str,
        user_pubkey: &Pubkey,
        amount_xnt_lamports: u64,
        slippage_pct: f64,
        output_mint: &str,
        output_decimals: u8,
    ) -> Result<Transaction, RpcError> {
                                                
        // Parse addresses
        let wxnt_mint = Pubkey::from_str(WRAPPED_XNT_MINT)
            .map_err(|e| RpcError::Other(format!("Invalid WXNT mint: {}", e)))?;
        let output_mint_pk = Pubkey::from_str(output_mint)
            .map_err(|e| RpcError::Other(format!("Invalid output mint: {}", e)))?;
        let pool_pubkey = Pubkey::from_str(pool_address)
            .map_err(|e| RpcError::Other(format!("Invalid pool address: {}", e)))?;
        
        // Standard SPL Token program
        let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
            .map_err(|e| RpcError::Other(format!("Invalid token program: {}", e)))?;
        
        // Token-2022 program (for MEMO)
        let token_2022_program = Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
            .map_err(|e| RpcError::Other(format!("Invalid token-2022 program: {}", e)))?;
        
        let associated_token_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
            .map_err(|e| RpcError::Other(format!("Invalid ATA program: {}", e)))?;
        
        // Derive WXNT ATA (using standard SPL Token program)
        let (wxnt_ata, _) = Pubkey::find_program_address(
            &[
                user_pubkey.as_ref(),
                token_program.as_ref(),
                wxnt_mint.as_ref(),
            ],
            &associated_token_program,
        );
        
        // Derive output token ATA
        let (output_ata, _) = Pubkey::find_program_address(
            &[
                user_pubkey.as_ref(),
                token_2022_program.as_ref(),
                output_mint_pk.as_ref(),
            ],
            &associated_token_program,
        );
        
                        
        // Build base instructions (without compute budget)
        let mut base_instructions = Vec::new();
        
        // Check and create WXNT ATA if needed
        let wxnt_ata_params = serde_json::json!([
            wxnt_ata.to_string(),
            {"encoding": "jsonParsed"}
        ]);
        
        let wxnt_ata_exists = match self.rpc.send_request::<_, serde_json::Value>("getAccountInfo", wxnt_ata_params).await {
            Ok(result) => !result.get("value").map(|v| v.is_null()).unwrap_or(true),
            Err(_) => false,
        };
        
        if !wxnt_ata_exists {
                        let create_wxnt_ata_ix = Instruction {
                program_id: associated_token_program,
                accounts: vec![
                    AccountMeta::new(*user_pubkey, true),
                    AccountMeta::new(wxnt_ata, false),
                    AccountMeta::new_readonly(*user_pubkey, false),
                    AccountMeta::new_readonly(wxnt_mint, false),
                    AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                    AccountMeta::new_readonly(token_program, false),
                ],
                data: vec![],
            };
            base_instructions.push(create_wxnt_ata_ix);
        }
        
        // Transfer native XNT to WXNT ATA
        let transfer_ix = system_instruction::transfer(user_pubkey, &wxnt_ata, amount_xnt_lamports);
        base_instructions.push(transfer_ix);
        
        // SyncNative instruction
        let sync_native_ix = Instruction {
            program_id: token_program,
            accounts: vec![AccountMeta::new(wxnt_ata, false)],
            data: vec![17], // SyncNative discriminator
        };
        base_instructions.push(sync_native_ix);
        
        // Check and create output ATA if needed
        let output_ata_params = serde_json::json!([
            output_ata.to_string(),
            {"encoding": "jsonParsed"}
        ]);
        
        let output_ata_exists = match self.rpc.send_request::<_, serde_json::Value>("getAccountInfo", output_ata_params).await {
            Ok(result) => !result.get("value").map(|v| v.is_null()).unwrap_or(true),
            Err(_) => false,
        };
        
        if !output_ata_exists {
                        let create_output_ata_ix = Instruction {
                program_id: associated_token_program,
                accounts: vec![
                    AccountMeta::new(*user_pubkey, true),
                    AccountMeta::new(output_ata, false),
                    AccountMeta::new_readonly(*user_pubkey, false),
                    AccountMeta::new_readonly(output_mint_pk, false),
                    AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                    AccountMeta::new_readonly(token_2022_program, false),
                ],
                data: vec![],
            };
            base_instructions.push(create_output_ata_ix);
        }
        
        // Get pool info for swap
        let pool_params = serde_json::json!([pool_address, {"encoding": "base64"}]);
        let pool_result: serde_json::Value = self.rpc.send_request("getAccountInfo", pool_params).await?;
        let pool_data_str = pool_result.get("value").and_then(|v| v.get("data")).and_then(|d| d.as_array())
            .and_then(|arr| arr.get(0)).and_then(|s| s.as_str())
            .ok_or_else(|| RpcError::Other("Failed to get pool data".to_string()))?;
        let pool_data = base64::decode(pool_data_str)
            .map_err(|e| RpcError::Other(format!("Failed to decode pool data: {}", e)))?;
        let pool_info = Self::parse_pool_state(pool_address, &pool_data)?;
        
        // Calculate minimum output
        let pool_price = self.get_pool_price(&pool_info).await?;
        let amount_in_tokens = amount_xnt_lamports as f64 / 10_f64.powi(9);
        let estimated_output = amount_in_tokens / pool_price.price;
        let minimum_output = (estimated_output * (1.0 - slippage_pct / 100.0) * 10_f64.powi(output_decimals as i32)) as u64;
        
                
        // Get pool addresses
        let amm_config = Pubkey::from_str(&Self::read_pubkey(&pool_data, 8)?)
            .map_err(|e| RpcError::Other(format!("Invalid AMM config: {}", e)))?;
        let observation_key = Pubkey::from_str(&Self::read_pubkey(&pool_data, 296)?)
            .map_err(|e| RpcError::Other(format!("Invalid observation key: {}", e)))?;
        let (authority, _) = Pubkey::find_program_address(&[b"vault_and_lp_mint_auth_seed"], &self.program_id);
        
        let input_vault = Pubkey::from_str(&pool_info.token_0_vault)
            .map_err(|e| RpcError::Other(format!("Invalid input vault: {}", e)))?;
        let output_vault = Pubkey::from_str(&pool_info.token_1_vault)
            .map_err(|e| RpcError::Other(format!("Invalid output vault: {}", e)))?;
        
        // Build swap instruction
        let discriminator: [u8; 8] = [143, 190, 90, 218, 196, 30, 51, 222];
        let mut swap_data = Vec::new();
        swap_data.extend_from_slice(&discriminator);
        swap_data.extend_from_slice(&amount_xnt_lamports.to_le_bytes());
        swap_data.extend_from_slice(&minimum_output.to_le_bytes());
        
        let swap_ix = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new_readonly(*user_pubkey, true),
                AccountMeta::new_readonly(authority, false),
                AccountMeta::new_readonly(amm_config, false),
                AccountMeta::new(pool_pubkey, false),
                AccountMeta::new(wxnt_ata, false),
                AccountMeta::new(output_ata, false),
                AccountMeta::new(input_vault, false),
                AccountMeta::new(output_vault, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(token_2022_program, false),
                AccountMeta::new_readonly(wxnt_mint, false),
                AccountMeta::new_readonly(output_mint_pk, false),
                AccountMeta::new(observation_key, false),
            ],
            data: swap_data,
        };
        base_instructions.push(swap_ix);
        
                
        // Get blockhash
        let blockhash = self.rpc.get_latest_blockhash().await?;
        
        // Simulate with dummy compute budget instructions for accurate CU estimation
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_400_000u32));
        
        // If user has set a price, include it in simulation to match final transaction
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(price));
            }
        }
        
        let sim_message = Message::new(&sim_instructions, Some(user_pubkey));
        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = blockhash;
        
        // Serialize and simulate
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);
        
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });
        
                let sim_result = self.rpc.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
                        units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction with compute budget based on simulation
        let mut final_instructions = base_instructions;
        
        // Add compute budget instructions using unified method
        let compute_budget_ixs = RpcConnection::build_compute_budget_instructions(
            simulated_cu,
            COMPUTE_UNIT_BUFFER
        );
        final_instructions.extend(compute_budget_ixs);
        
        let message = Message::new(&final_instructions, Some(user_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        
                
        Ok(transaction)
    }
}

impl Default for XDexConnection {
    fn default() -> Self {
        Self::new()
    }
}
