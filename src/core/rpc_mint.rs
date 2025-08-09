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
use spl_associated_token_account;
use spl_memo;
use serde::{Serialize, Deserialize};

/// Supply tier configuration for mint rewards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyTier {
    pub min: u64,
    pub max: u64,
    pub reward: f64,
    pub label: String,
}

// Mint contract configuration
pub struct MintConfig;

impl MintConfig {
    // Mint contract program ID
    pub const MINT_PROGRAM_ID: &'static str = "A31a17bhgQyRQygeZa1SybytjbCdjMpu6oPr9M3iQWzy";
    
    // Authorized mint token address - now using global constant
    pub const TOKEN_MINT: &'static str = "HLCoc7wNDavNMfWWw2Bwd7U7A24cesuhBSNkxZgvZm1";
    
    // Token 2022 Program ID
    pub const TOKEN_2022_PROGRAM_ID: &'static str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    
    // PDA Seeds
    pub const MINT_AUTHORITY_SEED: &'static [u8] = b"mint_authority";
    
    // Memo validation limits (from contract: 69-800 bytes)
    pub const MIN_MEMO_LENGTH: usize = 69;
    pub const MAX_MEMO_LENGTH: usize = 800;
    
    // Compute budget configuration
    pub const COMPUTE_UNIT_BUFFER: f64 = 1.2; // 20% buffer
}

// Helper functions
impl MintConfig {
    pub fn get_mint_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MINT_PROGRAM_ID)
            .map_err(|e| RpcError::Other(format!("Invalid mint program ID: {}", e)))
    }
    
    pub fn get_token_mint() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::TOKEN_MINT)
            .map_err(|e| RpcError::Other(format!("Invalid token mint address: {}", e)))
    }
    
    pub fn get_token_2022_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::TOKEN_2022_PROGRAM_ID)
            .map_err(|e| RpcError::Other(format!("Invalid Token 2022 program ID: {}", e)))
    }
    
    pub fn get_mint_authority_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_mint_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::MINT_AUTHORITY_SEED],
            &program_id
        ))
    }
    
    pub fn validate_memo_length(memo: &str) -> Result<(), RpcError> {
        let memo_length = memo.len();
        if memo_length < Self::MIN_MEMO_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Memo length must be at least {} bytes, got {} bytes", 
                Self::MIN_MEMO_LENGTH,
                memo_length
            )));
        }
        if memo_length > Self::MAX_MEMO_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Memo length cannot exceed {} bytes, got {} bytes", 
                Self::MAX_MEMO_LENGTH,
                memo_length
            )));
        }
        Ok(())
    }
    
    pub fn get_process_mint_discriminator() -> [u8; 8] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"global:process_mint");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }

    /// Get all available supply tiers
    pub fn get_supply_tiers() -> Vec<SupplyTier> {
        vec![
            SupplyTier { min: 0, max: 100_000_000_000_000, reward: 1.0, label: "0-100M".to_string() },
            SupplyTier { min: 100_000_000_000_000, max: 1_000_000_000_000_000, reward: 0.1, label: "100M-1B".to_string() },
            SupplyTier { min: 1_000_000_000_000_000, max: 10_000_000_000_000_000, reward: 0.01, label: "1B-10B".to_string() },
            SupplyTier { min: 10_000_000_000_000_000, max: 100_000_000_000_000_000, reward: 0.001, label: "10B-100B".to_string() },
            SupplyTier { min: 100_000_000_000_000_000, max: 1_000_000_000_000_000_000, reward: 0.0001, label: "100B-1T".to_string() },
            SupplyTier { min: 1_000_000_000_000_000_000, max: u64::MAX, reward: 0.000001, label: "1T+".to_string() },
        ]
    }
    
    /// Get the current supply tier based on total supply
    pub fn get_current_supply_tier(supply: u64) -> SupplyTier {
        Self::get_supply_tiers().into_iter()
            .find(|tier| supply >= tier.min && supply < tier.max)
            .unwrap_or_else(|| Self::get_supply_tiers().last().unwrap().clone())
    }
    
    /// Calculate mint reward amount based on current supply
    pub fn calculate_mint_reward(supply: u64) -> f64 {
        let tier = Self::get_current_supply_tier(supply);
        tier.reward
    }
    
    /// Format mint amount for display (smart decimal precision)
    pub fn format_mint_reward(amount: f64) -> String {
        // If it's a whole number, don't show decimals
        if amount.fract() == 0.0 {
            format!("+{} MEMO", amount as u64)
        } else if amount >= 1.0 {
            // For values >= 1, show minimal decimals (remove trailing zeros)
            let formatted = format!("{}", amount);
            format!("+{} MEMO", formatted)
        } else {
            // For values < 1, show appropriate precision (remove trailing zeros)
            let formatted = if amount >= 0.1 {
                format!("{:.1}", amount)
            } else if amount >= 0.01 {
                format!("{:.2}", amount)
            } else if amount >= 0.001 {
                format!("{:.3}", amount)
            } else if amount >= 0.0001 {
                format!("{:.4}", amount)
            } else if amount >= 0.00001 {
                format!("{:.5}", amount)
            } else {
                format!("{:.6}", amount)
            };
            // Remove trailing zeros
            let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
            format!("+{} MEMO", trimmed)
        }
    }

    /// Calculate visual progress percentage for supply progress bar
    pub fn calculate_visual_progress_percentage(supply: u64) -> f64 {
        // Define visual breakpoints to make the left range look bigger
        let visual_breakpoints = [
            (0u64, 0.0f64),                           // 0% 
            (100_000_000_000_000u64, 50.0f64),        // 50% - 0-100M tier gets 50% space
            (1_000_000_000_000_000u64, 75.0f64),      // 75% - 100M-1B tier gets 25% space  
            (10_000_000_000_000_000u64, 87.0f64),     // 87% - 1B-10B tier gets 12% space
            (100_000_000_000_000_000u64, 95.0f64),    // 95% - 10B-100B tier gets 8% space
            (1_000_000_000_000_000_000u64, 100.0f64), // 100% - 100B-1T tier gets 5% space
        ];
        
        // Find current supply in which interval
        for i in 0..visual_breakpoints.len() - 1 {
            let (lower_supply, lower_percent) = visual_breakpoints[i];
            let (upper_supply, upper_percent) = visual_breakpoints[i + 1];
            
            if supply >= lower_supply && supply <= upper_supply {
                if upper_supply == lower_supply {
                    return lower_percent;
                }
                
                // Linear interpolation in the interval
                let ratio = (supply - lower_supply) as f64 / (upper_supply - lower_supply) as f64;
                return lower_percent + ratio * (upper_percent - lower_percent);
            }
        }
        
        // If out of range, return 100%
        100.0
    }
    
    /// Calculate visual position for tier markers
    pub fn calculate_visual_marker_position(tier_max: u64) -> f64 {
        Self::calculate_visual_progress_percentage(tier_max)
    }
}

