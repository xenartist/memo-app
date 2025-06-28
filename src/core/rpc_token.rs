use super::rpc_base::{RpcConnection, RpcError};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::{
    signature::{Keypair, Signer},
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
    message::Message,
    compute_budget::ComputeBudgetInstruction,
};
use base64;
use bincode;
use sha2::Digest;
use spl_associated_token_account;
use spl_memo;
use serde_json::json;

// public constant definitions
pub struct ProgramConfig;

impl ProgramConfig {
    // main program ID
    pub const PROGRAM_ID: &'static str = "TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw";
    
    // token mint address
    pub const TOKEN_MINT: &'static str = "MEM69mjnKAMxgqwosg5apfYNk2rMuV26FR9THDfT3Q7";
    
    // PDA Seeds
    pub const USER_PROFILE_SEED: &'static [u8] = b"user_profile";
    pub const MINT_AUTHORITY_SEED: &'static [u8] = b"mint_authority";
    pub const LATEST_BURN_SHARD_SEED: &'static [u8] = b"latest_burn_shard";
    pub const GLOBAL_TOP_BURN_INDEX_SEED: &'static [u8] = b"global_top_burn_index";
    pub const TOP_BURN_SHARD_SEED: &'static [u8] = b"top_burn_shard";
    pub const BURN_HISTORY_SEED: &'static [u8] = b"burn_history";
    
    // instruction discriminators - unified to array format, from IDL
    pub const INITIALIZE_USER_PROFILE_DISCRIMINATOR: [u8; 8] = [192, 144, 204, 140, 113, 25, 59, 102];
    pub const CLOSE_USER_PROFILE_DISCRIMINATOR: [u8; 8] = [242, 80, 248, 79, 81, 251, 65, 113];
    pub const PROCESS_TRANSFER_DISCRIMINATOR: [u8; 8] = [212, 115, 192, 211, 191, 149, 132, 69];
    pub const PROCESS_BURN_DISCRIMINATOR: [u8; 8] = [220, 214, 24, 210, 116, 16, 167, 18];
    pub const PROCESS_BURN_WITH_HISTORY_DISCRIMINATOR: [u8; 8] = [97, 115, 133, 136, 113, 113, 180, 185];
    pub const INITIALIZE_USER_BURN_HISTORY_DISCRIMINATOR: [u8; 8] = [40, 163, 144, 239, 40, 5, 88, 119];
    pub const CLOSE_USER_BURN_HISTORY_DISCRIMINATOR: [u8; 8] = [208, 153, 10, 179, 27, 50, 158, 161];
    pub const INITIALIZE_LATEST_BURN_SHARD_DISCRIMINATOR: [u8; 8] = [150, 220, 2, 213, 30, 67, 33, 31];
    pub const CLOSE_LATEST_BURN_SHARD_DISCRIMINATOR: [u8; 8] = [93, 129, 3, 152, 194, 180, 0, 53];
    pub const INITIALIZE_TOP_BURN_SHARD_DISCRIMINATOR: [u8; 8] = [100, 156, 197, 248, 154, 101, 107, 185];
    pub const CLOSE_TOP_BURN_SHARD_DISCRIMINATOR: [u8; 8] = [252, 203, 86, 232, 209, 69, 97, 14];
    pub const INITIALIZE_GLOBAL_TOP_BURN_INDEX_DISCRIMINATOR: [u8; 8] = [89, 23, 213, 27, 103, 194, 63, 67];
    pub const CLOSE_GLOBAL_TOP_BURN_INDEX_DISCRIMINATOR: [u8; 8] = [169, 205, 89, 205, 64, 5, 147, 219];
    
    // validation limits
    pub const MIN_MEMO_LENGTH: usize = 69;
    pub const MAX_MEMO_LENGTH: usize = 700;
    
    // compute unit limits
    pub const MIN_COMPUTE_UNITS: u64 = 1000;
    pub const COMPUTE_UNIT_BUFFER: f64 = 1.1; // 10% buffer
    
    // fallback compute unit configuration
    pub const FALLBACK_COMPUTE_UNITS: [(usize, u64); 7] = [
        (100, 100_000),   // 69-100 bytes
        (200, 150_000),   // 101-200 bytes
        (300, 200_000),   // 201-300 bytes
        (400, 250_000),   // 301-400 bytes
        (500, 300_000),   // 401-500 bytes
        (600, 350_000),   // 501-600 bytes
        (700, 400_000),   // 601-700 bytes
    ];
    pub const DEFAULT_COMPUTE_UNITS: u64 = 400_000;
    
    // Token 2022 Program ID
    pub const TOKEN_2022_PROGRAM_ID: &'static str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    
    // burn related constants
    pub const MIN_BURN_AMOUNT: u64 = 1_000_000_000; // 1 token in lamports
    pub const TOP_BURN_THRESHOLD: u64 = 420_000_000_000; // 420 tokens in lamports
}

