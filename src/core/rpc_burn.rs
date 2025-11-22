use crate::core::rpc_base::{RpcConnection, RpcError};
use crate::core::network_config::get_program_ids;
use solana_sdk::{
    pubkey::Pubkey,
    instruction::{Instruction, AccountMeta},
    message::Message,
    transaction::Transaction,
    compute_budget::ComputeBudgetInstruction,
};
use spl_associated_token_account;
use spl_memo;
use borsh::{BorshSerialize, BorshDeserialize};
use serde::{Serialize, Deserialize};
use base64;
use std::str::FromStr;
use log;
use bincode;
use sha2::{Sha256, Digest};
use bs58;

/// Borsh serialization version constants
pub const BURN_MEMO_VERSION: u8 = 1;

/// Borsh length constants
const BORSH_U8_SIZE: usize = 1;         // version (u8)
const BORSH_U64_SIZE: usize = 8;        // burn_amount (u64)
const BORSH_VEC_LENGTH_SIZE: usize = 4; // payload.len() (u32)
const BORSH_FIXED_OVERHEAD: usize = BORSH_U8_SIZE + BORSH_U64_SIZE + BORSH_VEC_LENGTH_SIZE;

/// Memo-Burn contract configuration and constants
pub struct BurnConfig;

impl BurnConfig {
    // Note: Program IDs and token mint are now retrieved dynamically from network configuration
    
    /// Minimum burn amount (1 token = 1,000,000 units)
    pub const MIN_BURN_AMOUNT: u64 = 1_000_000;
    
    /// Maximum burn per transaction (1 trillion tokens)
    pub const MAX_BURN_PER_TX: u64 = 1_000_000_000_000 * 1_000_000;
    
    /// Memo validation limits (from contract: 69-800 bytes)
    pub const MIN_MEMO_LENGTH: usize = 69;
    pub const MAX_MEMO_LENGTH: usize = 800;
    
    /// Maximum payload length = memo maximum length - borsh fixed overhead
    pub const MAX_PAYLOAD_LENGTH: usize = Self::MAX_MEMO_LENGTH - BORSH_FIXED_OVERHEAD; // 800 - 13 = 787
    
    /// Compute budget configuration
    pub const COMPUTE_UNIT_BUFFER: f64 = 1.0; // 0% buffer - exact simulation
    pub const MIN_COMPUTE_UNITS: u64 = 300_000;
    
    /// PDA Seeds
    pub const USER_GLOBAL_BURN_STATS_SEED: &'static [u8] = b"user_global_burn_stats";
    
    /// Helper functions
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.burn_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-burn program ID: {}", e)))
    }
    
    pub fn get_token_mint() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.token_mint)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid token mint: {}", e)))
    }
    
    pub fn get_token_2022_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.token_2022_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid Token 2022 program ID: {}", e)))
    }
    
    /// Calculate user global burn stats PDA
    pub fn get_user_global_burn_stats_pda(user_pubkey: &Pubkey) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::USER_GLOBAL_BURN_STATS_SEED, user_pubkey.as_ref()],
            &program_id
        ))
    }
    
    /// Validate memo length
    pub fn validate_memo_length(memo: &str) -> Result<(), RpcError> {
        let len = memo.len();
        if len < Self::MIN_MEMO_LENGTH {
            return Err(RpcError::Other(format!("Memo too short: {} bytes (min: {})", len, Self::MIN_MEMO_LENGTH)));
        }
        if len > Self::MAX_MEMO_LENGTH {
            return Err(RpcError::Other(format!("Memo too long: {} bytes (max: {})", len, Self::MAX_MEMO_LENGTH)));
        }
        Ok(())
    }
    
    /// Generate Anchor instruction discriminator using SHA256
    pub fn get_instruction_discriminator(instruction_name: &str) -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(format!("global:{}", instruction_name).as_bytes());
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
}

/// BurnMemo structure (compatible with memo-burn contract)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BurnMemo {
    /// Version of the BurnMemo structure (for future compatibility)
    pub version: u8,
    
    /// Burn amount (must match actual burn amount)
    pub burn_amount: u64,
    
    /// Application payload (variable length, max 787 bytes)
    pub payload: Vec<u8>,
}