// Mint RPC client implementation
impl RpcConnection {
    /// Mint tokens using the memo mint contract
    /// 
    /// # Parameters
    /// * `memo` - The memo text (must be 69-800 bytes)
    /// * `keypair_bytes` - The user's keypair bytes for signing
    /// 
    /// # Returns
    /// Transaction signature on success
    pub async fn mint(
        &self,
        memo: &str,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        // Validate memo length
        MintConfig::validate_memo_length(memo)?;
        
        log::info!("Starting mint operation with memo length: {} bytes", memo.len());
        
        // Get configuration values
        let mint_program_id = MintConfig::get_mint_program_id()?;
        let mint = MintConfig::get_token_mint()?;
        let token_2022_program_id = MintConfig::get_token_2022_program_id()?;
        
        // Create keypair from bytes and get pubkey
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let user_pubkey = keypair.pubkey();
        
        log::info!("User pubkey: {}", user_pubkey);
        
        // Calculate mint authority PDA
        let (mint_authority_pda, _) = MintConfig::get_mint_authority_pda()?;
        
        // Calculate user's token account (ATA) using Token 2022 program
        let token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &user_pubkey,
            &mint,
            &token_2022_program_id,
        );
        
        log::info!("Token account: {}", token_account);
        log::info!("Mint authority PDA: {}", mint_authority_pda);
        