// helper functions
impl ProgramConfig {
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::PROGRAM_ID)
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))
    }
    
    pub fn get_token_mint() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::TOKEN_MINT)
            .map_err(|e| RpcError::Other(format!("Invalid token mint address: {}", e)))
    }
    
    pub fn get_user_profile_pda(user_pubkey: &Pubkey) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::USER_PROFILE_SEED, user_pubkey.as_ref()],
            &program_id
        ))
    }
    
    pub fn get_mint_authority_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::MINT_AUTHORITY_SEED],
            &program_id
        ))
    }
    
    pub fn get_latest_burn_shard_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::LATEST_BURN_SHARD_SEED],
            &program_id
        ))
    }
    
    pub fn calculate_fallback_compute_units(memo_length: usize) -> u64 {
        for (max_length, compute_units) in Self::FALLBACK_COMPUTE_UNITS.iter() {
            if memo_length <= *max_length {
                return *compute_units;
            }
        }
        Self::DEFAULT_COMPUTE_UNITS
    }
    
    pub fn validate_memo_length(memo: &str) -> Result<(), RpcError> {
        let memo_length = memo.len();
        if memo_length < Self::MIN_MEMO_LENGTH {
            return Err(RpcError::Other(format!(
                "Memo length must be at least {} bytes", 
                Self::MIN_MEMO_LENGTH
            )));
        }
        if memo_length > Self::MAX_MEMO_LENGTH {
            return Err(RpcError::Other(format!(
                "Memo length cannot exceed {} bytes", 
                Self::MAX_MEMO_LENGTH
            )));
        }
        Ok(())
    }
    
    pub fn get_token_2022_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::TOKEN_2022_PROGRAM_ID)
            .map_err(|e| RpcError::Other(format!("Invalid Token 2022 program ID: {}", e)))
    }
    
    // global top burn index PDA calculation function
    pub fn get_global_top_burn_index_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::GLOBAL_TOP_BURN_INDEX_SEED],
            &program_id
        ))
    }
    
    pub fn get_top_burn_shard_pda(index: u64) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::TOP_BURN_SHARD_SEED, &index.to_le_bytes()],
            &program_id
        ))
    }

    pub fn get_user_burn_history_pda(user_pubkey: &Pubkey, index: u64) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::BURN_HISTORY_SEED, user_pubkey.as_ref(), &index.to_le_bytes()],
            &program_id
        ))
    }
    
    // create burn memo helper function
    pub fn create_burn_memo(message: &str, signature: &str) -> Result<String, RpcError> {
        let memo_json = serde_json::json!({
            "signature": signature,
            "message": message
        });
        
        let memo_text = serde_json::to_string(&memo_json)
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo JSON: {}", e)))?;
        
        // ensure memo length is within requirements
        let memo_text = Self::ensure_min_memo_length(memo_text, Self::MIN_MEMO_LENGTH);
        Self::validate_memo_length(&memo_text)?;
        
        Ok(memo_text)
    }
    
    // ensure memo minimum length helper function
    fn ensure_min_memo_length(text: String, min_length: usize) -> String {
        if text.as_bytes().len() >= min_length {
            return text;
        }
        
        // parse existing JSON
        let mut json: serde_json::Value = serde_json::from_str(&text)
            .expect("Failed to parse JSON");
        
        // get existing message
        let message = json["message"].as_str().unwrap_or("");
        
        // calculate required padding length
        let current_length = text.as_bytes().len();
        let padding_needed = min_length - current_length;
        
        // fill with spaces
        let padding = " ".repeat(padding_needed);
        let new_message = format!("{}{}", message, padding);
        json["message"] = serde_json::Value::String(new_message);
        
        // convert back to string
        serde_json::to_string(&json).expect("Failed to serialize JSON")
    }
}

impl RpcConnection {
    pub async fn get_user_profile(&self, pubkey: &str) -> Result<String, RpcError> {
        let target_pubkey = Pubkey::from_str(pubkey)
            .map_err(|e| RpcError::Other(format!("Invalid public key: {}", e)))?;

        // Calculate user profile PDA using config
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;

        // get account info, using base64 encoding
        self.get_account_info(&user_profile_pda.to_string(), Some("base64")).await
    }

    pub async fn initialize_user_profile(
        &self, 
        keypair_bytes: &[u8]
    ) -> Result<String, RpcError> {
        let program_id = ProgramConfig::get_program_id()?;
        
        // Create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // Calculate user profile PDA using config
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;

        // Get latest blockhash with specific commitment
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

        // Simplified instruction data - only discriminator, no username/image
        let instruction_data = ProgramConfig::INITIALIZE_USER_PROFILE_DISCRIMINATOR.to_vec();

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
                "preflightCommitment": "confirmed",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        self.send_request("sendTransaction", params).await
    }