/// User global burn statistics (simplified client version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGlobalBurnStats {
    pub total_burned: u64,      // Total amount burned by this user (in units)
    pub burn_count: u64,        // Number of burn transactions
    pub last_burn_time: i64,    // Timestamp of last burn
}

/// Burn RPC client implementation
impl RpcConnection {
    /// Check if user has initialized global burn stats
    /// 
    /// # Parameters
    /// * `user_pubkey` - User's public key string
    /// 
    /// # Returns
    /// Result containing Option<UserGlobalBurnStats> (None if not initialized)
    pub async fn get_user_global_burn_stats(
        &self,
        user_pubkey: &str,
    ) -> Result<Option<UserGlobalBurnStats>, RpcError> {
        let user_pubkey = Pubkey::from_str(user_pubkey)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid user pubkey: {}", e)))?;
        
        let (stats_pda, _) = BurnConfig::get_user_global_burn_stats_pda(&user_pubkey)?;
        
        log::info!("Checking user global burn stats for: {}", user_pubkey);
        
        // Get account info
        let account_info = self.get_account_info(&stats_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            log::info!("User global burn stats not found for: {}", user_pubkey);
            return Ok(None);
        }
        
        // Parse account data
        if let Some(data_str) = account_info["value"]["data"][0].as_str() {
            let data = base64::decode(data_str)
                .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
            
            // Skip discriminator (8 bytes) and parse the rest
            if data.len() < 8 + 32 + 8 + 8 + 8 + 1 {
                return Err(RpcError::Other("Invalid account data length".to_string()));
            }
            
            let mut offset = 8; // Skip discriminator
            
            // Parse user pubkey (32 bytes)
            let user_bytes = &data[offset..offset + 32];
            let user = Pubkey::new_from_array(user_bytes.try_into().unwrap());
            offset += 32;
            
            // Parse total_burned (8 bytes)
            let total_burned = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            
            // Parse burn_count (8 bytes)
            let burn_count = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            
            // Parse last_burn_time (8 bytes)
            let last_burn_time = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
            offset += 8;
            
            // Parse bump (1 byte)
            let bump = data[offset];
            
            let stats = UserGlobalBurnStats {
                total_burned,
                burn_count,
                last_burn_time,
            };
            
            log::info!("Found user global burn stats: total_burned={}, burn_count={}", 
                      stats.total_burned, stats.burn_count);
            
            Ok(Some(stats))
        } else {
            Err(RpcError::Other("Failed to get account data".to_string()))
        }
    }
    
    /// Build an unsigned transaction to burn tokens
    pub async fn build_burn_transaction(
        &self,
        user_pubkey: &Pubkey,
        amount: u64,
        message: &str,
    ) -> Result<Transaction, RpcError> {
        let amount_units = amount * 1_000_000;
        if amount_units < BurnConfig::MIN_BURN_AMOUNT {
            return Err(RpcError::Other(format!(
                "Burn amount too small. Must be at least {} tokens",
                BurnConfig::MIN_BURN_AMOUNT / 1_000_000
            )));
        }
        
        if amount_units > BurnConfig::MAX_BURN_PER_TX {
            return Err(RpcError::Other(format!(
                "Burn amount too large. Maximum allowed: {} tokens",
                BurnConfig::MAX_BURN_PER_TX / 1_000_000
            )));
        }
        
        log::info!("Building burn transaction: {} tokens for user: {}", amount, user_pubkey);
        
        let program_id = BurnConfig::get_program_id()?;
        let mint = BurnConfig::get_token_mint()?;
        let token_2022_program_id = BurnConfig::get_token_2022_program_id()?;
        let (stats_pda, _) = BurnConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey, &mint, &token_2022_program_id,
        );
        
        let burn_memo = BurnMemo {
            version: BURN_MEMO_VERSION,
            burn_amount: amount_units,
            payload: message.as_bytes().to_vec(),
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        BurnConfig::validate_memo_length(&memo_data_base64)?;
        
        let memo_instruction = spl_memo::build_memo(memo_data_base64.as_bytes(), &[user_pubkey]);
        
        let discriminator = BurnConfig::get_instruction_discriminator("process_burn");
        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&amount_units.to_le_bytes());
        