        // Check if token account exists
        let token_account_info = self.get_account_info(&token_account.to_string(), Some("base64")).await?;
        let token_account_info: serde_json::Value = serde_json::from_str(&token_account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse token account info: {}", e)))?;
        
        // Build base instructions (for simulation first)
        let mut base_instructions = vec![];
        
        // Add memo instruction first
        base_instructions.push(spl_memo::build_memo(
            memo.as_bytes(),
            &[&user_pubkey],
        ));
        
        // If token account doesn't exist, add create ATA instruction for Token 2022
        if token_account_info["value"].is_null() {
            log::info!("Token account does not exist, creating it...");
            base_instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                    &user_pubkey,           // Funding account (fee payer)
                    &user_pubkey,           // Wallet address  
                    &mint,                  // Mint address
                    &token_2022_program_id  // Token 2022 program ID
                )
            );
        }
        
        // Create the mint instruction
        let instruction_data = MintConfig::get_process_mint_discriminator().to_vec();
        
        let accounts = vec![
            AccountMeta::new(user_pubkey, true),                    // user (signer)
            AccountMeta::new(mint, false),                          // mint
            AccountMeta::new_readonly(mint_authority_pda, false),   // mint_authority PDA
            AccountMeta::new(token_account, false),                 // token_account
            AccountMeta::new_readonly(token_2022_program_id, false), // Token 2022 program
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions sysvar
        ];
        
        // Add mint instruction
        base_instructions.push(Instruction::new_with_bytes(
            mint_program_id,
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
            Some(&user_pubkey),
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
        
        // Parse simulation result to extract compute units consumed
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Mint simulation consumed {} compute units", units_consumed);
            // 10% buffer
            (units_consumed as f64 * MintConfig::COMPUTE_UNIT_BUFFER) as u64
        } else {
            return Err(RpcError::Other(
                "Failed to get compute units from simulation - cannot proceed without accurate CU estimation".to_string()
            ));
        };
        
        log::info!("Using {} compute units for mint (simulation: {}, +20% buffer)", 
            computed_units, 
            sim_result["value"]["unitsConsumed"].as_u64().unwrap_or(0)
        );
        
        // Now build the final transaction with the calculated compute units
        let mut final_instructions = vec![];
        
        // Add compute budget instruction first
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));
        
        // Add all the base instructions
        final_instructions.extend(base_instructions);
        
        // Create and sign final transaction
        let final_message = Message::new(
            &final_instructions,
            Some(&user_pubkey),
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
        
        log::info!("Sending mint transaction...");
        let result = self.send_request("sendTransaction", params).await?;
        log::info!("Mint transaction sent successfully");
        
        Ok(result)
    }

    /// Get the current supply of the mint token
    /// 
    /// # Returns
    /// The current supply as u64 (in lamports)
    pub async fn get_token_supply(&self) -> Result<u64, RpcError> {
        let mint = MintConfig::get_token_mint()?;
        
        let params = serde_json::json!([
            mint.to_string(),
            {
                "commitment": "confirmed"
            }
        ]);
        
        log::info!("Getting token supply for mint: {}", mint);
        
        let result: serde_json::Value = self.send_request("getTokenSupply", params).await?;
        
        // Parse the supply from the response
        if let Some(supply_str) = result.get("value")
            .and_then(|v| v.get("amount"))
            .and_then(|a| a.as_str()) 
        {
            let supply = supply_str.parse::<u64>()
                .map_err(|e| RpcError::Other(format!("Failed to parse supply as u64: {}", e)))?;
            
            log::info!("Current token supply: {} lamports", supply);
            Ok(supply)
        } else {
            Err(RpcError::Other("Failed to extract supply from response".to_string()))
        }
    }

    /// Get current mint reward amount for the current supply
    /// 
    /// # Returns
    /// The formatted mint reward string (e.g., "+1.000000 MEMO")
    pub async fn get_current_mint_reward_formatted(&self) -> Result<String, RpcError> {
        let supply = self.get_token_supply().await?;
        let reward_amount = MintConfig::calculate_mint_reward(supply);
        Ok(MintConfig::format_mint_reward(reward_amount))
    }

    /// Get current supply tier information
    /// 
    /// # Returns
    /// Tuple of (current_supply, current_tier)
    pub async fn get_current_supply_tier_info(&self) -> Result<(u64, SupplyTier), RpcError> {
        let supply = self.get_token_supply().await?;
        let tier = MintConfig::get_current_supply_tier(supply);
        Ok((supply, tier))
    }
}