    pub async fn close_user_profile(
        &self,
        keypair_bytes: &[u8]
    ) -> Result<String, RpcError> {
        let program_id = ProgramConfig::get_program_id()?;
        
        // Create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // Calculate user profile PDA using config
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;

        // Get latest blockhash with specific commitment
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

        // Create instruction data with discriminator from config
        let instruction_data = ProgramConfig::CLOSE_USER_PROFILE_DISCRIMINATOR.to_vec();

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
                "preflightCommitment": "confirmed",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        self.send_request("sendTransaction", params).await
    }

    pub async fn get_latest_burn_shard(&self) -> Result<String, RpcError> {
        // Calculate PDA using config
        let (latest_burn_shard_pda, _) = ProgramConfig::get_latest_burn_shard_pda()?;

        // get account info
        self.get_account_info(&latest_burn_shard_pda.to_string(), Some("base64")).await
    }

    pub async fn mint(
        &self,
        memo: &str,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        // Validate memo length using config
        ProgramConfig::validate_memo_length(memo)?;
        
        // Get addresses from config
        let program_id = ProgramConfig::get_program_id()?;
        let mint = ProgramConfig::get_token_mint()?;
        let token_2022_program_id = ProgramConfig::get_token_2022_program_id()?; // use Token 2022

        // Create keypair from bytes and get pubkey
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // Calculate PDAs using config
        let (mint_authority_pda, _) = ProgramConfig::get_mint_authority_pda()?;
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;

        // Calculate token account (ATA) using Token 2022 program
        let token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &target_pubkey,
            &mint,
            &token_2022_program_id, // use Token 2022 program ID
        );

        // Check if token account exists
        let token_account_info = self.get_account_info(&token_account.to_string(), Some("base64")).await?;
        let token_account_info: serde_json::Value = serde_json::from_str(&token_account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse token account info: {}", e)))?;

        // Build instructions without compute budget first (for simulation)
        let mut base_instructions = vec![];

        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(
            memo.as_bytes(),
            &[&target_pubkey],
        ));

