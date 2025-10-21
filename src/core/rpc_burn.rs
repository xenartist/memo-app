use crate::core::rpc_base::{RpcConnection, RpcError};
use solana_sdk::{
    pubkey::Pubkey,
    instruction::{Instruction, AccountMeta},
    message::Message,
    transaction::Transaction,
    compute_budget::ComputeBudgetInstruction,
    system_instruction,
    signature::{Keypair, Signer},
    hash::Hash,
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
    /// Memo-Burn contract program ID
    pub const MEMO_BURN_PROGRAM_ID: &'static str = "FEjJ9KKJETocmaStfsFteFrktPchDLAVNTMeTvndoxaP";
    
    /// Authorized MEMO token mint address
    pub const MEMO_TOKEN_MINT: &'static str = "HLCoc7wNDavNMfWWw2Bwd7U7A24cesuhBSNkxZgvZm1";
    
    /// Token 2022 Program ID
    pub const TOKEN_2022_PROGRAM_ID: &'static str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    
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
    pub const COMPUTE_UNIT_BUFFER: f64 = 1.2; // 20% buffer for burn operations
    pub const MIN_COMPUTE_UNITS: u64 = 300_000;
    
    /// PDA Seeds
    pub const USER_GLOBAL_BURN_STATS_SEED: &'static [u8] = b"user_global_burn_stats";
    
    /// Helper functions
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MEMO_BURN_PROGRAM_ID)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-burn program ID: {}", e)))
    }
    
    pub fn get_token_mint() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MEMO_TOKEN_MINT)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid token mint: {}", e)))
    }
    
    pub fn get_token_2022_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::TOKEN_2022_PROGRAM_ID)
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
        let base_instructions = vec![memo_instruction.clone(), burn_instruction.clone()];
        let message = Message::new(&base_instructions, Some(user_pubkey));
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
        
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            let with_buffer = (units_consumed as f64 * BurnConfig::COMPUTE_UNIT_BUFFER) as u64;
            std::cmp::max(with_buffer, BurnConfig::MIN_COMPUTE_UNITS)
        } else {
            400_000u64
        };
        
        let mut final_instructions = vec![];
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));
        final_instructions.push(memo_instruction);
        final_instructions.push(burn_instruction);
        
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
        
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            let with_buffer = (units_consumed as f64 * BurnConfig::COMPUTE_UNIT_BUFFER) as u64;
            std::cmp::max(with_buffer, BurnConfig::MIN_COMPUTE_UNITS)
        } else {
            300_000u64
        };
        
        let mut final_instructions = vec![];
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));
        final_instructions.push(instruction);
        
        let final_message = Message::new(&final_instructions, Some(user_pubkey));
        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = blockhash;
        
        Ok(final_transaction)
    }

    /// Initialize user global burn statistics (legacy method)
    /// 
    /// # Parameters
    /// * `keypair_bytes` - User's keypair bytes for signing
    /// 
    /// # Returns
    /// Transaction signature on success
    /// 
    /// # Note
    /// Deprecated. Use build_initialize_burn_stats_transaction + sign in Session + send_signed_transaction
    pub async fn initialize_user_global_burn_stats(
        &self,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Invalid keypair: {}", e)))?;
        let user_pubkey = keypair.pubkey();
        
        log::info!("Initializing user global burn stats for: {}", user_pubkey);
        
        // Get program and account addresses
        let program_id = BurnConfig::get_program_id()?;
        let (stats_pda, _) = BurnConfig::get_user_global_burn_stats_pda(&user_pubkey)?;
        let system_program = solana_sdk::system_program::id();
        
        // Generate instruction discriminator using SHA256
        let discriminator = BurnConfig::get_instruction_discriminator("initialize_user_global_burn_stats");
        let instruction_data = discriminator.to_vec();
        
        log::info!("Using discriminator: {:?}", discriminator);
        
        // Create instruction
        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(user_pubkey, true),
                AccountMeta::new(stats_pda, false),
                AccountMeta::new_readonly(system_program, false),
            ],
            data: instruction_data,
        };
        
        // Build transaction for simulation - get latest blockhash
        let blockhash = self.get_latest_blockhash().await?;
        let message = Message::new(&[instruction.clone()], Some(&user_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        transaction.sign(&[&keypair], transaction.message.recent_blockhash);
        
        // Serialize simulation transaction
        let sim_serialized_tx = base64::encode(bincode::serialize(&transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);
        
        // Simulate transaction
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });
        
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Initialize burn stats simulation consumed {} compute units", units_consumed);
            let with_buffer = (units_consumed as f64 * BurnConfig::COMPUTE_UNIT_BUFFER) as u64;
            std::cmp::max(with_buffer, BurnConfig::MIN_COMPUTE_UNITS)
        } else {
            log::info!("Failed to get compute units from simulation, using default");
            BurnConfig::MIN_COMPUTE_UNITS
        };
        
        log::info!("Using {} compute units for initialize burn stats", computed_units);
        
        // Build final transaction with compute budget
        let mut final_instructions = vec![];
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));
        final_instructions.push(instruction);
        
        let final_message = Message::new(&final_instructions, Some(&user_pubkey));
        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = blockhash;
        final_transaction.sign(&[&keypair], blockhash);
        
        // Serialize and send final transaction
        let final_serialized_tx = base64::encode(bincode::serialize(&final_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize final transaction: {}", e)))?);
        
        let send_params = serde_json::json!([
            final_serialized_tx,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "processed",
                "maxRetries": 3
            }
        ]);
        
        // Send transaction
        let signature = self.send_request("sendTransaction", send_params).await?;
        log::info!("Initialize user global burn stats transaction sent: {}", signature);
        
        Ok(signature)
    }

    /// Burn tokens using the memo-burn contract
    /// 
    /// # Parameters
    /// * `amount` - Amount to burn (in token units, not lamports)
    /// * `message` - Burn message (will be included in payload)
    /// * `keypair_bytes` - User's keypair bytes for signing
    /// 
    /// # Returns
    /// Transaction signature on success
    pub async fn burn_tokens(
        &self,
        amount: u64,
        message: &str,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        // Validate burn amount
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
        
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Invalid keypair: {}", e)))?;
        let user_pubkey = keypair.pubkey();
        
        log::info!("Burning {} tokens ({} units) for user: {}", amount, amount_units, user_pubkey);
        
        // Get program and account addresses
        let program_id = BurnConfig::get_program_id()?;
        let mint = BurnConfig::get_token_mint()?;
        let token_2022_program_id = BurnConfig::get_token_2022_program_id()?;
        let (stats_pda, _) = BurnConfig::get_user_global_burn_stats_pda(&user_pubkey)?;
        
        // Calculate token account (ATA) using Token 2022
        let token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &user_pubkey,
            &mint,
            &token_2022_program_id,
        );
        
        // Create burn memo with message as payload
        let burn_memo = BurnMemo {
            version: BURN_MEMO_VERSION,
            burn_amount: amount_units,
            payload: message.as_bytes().to_vec(),
        };
        
        // Serialize and encode to Base64
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        // Validate memo length
        BurnConfig::validate_memo_length(&memo_data_base64)?;
        
        log::info!("Burn memo data: {} bytes Borsh → {} bytes Base64", 
                  memo_data_bytes.len(), memo_data_base64.len());
        
        // Create memo instruction
        let memo_instruction = spl_memo::build_memo(
            memo_data_base64.as_bytes(),
            &[&user_pubkey],
        );
        
        // Generate instruction discriminator using SHA256 and add burn amount parameter
        let discriminator = BurnConfig::get_instruction_discriminator("process_burn");
        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&amount_units.to_le_bytes());
        
        log::info!("Using process_burn discriminator: {:?}", discriminator);
        
        // Create burn instruction
        let burn_instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(user_pubkey, true),
                AccountMeta::new(mint, false),
                AccountMeta::new(token_account, false),
                AccountMeta::new(stats_pda, false),
                AccountMeta::new_readonly(token_2022_program_id, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
            ],
            data: instruction_data,
        };
        
        // Build transaction for simulation - get latest blockhash
        let blockhash = self.get_latest_blockhash().await?;
        let base_instructions = vec![memo_instruction.clone(), burn_instruction.clone()];
        let message = Message::new(&base_instructions, Some(&user_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        transaction.sign(&[&keypair], transaction.message.recent_blockhash);
        
        // Serialize simulation transaction
        let sim_serialized_tx = base64::encode(bincode::serialize(&transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);
        
        // Simulate transaction
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });
        
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Burn simulation consumed {} compute units", units_consumed);
            let with_buffer = (units_consumed as f64 * BurnConfig::COMPUTE_UNIT_BUFFER) as u64;
            std::cmp::max(with_buffer, BurnConfig::MIN_COMPUTE_UNITS)
        } else {
            log::info!("Failed to get compute units from simulation, using default");
            400_000u64
        };
        
        log::info!("Using {} compute units for burn operation", computed_units);
        
        // Build final transaction with compute budget
        let mut final_instructions = vec![];
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));
        final_instructions.push(memo_instruction);
        final_instructions.push(burn_instruction);
        
        let final_message = Message::new(&final_instructions, Some(&user_pubkey));
        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = blockhash;
        final_transaction.sign(&[&keypair], blockhash);
        
        // Serialize and send final transaction
        let final_serialized_tx = base64::encode(bincode::serialize(&final_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize final transaction: {}", e)))?);
        
        let send_params = serde_json::json!([
            final_serialized_tx,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "processed",
                "maxRetries": 3
            }
        ]);
        
        // Send transaction
        let signature = self.send_request("sendTransaction", send_params).await?;
        log::info!("Burn transaction sent: {}", signature);
        
        Ok(signature)
    }
}
