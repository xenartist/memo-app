use super::base_rpc::{RpcConnection, RpcError};
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

impl RpcConnection {
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
        self.get_account_info(&user_profile_pda.to_string(), Some("base64")).await
    }

    pub async fn initialize_user_profile(
        &self, 
        username: &str, 
        profile_image: &str,
        keypair_bytes: &[u8]
    ) -> Result<String, RpcError> {
        // Program ID
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))?;
        
        // Create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // Validate inputs
        if username.len() > 32 {
            return Err(RpcError::Other("Username too long. Maximum length is 32 characters.".to_string()));
        }
        if profile_image.len() > 256 {
            return Err(RpcError::Other("Profile image too long. Maximum length is 256 characters.".to_string()));
        }

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

        self.send_request("sendTransaction", params).await
    }

    pub async fn close_user_profile(
        &self,
        keypair_bytes: &[u8]
    ) -> Result<String, RpcError> {
        // Program ID
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))?;
        
        // Create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

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

        self.send_request("sendTransaction", params).await
    }

    pub async fn update_user_profile(
        &self,
        username: Option<String>,
        profile_image: Option<String>,
        keypair_bytes: &[u8]
    ) -> Result<String, RpcError> {
        // Program ID
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))?;
        
        // Create keypair from bytes
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

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

        self.send_request("sendTransaction", params).await
    }

    pub async fn get_latest_burn_shard(&self) -> Result<String, RpcError> {
        // program ID
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::InvalidParameter(format!("Invalid program ID: {}", e)))?;

        // calculate PDA
        let (latest_burn_shard_pda, _) = Pubkey::find_program_address(
            &[b"latest_burn_shard"],
            &program_id,
        );

        // get account info
        self.get_account_info(&latest_burn_shard_pda.to_string(), Some("base64")).await
    }

    pub async fn mint(
        &self,
        memo: &str,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        // Program and mint addresses
        let program_id = Pubkey::from_str("TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw")
            .map_err(|e| RpcError::Other(format!("Invalid program ID: {}", e)))?;
        let mint = Pubkey::from_str("CrfhYtP7XtqFyHTWMyXp25CCzhjhzojngrPCZJ7RarUz")
            .map_err(|e| RpcError::Other(format!("Invalid mint address: {}", e)))?;

        // Create keypair from bytes and get pubkey
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let target_pubkey = keypair.pubkey();

        // Calculate PDAs
        let (mint_authority_pda, _) = Pubkey::find_program_address(
            &[b"mint_authority"],
            &program_id,
        );

        // Calculate user profile PDA
        let (user_profile_pda, _) = Pubkey::find_program_address(
            &[b"user_profile", target_pubkey.as_ref()],
            &program_id,
        );

        // Calculate token account (ATA)
        let token_account = spl_associated_token_account::get_associated_token_address(
            &target_pubkey,
            &mint,
        );

        // Check if token account exists
        let token_account_info = self.get_account_info(&token_account.to_string(), Some("base64")).await?;
        let token_account_info: serde_json::Value = serde_json::from_str(&token_account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse token account info: {}", e)))?;

        // Verify memo length
        let memo_length = memo.len();
        if memo_length < 69 {
            return Err(RpcError::Other("Memo length must be at least 69 bytes".to_string()));
        }
        if memo_length > 700 {
            return Err(RpcError::Other("Memo length cannot exceed 700 bytes".to_string()));
        }

        // Build instructions without compute budget first (for simulation)
        let mut base_instructions = vec![];

        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(
            memo.as_bytes(),
            &[&target_pubkey],
        ));

        // If token account doesn't exist, add create ATA instruction
        if token_account_info["value"].is_null() {
            log::info!("Token account does not exist, creating it...");
            base_instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &target_pubkey,  // Funding account (fee payer)
                    &target_pubkey,  // Wallet address
                    &mint,           // Mint address
                    &spl_token::id() // Token program
                )
            );
        }

        // Calculate Anchor instruction sighash for mint
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"global:process_transfer");
        let result = hasher.finalize();
        let instruction_data = result[..8].to_vec();

        // Create mint instruction accounts
        let mut accounts = vec![
            AccountMeta::new(target_pubkey, true),         // user
            AccountMeta::new(mint, false),                 // mint
            AccountMeta::new(mint_authority_pda, false),   // mint_authority
            AccountMeta::new(token_account, false),        // token_account
            AccountMeta::new_readonly(spl_token::id(), false), // token_program
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
                "commitment": "finalized",
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
            "commitment": "finalized",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });

        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse simulation result to extract compute units consumed
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Simulation consumed {} compute units", units_consumed);
            // Add 10% buffer to the simulated consumption
            let with_buffer = (units_consumed as f64 * 1.1) as u64;
            // Ensure minimum of 1000 CU
            std::cmp::max(with_buffer, 1000)
        } else {
            log::info!("Failed to get compute units from simulation, using fallback");
            // Fallback to original calculation if simulation fails
            match memo_length {
                69..=100 => 100_000,
                101..=200 => 150_000,
                201..=300 => 200_000,
                301..=400 => 250_000,
                401..=500 => 300_000,
                501..=600 => 350_000,
                601..=700 => 400_000,
                _ => 400_000
            }
        };

        log::info!("Using {} compute units (with 10% buffer)", computed_units);

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
                "preflightCommitment": "finalized",
                "skipPreflight": false,
                "maxRetries": 3
            }
        ]);

        self.send_request("sendTransaction", params).await
    }
} 