        // If token account doesn't exist, add create ATA instruction for Token 2022
        if token_account_info["value"].is_null() {
            log::info!("Token 2022 account does not exist, creating it...");
            base_instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                    &target_pubkey,         // Funding account (fee payer)
                    &target_pubkey,         // Wallet address
                    &mint,                  // Mint address
                    &token_2022_program_id  // Token 2022 program ID
                )
            );
        }

        // use discriminator array directly, no SHA256
        let instruction_data = ProgramConfig::PROCESS_TRANSFER_DISCRIMINATOR.to_vec();

        // Create mint instruction accounts with Token 2022 program
        let mut accounts = vec![
            AccountMeta::new(target_pubkey, true),               // user
            AccountMeta::new(mint, false),                       // mint
            AccountMeta::new(mint_authority_pda, false),         // mint_authority
            AccountMeta::new(token_account, false),              // token_account
            AccountMeta::new_readonly(token_2022_program_id, false), // Token 2022 program
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions sysvar
        ];

        // Check if user profile exists
        let profile_info = self.get_account_info(&user_profile_pda.to_string(), Some("base64")).await?;
        let profile_info: serde_json::Value = serde_json::from_str(&profile_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse profile info: {}", e)))?;

        if !profile_info["value"].is_null() {
            accounts.push(AccountMeta::new(user_profile_pda, false));
        }

        // Add mint instruction
        base_instructions.push(Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            accounts,
        ));

        // Get latest blockhash
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

        // Create simulation transaction (without compute budget instruction)
        let sim_message = Message::new(
            &base_instructions,
            Some(&target_pubkey),
        );

        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        sim_transaction.sign(&[&keypair], sim_transaction.message.recent_blockhash);

        // Serialize simulation transaction
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);

        // Simulate transaction to get compute units consumption
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });

        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse simulation result to extract compute units consumed using config
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Token 2022 mint simulation consumed {} compute units", units_consumed);
            // Add buffer using config
            let with_buffer = (units_consumed as f64 * ProgramConfig::COMPUTE_UNIT_BUFFER) as u64;
            // Ensure minimum using config
            std::cmp::max(with_buffer, ProgramConfig::MIN_COMPUTE_UNITS)
        } else {
            log::info!("Failed to get compute units from simulation, using fallback");
            // Fallback calculation using config
            ProgramConfig::calculate_fallback_compute_units(memo.len())
        };

        log::info!("Using {} compute units for Token 2022 mint (with {}% buffer)", computed_units, (ProgramConfig::COMPUTE_UNIT_BUFFER - 1.0) * 100.0);

        // Now build the final transaction with the calculated compute units
        let mut final_instructions = vec![];

        // Add compute budget instruction first
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));

        // Add all the base instructions
        final_instructions.extend(base_instructions);

        // Create and sign final transaction
        let final_message = Message::new(
            &final_instructions,
            Some(&target_pubkey),
        );

        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        final_transaction.sign(&[&keypair], final_transaction.message.recent_blockhash);

        // Serialize and send final transaction
        let final_serialized_tx = base64::encode(bincode::serialize(&final_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize final transaction: {}", e)))?);

        let params = serde_json::json!([
            final_serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "confirmed",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        self.send_request("sendTransaction", params).await
    }

    // get current top burn shard index
    pub async fn get_current_top_burn_shard_index(&self) -> Result<Option<u64>, RpcError> {
        let (global_top_burn_index_pda, _) = ProgramConfig::get_global_top_burn_index_pda()?;
        
        match self.get_account_info(&global_top_burn_index_pda.to_string(), Some("base64")).await {
            Ok(account_info_str) => {
                let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                    .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
                
                if let Some(data) = account_info["value"]["data"].get(0).and_then(|v| v.as_str()) {
                    let decoded = base64::decode(data)
                        .map_err(|e| RpcError::Other(format!("Failed to decode base64 data: {}", e)))?;
                    
                    if decoded.len() >= 17 { // 8 bytes discriminator + 8 bytes total_count + 1 byte option tag
                        let data = &decoded[8..]; // skip discriminator
                        let option_tag = data[8];
                        
                        if option_tag == 1 && data.len() >= 17 {
                            let current_index = u64::from_le_bytes(data[9..17].try_into().unwrap());
                            return Ok(Some(current_index));
                        }
                    }
                }
                Ok(None)
            },
            Err(_) => Ok(None) // Account doesn't exist
        }
    }

    // main burn function
    pub async fn burn(
        &self,
        amount: u64,
        message: &str,
        signature: &str,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        // validate burn amount
        if amount < ProgramConfig::MIN_BURN_AMOUNT {
            return Err(RpcError::Other(format!(
                "Burn amount too small. Must be at least {} tokens",
                ProgramConfig::MIN_BURN_AMOUNT / 1_000_000_000
            )));
        }
        
        // create memo
        let memo = ProgramConfig::create_burn_memo(message, signature)?;
        
        // get address configuration
        let program_id = ProgramConfig::get_program_id()?;
        let mint = ProgramConfig::get_token_mint()?;
        let token_2022_program_id = ProgramConfig::get_token_2022_program_id()?;

        // create keypair and get pubkey
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // calculate PDAs
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;
        let (latest_burn_shard_pda, _) = ProgramConfig::get_latest_burn_shard_pda()?;
        let (global_top_burn_index_pda, _) = ProgramConfig::get_global_top_burn_index_pda()?;

        // calculate token account (ATA) using Token 2022
        let token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &target_pubkey,
            &mint,
            &token_2022_program_id,
        );

        // check if user profile exists
        let profile_info = self.get_account_info(&user_profile_pda.to_string(), Some("base64")).await?;
        let profile_info: serde_json::Value = serde_json::from_str(&profile_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse profile info: {}", e)))?;
        let user_profile_exists = !profile_info["value"].is_null();

        // get current top burn shard index
        let current_top_burn_shard_index = self.get_current_top_burn_shard_index().await?;

        // build base instructions (for simulation)
        let mut base_instructions = vec![];

        // add memo instruction
        base_instructions.push(spl_memo::build_memo(
            memo.as_bytes(),
            &[&target_pubkey],
        ));

        // use discriminator array directly, no SHA256
        let mut instruction_data = ProgramConfig::PROCESS_BURN_DISCRIMINATOR.to_vec();
        
        // add burn amount parameter
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        // create burn instruction accounts
        let mut accounts = vec![
            AccountMeta::new(target_pubkey, true),                        // user
            AccountMeta::new(mint, false),                               // mint
            AccountMeta::new(token_account, false),                      // token_account
            AccountMeta::new_readonly(token_2022_program_id, false),     // token_program
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions sysvar
            AccountMeta::new(latest_burn_shard_pda, false),              // latest_burn_shard
            AccountMeta::new(global_top_burn_index_pda, false),          // global_top_burn_index
        ];

        // if there is current top burn shard, add to accounts list
        if let Some(index) = current_top_burn_shard_index {
            let (top_burn_shard_pda, _) = ProgramConfig::get_top_burn_shard_pda(index)?;
            accounts.push(AccountMeta::new(top_burn_shard_pda, false)); // top_burn_shard
        }

        // if user profile exists, add to accounts list
        if user_profile_exists {
            accounts.push(AccountMeta::new(user_profile_pda, false)); // user_profile
        }

        // add burn instruction
        base_instructions.push(Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            accounts,
        ));

        // get latest blockhash
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

        // create simulation transaction (without compute budget instruction)
        let sim_message = Message::new(
            &base_instructions,
            Some(&target_pubkey),
        );

        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        sim_transaction.sign(&[&keypair], sim_transaction.message.recent_blockhash);

        // serialize simulation transaction
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);

        // simulate transaction to get compute units consumed
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });

        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // parse simulation result to extract compute units consumed
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Token 2022 burn simulation consumed {} compute units", units_consumed);
            // add buffer
            let with_buffer = (units_consumed as f64 * ProgramConfig::COMPUTE_UNIT_BUFFER) as u64;
            // ensure minimum
            std::cmp::max(with_buffer, ProgramConfig::MIN_COMPUTE_UNITS)
        } else {
            log::info!("Failed to get compute units from simulation, using fallback");
            // fallback calculation - burn operations usually require more compute units
            440_000u64 // default value based on reference code
        };

        log::info!("Using {} compute units for Token 2022 burn (with {}% buffer)", computed_units, (ProgramConfig::COMPUTE_UNIT_BUFFER - 1.0) * 100.0);

        // build final transaction, including computed compute units
        let mut final_instructions = vec![];

        // first add compute budget instruction
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));

        // add all base instructions
        final_instructions.extend(base_instructions);

        // create and sign final transaction
        let final_message = Message::new(
            &final_instructions,
            Some(&target_pubkey),
        );

        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        final_transaction.sign(&[&keypair], final_transaction.message.recent_blockhash);

        // serialize and send final transaction
        let final_serialized_tx = base64::encode(bincode::serialize(&final_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize final transaction: {}", e)))?);

        let params = serde_json::json!([
            final_serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "confirmed",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        self.send_request("sendTransaction", params).await
    }

    // get global top burn index info
    pub async fn get_global_top_burn_index(&self) -> Result<String, RpcError> {
        let (global_top_burn_index_pda, _) = ProgramConfig::get_global_top_burn_index_pda()?;
        self.get_account_info(&global_top_burn_index_pda.to_string(), Some("base64")).await
    }

    // get top burn shard by index
    pub async fn get_top_burn_shard(&self, index: u64) -> Result<String, RpcError> {
        let (top_burn_shard_pda, _) = ProgramConfig::get_top_burn_shard_pda(index)?;
        self.get_account_info(&top_burn_shard_pda.to_string(), Some("base64")).await
    }

    // burn with history
    pub async fn burn_with_history(
        &self,
        amount: u64,
        message: &str,
        signature: &str,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        // validate burn amount
        if amount < ProgramConfig::MIN_BURN_AMOUNT {
            return Err(RpcError::Other(format!(
                "Burn amount too small. Must be at least {} tokens",
                ProgramConfig::MIN_BURN_AMOUNT / 1_000_000_000
            )));
        }
        
        // create memo
        let memo = ProgramConfig::create_burn_memo(message, signature)?;
        
        // get address configuration
        let program_id = ProgramConfig::get_program_id()?;
        let mint = ProgramConfig::get_token_mint()?;
        let token_2022_program_id = ProgramConfig::get_token_2022_program_id()?;

        // create keypair and get pubkey
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // calculate PDAs
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;
        let (latest_burn_shard_pda, _) = ProgramConfig::get_latest_burn_shard_pda()?;
        let (global_top_burn_index_pda, _) = ProgramConfig::get_global_top_burn_index_pda()?;

        // check if user profile exists (required for burn with history)
        let profile_info = self.get_account_info(&user_profile_pda.to_string(), Some("base64")).await?;
        let profile_info: serde_json::Value = serde_json::from_str(&profile_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse profile info: {}", e)))?;
        
        if profile_info["value"].is_null() {
            return Err(RpcError::Other("User profile must exist for burn with history operation".to_string()));
        }

        // parse user profile to get burn_history_index
        let data = profile_info["value"]["data"][0].as_str()
            .ok_or_else(|| RpcError::Other("Failed to get profile data".to_string()))?;
        let decoded = base64::decode(data)
            .map_err(|e| RpcError::Other(format!("Failed to decode profile data: {}", e)))?;

        // extract burn_history_index from user profile data
        // UserProfile structure: discriminator(8) + pubkey(32) + total_minted(8) + total_burned(8) + 
        // mint_count(8) + burn_count(8) + created_at(8) + last_updated(8) + burn_history_index(Option<u64> = 1+8)
        let burn_history_index = if decoded.len() >= 89 { // minimum size to contain option tag
            let option_tag = decoded[88]; // position of Option<u64> tag
            if option_tag == 1 && decoded.len() >= 97 { // Some variant with u64 value
                u64::from_le_bytes(decoded[89..97].try_into().unwrap())
            } else {
                return Err(RpcError::Other("User profile has no burn history index. Please initialize burn history first.".to_string()));
            }
        } else {
            return Err(RpcError::Other("Invalid user profile data structure".to_string()));
        };

        // calculate burn history PDA for the next index
        let (user_burn_history_pda, _) = ProgramConfig::get_user_burn_history_pda(&target_pubkey, burn_history_index)?;

        // verify burn history account exists
        let burn_history_info = self.get_account_info(&user_burn_history_pda.to_string(), Some("base64")).await?;
        let burn_history_info: serde_json::Value = serde_json::from_str(&burn_history_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse burn history info: {}", e)))?;
        
        if burn_history_info["value"].is_null() {
            return Err(RpcError::Other("Burn history account does not exist. Please initialize burn history first.".to_string()));
        }

        // calculate token account (ATA) using Token 2022
        let token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &target_pubkey,
            &mint,
            &token_2022_program_id,
        );

        // get current top burn shard index
        let current_top_burn_shard_index = self.get_current_top_burn_shard_index().await?;

        // build base instructions (for simulation)
        let mut base_instructions = vec![];

        // add memo instruction
        base_instructions.push(spl_memo::build_memo(
            memo.as_bytes(),
            &[&target_pubkey],
        ));

        // create instruction data using discriminator
        let mut instruction_data = ProgramConfig::PROCESS_BURN_WITH_HISTORY_DISCRIMINATOR.to_vec();
        
        // add burn amount parameter
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        // create burn with history instruction accounts
        let mut accounts = vec![
            AccountMeta::new(target_pubkey, true),                        // user
            AccountMeta::new(mint, false),                               // mint
            AccountMeta::new(token_account, false),                      // token_account
            AccountMeta::new_readonly(token_2022_program_id, false),     // token_program
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions sysvar
            AccountMeta::new(latest_burn_shard_pda, false),              // latest_burn_shard (optional)
            AccountMeta::new(global_top_burn_index_pda, false),          // global_top_burn_index (optional)
        ];

        // if there is current top burn shard, add to accounts list
        if let Some(index) = current_top_burn_shard_index {
            let (top_burn_shard_pda, _) = ProgramConfig::get_top_burn_shard_pda(index)?;
            accounts.push(AccountMeta::new(top_burn_shard_pda, false)); // top_burn_shard (optional)
        }

        // add user profile (optional but required for burn with history)
        accounts.push(AccountMeta::new(user_profile_pda, false)); // user_profile

        // add burn history (required for this instruction)
        accounts.push(AccountMeta::new(user_burn_history_pda, false)); // user_burn_history

        // add burn with history instruction
        base_instructions.push(Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            accounts,
        ));

        // get latest blockhash
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

        // create simulation transaction (without compute budget instruction)
        let sim_message = Message::new(
            &base_instructions,
            Some(&target_pubkey),
        );

        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        sim_transaction.sign(&[&keypair], sim_transaction.message.recent_blockhash);

        // serialize simulation transaction
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);

        // simulate transaction to get compute units consumed
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });

        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // parse simulation result to extract compute units consumed
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Token 2022 burn with history simulation consumed {} compute units", units_consumed);
            // add buffer
            let with_buffer = (units_consumed as f64 * ProgramConfig::COMPUTE_UNIT_BUFFER) as u64;
            // ensure minimum
            std::cmp::max(with_buffer, ProgramConfig::MIN_COMPUTE_UNITS)
        } else {
            log::info!("Failed to get compute units from simulation, using fallback");
            // fallback calculation - burn with history operations usually require more compute units
            480_000u64 // slightly higher than regular burn due to history processing
        };

        log::info!("Using {} compute units for Token 2022 burn with history (with {}% buffer)", computed_units, (ProgramConfig::COMPUTE_UNIT_BUFFER - 1.0) * 100.0);

        // build final transaction with computed compute units
        let mut final_instructions = vec![];

        // first add compute budget instruction
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));

        // add all base instructions
        final_instructions.extend(base_instructions);

        // create and sign final transaction
        let final_message = Message::new(
            &final_instructions,
            Some(&target_pubkey),
        );

        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        final_transaction.sign(&[&keypair], final_transaction.message.recent_blockhash);

        // serialize and send final transaction
        let final_serialized_tx = base64::encode(bincode::serialize(&final_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize final transaction: {}", e)))?);

        let params = serde_json::json!([
            final_serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "confirmed",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        self.send_request("sendTransaction", params).await
    }

    // initialize burn history
    pub async fn initialize_user_burn_history(
        &self,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        let program_id = ProgramConfig::get_program_id()?;
        
        // create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // calculate user profile PDA
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;

        // check if user profile exists
        let profile_info = self.get_account_info(&user_profile_pda.to_string(), Some("base64")).await?;
        let profile_info: serde_json::Value = serde_json::from_str(&profile_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse profile info: {}", e)))?;
        
        if profile_info["value"].is_null() {
            return Err(RpcError::Other("User profile must exist before initializing user burn history".to_string()));
        }

        // parse user profile to get current burn_history_index
        let data = profile_info["value"]["data"][0].as_str()
            .ok_or_else(|| RpcError::Other("Failed to get profile data".to_string()))?;
        let decoded = base64::decode(data)
            .map_err(|e| RpcError::Other(format!("Failed to decode profile data: {}", e)))?;

        // determine next burn history index
        let next_index = if decoded.len() >= 89 {
            let option_tag = decoded[88];
            if option_tag == 1 && decoded.len() >= 97 {
                let current_index = u64::from_le_bytes(decoded[89..97].try_into().unwrap());
                current_index + 1
            } else {
                0 // first burn history
            }
        } else {
            0
        };

        // calculate burn history PDA for the next index
        let (user_burn_history_pda, _) = ProgramConfig::get_user_burn_history_pda(&target_pubkey, next_index)?;

        // get latest blockhash
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

        // create instruction data
        let instruction_data = ProgramConfig::INITIALIZE_USER_BURN_HISTORY_DISCRIMINATOR.to_vec();

        // create the instruction
        let instruction = Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(target_pubkey, true),             // user (signer)
                AccountMeta::new(user_profile_pda, false),         // user_profile
                AccountMeta::new(user_burn_history_pda, false),    // user_burn_history
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // System Program
            ],
        );

        // create the message
        let message = Message::new(
            &[instruction],
            Some(&target_pubkey), // fee payer
        );

        // create and sign transaction
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        transaction.sign(&[&keypair], transaction.message.recent_blockhash);

        // serialize the transaction to base64
        let serialized_tx = base64::encode(bincode::serialize(&transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize transaction: {}", e)))?);

        // send transaction
        let params = serde_json::json!([
            serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "confirmed",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        self.send_request("sendTransaction", params).await
    }

    // get user current burn history index
    pub async fn get_user_burn_history_index(
        &self,
        user_pubkey: &str,
    ) -> Result<Option<u64>, RpcError> {
        let target_pubkey = Pubkey::from_str(user_pubkey)
            .map_err(|e| RpcError::Other(format!("Invalid public key: {}", e)))?;

        // calculate user profile PDA
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;

        // get account info
        match self.get_account_info(&user_profile_pda.to_string(), Some("base64")).await {
            Ok(account_info_str) => {
                let account_info: serde_json::Value = serde_json::from_str(&account_info_str)
                    .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;

                if account_info["value"].is_null() {
                    return Ok(None);
                }

                // parse account data
                if let Some(data) = account_info["value"]["data"].get(0).and_then(|v| v.as_str()) {
                    let decoded = base64::decode(data)
                        .map_err(|e| RpcError::Other(format!("Failed to decode base64 data: {}", e)))?;

                    // UserProfile struct: discriminator(8) + pubkey(32) + total_minted(8) + total_burned(8) + 
                    // mint_count(8) + burn_count(8) + created_at(8) + last_updated(8) + burn_history_index(Option<u64> = 1+8)
                    if decoded.len() >= 89 {
                        let option_tag = decoded[88]; // Option<u64> tag position
                        if option_tag == 1 && decoded.len() >= 97 {
                            // Some variant
                            let user_burn_history_index = u64::from_le_bytes(decoded[89..97].try_into().unwrap());
                            Ok(Some(user_burn_history_index))
                        } else {
                            // None variant
                            Ok(None)
                        }
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            },
            Err(_) => Ok(None) // account not exists
        }
    }

    // get user burn history account info
    pub async fn get_user_burn_history(
        &self,
        user_pubkey: &str,
        index: u64,
    ) -> Result<String, RpcError> {
        let target_pubkey = Pubkey::from_str(user_pubkey)
            .map_err(|e| RpcError::Other(format!("Invalid public key: {}", e)))?;

        // calculate user burn history PDA
        let (user_burn_history_pda, _) = ProgramConfig::get_user_burn_history_pda(&target_pubkey, index)?;

        // get account info
        self.get_account_info(&user_burn_history_pda.to_string(), Some("base64")).await
    }

    // close user burn history account
    pub async fn close_user_burn_history(
        &self,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        let program_id = ProgramConfig::get_program_id()?;
        
        // create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // calculate user profile PDA
        let (user_profile_pda, _) = ProgramConfig::get_user_profile_pda(&target_pubkey)?;

        // check if user has user burn history record
        let current_user_burn_history_index = match self.get_user_burn_history_index(&target_pubkey.to_string()).await? {
            Some(index) => index,
            None => return Err(RpcError::Other("User has no user burn history records to close".to_string())),
        };

        // calculate current user burn history PDA
        let (user_burn_history_pda, _) = ProgramConfig::get_user_burn_history_pda(&target_pubkey, current_user_burn_history_index)?;

        // build base instructions (for simulation)
        let mut base_instructions = vec![];

        // create instruction data
        let instruction_data = ProgramConfig::CLOSE_USER_BURN_HISTORY_DISCRIMINATOR.to_vec();

        // create instruction
        let instruction = Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(target_pubkey, true),             // user (signer)
                AccountMeta::new(user_profile_pda, false),         // user_profile
                AccountMeta::new(user_burn_history_pda, false),    // user_burn_history
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system_program
            ],
        );

        base_instructions.push(instruction);

        // get latest blockhash
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

        // create simulation transaction (without compute budget instruction)
        let sim_message = Message::new(
            &base_instructions,
            Some(&target_pubkey),
        );

        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        sim_transaction.sign(&[&keypair], sim_transaction.message.recent_blockhash);

        // serialize simulation transaction
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);

        // simulate transaction to get compute units consumed
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });

        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // parse simulation result to get compute units consumed
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Close user burn history simulation consumed {} compute units", units_consumed);
            // add 10% buffer
            let with_buffer = (units_consumed as f64 * ProgramConfig::COMPUTE_UNIT_BUFFER) as u64;
            // ensure minimum value
            std::cmp::max(with_buffer, ProgramConfig::MIN_COMPUTE_UNITS)
        } else {
            log::info!("Failed to get compute units from simulation, using fallback");
            // close
            200_000u64
        };

        log::info!("Using {} compute units for close user burn history (with {}% buffer)", computed_units, (ProgramConfig::COMPUTE_UNIT_BUFFER - 1.0) * 100.0);

        // build final transaction, including computed compute units
        let mut final_instructions = vec![];

        // first add compute budget instruction
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));

        // add all base instructions
        final_instructions.extend(base_instructions);

        // create and sign final transaction
        let final_message = Message::new(
            &final_instructions,
            Some(&target_pubkey),
        );

        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = solana_sdk::hash::Hash::from_str(recent_blockhash)
            .map_err(|e| RpcError::Other(format!("Invalid blockhash: {}", e)))?;
        final_transaction.sign(&[&keypair], final_transaction.message.recent_blockhash);

        // serialize and send final transaction
        let final_serialized_tx = base64::encode(bincode::serialize(&final_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize final transaction: {}", e)))?);

        let params = serde_json::json!([
            final_serialized_tx,
            {
                "encoding": "base64",
                "preflightCommitment": "confirmed",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        self.send_request("sendTransaction", params).await
    }

    /// interface: get memo from transaction by signature
    /// call get_transaction_details in rpc_base.rs, then extract memo
    /// return raw memo string to caller
    pub async fn get_transaction_memo(&self, signature: &str) -> Result<Option<String>, RpcError> {
        // call interface to get transaction details
        let transaction_details = self.get_transaction_details(signature).await?;
        
        // parse JSON to extract memo
        let tx_data: serde_json::Value = serde_json::from_str(&transaction_details)
            .map_err(|e| RpcError::Other(format!("Failed to parse transaction details: {}", e)))?;
        
        // check if transaction exists
        if tx_data.is_null() {
            return Ok(None);
        }
        
        // get log messages from meta
        let log_messages = tx_data
            .get("meta")
            .and_then(|meta| meta.get("logMessages"))
            .and_then(|logs| logs.as_array());
        
        if let Some(logs) = log_messages {
            // loop through all log messages, find memo log
            for log_message in logs {
                if let Some(log_str) = log_message.as_str() {
                    // look for "Program log: Memo" pattern
                    if log_str.starts_with("Program log: Memo") {
                        // extract memo content after "Program log: Memo (len XXX): "
                        if let Some(memo_start) = log_str.find("): ") {
                            let memo_content = &log_str[memo_start + 3..]; // skip "): "
                            
                            // the memo content might be JSON-escaped, so we need to unescape it
                            match serde_json::from_str::<String>(memo_content) {
                                Ok(unescaped_memo) => {
                                    return Ok(Some(unescaped_memo));
                                },
                                Err(_) => {
                                    // if it's not JSON-escaped, return as is (minus quotes if present)
                                    let cleaned_memo = memo_content.trim_matches('"');
                                    return Ok(Some(cleaned_memo.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // if no memo found, return None
        Ok(None)
    }
} 