        let burn_instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(mint, false),
                AccountMeta::new(token_account, false),
                AccountMeta::new(stats_pda, false),
                AccountMeta::new_readonly(token_2022_program_id, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
            ],
            data: instruction_data,
        };
        
        let blockhash = self.get_latest_blockhash().await?;
        
        // Simulate with dummy compute budget instructions for accurate CU estimation
        // Note: Keep same instruction order as final transaction (memo at index 0)
        let mut sim_instructions = vec![memo_instruction.clone(), burn_instruction.clone()];
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000u32));
        
        // If user has set a price, include it in simulation to match final transaction
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
            }
        }
        let message = Message::new(&sim_instructions, Some(user_pubkey));
        let mut sim_transaction = Transaction::new_unsigned(message);
        sim_transaction.message.recent_blockhash = blockhash;
        
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);
        
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });
        
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            std::cmp::max(units_consumed, BurnConfig::MIN_COMPUTE_UNITS)
        } else {
            400_000u64
        };
        
        // Build final transaction: memo at index 0, then other instructions, compute budget at end
        let mut final_instructions = vec![];
        final_instructions.push(memo_instruction);
        final_instructions.push(burn_instruction);
        
        // Add compute budget instructions using unified method
        let compute_budget_ixs = RpcConnection::build_compute_budget_instructions(
            simulated_cu,
            BurnConfig::COMPUTE_UNIT_BUFFER
        );
        final_instructions.extend(compute_budget_ixs);
        
        let final_message = Message::new(&final_instructions, Some(user_pubkey));
        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = blockhash;
        
        Ok(final_transaction)
    }

    /// Build an unsigned transaction to initialize burn stats
    pub async fn build_initialize_burn_stats_transaction(
        &self,
        user_pubkey: &Pubkey,
    ) -> Result<Transaction, RpcError> {
        log::info!("Building initialize burn stats transaction for: {}", user_pubkey);
        
        let program_id = BurnConfig::get_program_id()?;
        let (stats_pda, _) = BurnConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let system_program = solana_sdk::system_program::id();
        
        let discriminator = BurnConfig::get_instruction_discriminator("initialize_user_global_burn_stats");
        let instruction_data = discriminator.to_vec();
        
        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(stats_pda, false),
                AccountMeta::new_readonly(system_program, false),
            ],
            data: instruction_data,
        };
        
        let blockhash = self.get_latest_blockhash().await?;
        let message = Message::new(&[instruction.clone()], Some(user_pubkey));
        let mut sim_transaction = Transaction::new_unsigned(message);
        sim_transaction.message.recent_blockhash = blockhash;
        
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);
        
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });
        
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            std::cmp::max(units_consumed, BurnConfig::MIN_COMPUTE_UNITS)
        } else {
            300_000u64
        };
        
        let mut final_instructions = vec![];
        
        // Add compute budget instructions using unified method
        let compute_budget_ixs = RpcConnection::build_compute_budget_instructions(
            simulated_cu,
            BurnConfig::COMPUTE_UNIT_BUFFER
        );
        final_instructions.extend(compute_budget_ixs);
        final_instructions.push(instruction);
        
        let final_message = Message::new(&final_instructions, Some(user_pubkey));
        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = blockhash;
        
        Ok(final_transaction)
    }

    /// Get top token burners using getProgramAccounts
    /// Returns users sorted by total burned amount (descending)
    /// 
    /// # Parameters
    /// * `limit` - Maximum number of top burners to return (e.g., 100)
    /// 
    /// # Returns
    /// Vector of (user_address, total_burned_tokens, burn_count) tuples, sorted by total_burned descending
    pub async fn get_top_burners(&self, limit: usize) -> Result<Vec<(String, f64, u64)>, RpcError> {
        let program_id = BurnConfig::get_program_id()?.to_string();
        
        log::info!("Fetching top burners from memo-burn program: {}", program_id);
        
        // UserGlobalBurnStats account size: 8 (discriminator) + 32 (user) + 8 (total_burned) + 8 (burn_count) + 8 (last_burn_time) + 1 (bump) = 65 bytes
        let params = serde_json::json!([
            program_id,
            {
                "encoding": "base64",
                "filters": [
                    {
                        "dataSize": 65  // UserGlobalBurnStats account size
                    }
                ]
            }
        ]);
        
        let result: serde_json::Value = self.send_request("getProgramAccounts", params).await?;
        
        // Parse the response
        let mut burners: Vec<(String, f64, u64)> = Vec::new();
        
        if let Some(accounts) = result.as_array() {
            for account in accounts {
                if let Some(data_array) = account.get("account")
                    .and_then(|a| a.get("data"))
                    .and_then(|d| d.as_array())
                {
                    if let Some(data_str) = data_array.get(0).and_then(|d| d.as_str()) {
                        if let Ok(data) = base64::decode(data_str) {
                            // Parse account data
                            // Layout: [discriminator:8][user:32][total_burned:8][burn_count:8][last_burn_time:8][bump:1]
                            if data.len() >= 65 {
                                let mut offset = 8; // Skip discriminator
                                
                                // Parse user pubkey (32 bytes)
                                let user_bytes = &data[offset..offset + 32];
                                let user = Pubkey::new_from_array(user_bytes.try_into().unwrap());
                                offset += 32;
                                
                                // Parse total_burned (8 bytes)
                                let total_burned = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                                offset += 8;
                                
                                // Parse burn_count (8 bytes)
                                let burn_count = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
                                
                                // Convert to tokens (divide by 1,000,000)
                                let total_burned_tokens = total_burned as f64 / 1_000_000.0;
                                
                                if total_burned > 0 {
                                    burners.push((user.to_string(), total_burned_tokens, burn_count));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by total burned (descending)
        burners.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit to top N
        burners.truncate(limit);
        
        log::info!("Found {} burners (limited to top {})", burners.len(), limit);
        
        Ok(burners)
    }
    
    /// Get the latest burn transaction signatures for the burn program
    /// 
    /// # Parameters
    /// * `limit` - Maximum number of signatures to return (default: 1)
    /// 
    /// # Returns
    /// Result containing a vector of signature strings
    pub async fn get_latest_burn_signatures(&self, limit: usize) -> Result<Vec<String>, RpcError> {
        let program_id = BurnConfig::get_program_id()?;
        
        log::info!("Fetching latest {} burn signatures for program: {}", limit, program_id);
        
        let options = serde_json::json!({
            "limit": limit,
        });
        
        let result = self.get_signatures_for_address(&program_id.to_string(), Some(options)).await?;
        let result: serde_json::Value = serde_json::from_str(&result)
            .map_err(|e| RpcError::Other(format!("Failed to parse signatures: {}", e)))?;
        
        let mut signatures = Vec::new();
        if let Some(sigs) = result.as_array() {
            for sig in sigs {
                if let Some(signature) = sig["signature"].as_str() {
                    signatures.push(signature.to_string());
                }
            }
        }
        
        Ok(signatures)
    }
    
    /// Get and parse the latest burn transaction from mainnet
    /// Returns the latest burn of any type (profile, chat, project, etc.)
    /// Always queries mainnet burn contract regardless of selected network
    pub async fn get_latest_burn() -> Result<Option<LatestBurn>, RpcError> {
        // ALWAYS use mainnet burn program ID for login page display
        // Mainnet burn program ID: memo-burn contract on X1 mainnet
        let burn_program_id = "2sb3gz5Cmr2g1ia5si2rmCZqPACxgaZXEmiS5k6Htcvh";
        
        log::info!("Fetching latest profile burn from mainnet burn contract: {}", burn_program_id);
        
        // Create RPC connection to mainnet
        let mainnet_rpc = "https://rpc.mainnet.x1.xyz";
        let rpc = RpcConnection::with_endpoint(mainnet_rpc);
        
        // Get latest burn signatures with memo data included
        let options = serde_json::json!({
            "limit": 20,  // Check more transactions to find a profile burn
            "commitment": "confirmed",
        });
        
        let result = rpc.get_signatures_for_address(burn_program_id, Some(options)).await?;
        let signatures: serde_json::Value = serde_json::from_str(&result)
            .map_err(|e| RpcError::Other(format!("Failed to parse signatures: {}", e)))?;
        
        let sig_array = signatures.as_array()
            .ok_or_else(|| RpcError::Other("Invalid signatures response format".to_string()))?;
        
        log::info!("Found {} signatures to check", sig_array.len());
        
        // Process each signature - memo data is already included in the response!
        for sig_info in sig_array {
            let signature = sig_info["signature"]
                .as_str()
                .unwrap_or("")
                .to_string();
            
            if signature.is_empty() {
                continue;
            }
            
            // Extract memo field directly from signature info (no need for getTransaction!)
            if let Some(memo_str) = sig_info["memo"].as_str() {
                // The memo field format is "[length] base64_data"
                // Extract the base64 part after the length prefix
                let memo_data = if let Some(space_pos) = memo_str.find(' ') {
                    &memo_str[space_pos + 1..]
                } else {
                    memo_str
                };
                
                // Convert string to bytes for parsing
                let memo_bytes = memo_data.as_bytes();
                
                // Parse memo data as any type of burn
                if let Some(burn_info) = parse_burn_memo(memo_bytes) {
                    log::info!("Found {} burn by {}", burn_info.burn_type, burn_info.user_pubkey);
                    return Ok(Some(LatestBurn {
                        signature,
                        burn_type: burn_info.burn_type,
                        username: burn_info.username,
                        image: burn_info.image,
                        description: burn_info.description,
                        burn_amount: burn_info.burn_amount,
                        user_pubkey: burn_info.user_pubkey,
                    }));
                }
            }
        }
        
        log::info!("No profile burn found in recent transactions");
        Ok(None)
    }
}

/// Parse Base64+Borsh-formatted memo data to extract burn information (any type)
fn parse_burn_memo(memo_data: &[u8]) -> Option<LatestBurn> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to BurnMemo first
    let burn_memo = BurnMemo::try_from_slice(&borsh_bytes).ok()?;
    let burn_amount = burn_memo.burn_amount / 1_000_000; // Convert to tokens
    
    // Try to parse as ProfileCreationData
    if let Ok(profile_data) = crate::core::rpc_profile::ProfileCreationData::try_from_slice(&burn_memo.payload) {
        if profile_data.category == "profile" && 
           (profile_data.operation == "create_profile" || profile_data.operation == "update_profile") {
            return Some(LatestBurn {
                signature: String::new(),
                burn_type: "profile".to_string(),
                username: Some(profile_data.username),
                image: Some(profile_data.image),
                description: profile_data.about_me,
                burn_amount,
                user_pubkey: profile_data.user_pubkey,
            });
        }
    }
    
    // Try to parse as ChatGroupBurnData (burn_for_group operation)
    if let Ok(chat_burn_data) = crate::core::rpc_chat::ChatGroupBurnData::try_from_slice(&burn_memo.payload) {
        if chat_burn_data.category == "chat" && chat_burn_data.operation == "burn_for_group" {
            // Use the burn message as description, or show group ID if message is empty
            let description = if !chat_burn_data.message.is_empty() {
                Some(chat_burn_data.message)
            } else {
                Some(format!("Burned for chat group #{}", chat_burn_data.group_id))
            };
            
            return Some(LatestBurn {
                signature: String::new(),
                burn_type: "chat_burn".to_string(),
                username: None, // Will be fetched by UI layer if needed
                image: None,    // Will be fetched by UI layer if needed
                description,
                burn_amount,
                user_pubkey: chat_burn_data.burner,
            });
        }
    }
    
    // Try to parse as ChatGroupCreationData (create_group operation)
    if let Ok(group_creation) = crate::core::rpc_chat::ChatGroupCreationData::try_from_slice(&burn_memo.payload) {
        if group_creation.category == "chat" && group_creation.operation == "create_group" {
            // Show group name and description
            let description = if !group_creation.description.is_empty() {
                Some(format!("Created chat group: {} - {}", group_creation.name, group_creation.description))
            } else {
                Some(format!("Created chat group: {}", group_creation.name))
            };
            
            return Some(LatestBurn {
                signature: String::new(),
                burn_type: "chat_create".to_string(),
                username: None, // Creator info not in memo, would need to fetch from transaction
                image: Some(group_creation.image), // Use group image
                description,
                burn_amount,
                user_pubkey: String::new(), // Creator pubkey not in ChatGroupCreationData
            });
        }
    }
    
    // Could add parsing for other types here (project, etc.)
    // For now, return None if not a recognized type
    None
}

/// Latest burn transaction information (any type: profile, chat, project, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestBurn {
    pub signature: String,
    pub burn_type: String, // "profile", "chat", "project", etc.
    pub username: Option<String>,
    pub image: Option<String>,
    pub description: Option<String>,
    pub burn_amount: u64, // in tokens
    pub user_pubkey: String,
}
