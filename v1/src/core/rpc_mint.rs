use super::rpc_base::{
    RpcConnection, RpcError,
    get_token_mint, get_token_2022_program_id, validate_memo_length_str
};
use super::network_config::get_program_ids;
use super::constants::*;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
    message::Message,
    compute_budget::ComputeBudgetInstruction,
};
use base64;
use bincode;
use spl_associated_token_account;
use spl_memo;

/// One reward step of the emission curve: a mint at supply in (start, end] pays `reward_units`
#[derive(Debug, Clone, PartialEq)]
pub struct EmissionStep {
    pub start: u64,
    pub end: u64,
    pub reward_units: u64,
}

/// A stage of the emission curve: the flat start (0, 10M], then one stage per decade
/// (10M * 10^n, 10M * 10^(n+1)], each split into 9 equal steps
#[derive(Debug, Clone, PartialEq)]
pub struct EmissionStage {
    pub start: u64,
    pub end: u64,
    pub steps: Vec<EmissionStep>,
}

// Emission curve, identical to memo-mint 0.4.0 (amounts in units, 1 token = 1,000,000 units).
// Up to and including 10M every mint pays 1 token. Above it, supply in (d * 10^k, (d+1) * 10^k]
// inside the decade (10M * 10^n, 10M * 10^(n+1)] pays decade_base * (10 - d) / 10,
// decade_base = 1 token / 10^n: the reward drops by a tenth of the decade base just above every
// round number (a mint at exactly a round number still pays the previous step):
// up to 10M: 1, above 10M: 0.9, ..., above 90M: 0.1, above 100M: 0.09, ..., above 1B: 0.009, ...
// down to 1 unit above 9T.
const UNITS_PER_TOKEN: u64 = 1_000_000;
const CURVE_START_UNITS: u64 = 10_000_000 * UNITS_PER_TOKEN;
const BASE_MINT_UNITS: u64 = UNITS_PER_TOKEN;
const MAX_SUPPLY_UNITS: u64 = 10_000_000_000_000 * UNITS_PER_TOKEN;

// Mint contract configuration
pub struct MintConfig;

impl MintConfig {
    // Note: Mint program ID, Token mint, and Token 2022 program ID are now 
    // retrieved dynamically from network configuration
    
    // PDA Seeds
    pub const MINT_AUTHORITY_SEED: &'static [u8] = b"mint_authority";
    
    // Note: Memo validation limits and compute unit config are now directly 
    // used from the constants module to avoid duplication
}

// Helper functions
impl MintConfig {
    pub fn get_mint_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.mint_program_id)
            .map_err(|e| RpcError::Other(format!("Invalid mint program ID: {}", e)))
    }
    
    // Note: get_token_mint(), get_token_2022_program_id(), and validate_memo_length()
    // are now provided by rpc_base module to avoid duplication
    
    pub fn get_mint_authority_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_mint_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::MINT_AUTHORITY_SEED],
            &program_id
        ))
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

    /// Exact mint amount in units at the given supply (0 once the 10T cap is reached),
    /// same arithmetic as the memo-mint contract
    pub fn calculate_mint_amount_units(supply: u64) -> u64 {
        if supply >= MAX_SUPPLY_UNITS {
            return 0;
        }
        let amount = if supply <= CURVE_START_UNITS {
            BASE_MINT_UNITS
        } else {
            // Steps take effect just above each round number
            let below = supply - 1;
            let mut magnitude = CURVE_START_UNITS;
            let mut decade_base = BASE_MINT_UNITS;
            while below / 10 >= magnitude {
                magnitude *= 10;
                decade_base /= 10;
            }
            let digit = below / magnitude; // 1..=9
            decade_base * (10 - digit) / 10
        };
        if supply.saturating_add(amount) > MAX_SUPPLY_UNITS { 0 } else { amount }
    }

    /// All stages of the emission curve: (0, 10M] pays 1, then 6 decades of 9 steps each
    pub fn emission_stages() -> Vec<EmissionStage> {
        let mut stages = vec![EmissionStage {
            start: 0,
            end: CURVE_START_UNITS,
            steps: vec![EmissionStep { start: 0, end: CURVE_START_UNITS, reward_units: BASE_MINT_UNITS }],
        }];
        let mut magnitude = CURVE_START_UNITS;
        let mut decade_base = BASE_MINT_UNITS;
        while magnitude < MAX_SUPPLY_UNITS {
            let steps = (1..=9u64)
                .map(|d| EmissionStep {
                    start: d * magnitude,
                    end: (d + 1) * magnitude,
                    reward_units: decade_base * (10 - d) / 10,
                })
                .collect();
            stages.push(EmissionStage { start: magnitude, end: magnitude * 10, steps });
            magnitude *= 10;
            decade_base /= 10;
        }
        stages
    }

    /// Index of the stage and step that a mint at this supply falls in (the last step once the cap is reached)
    pub fn locate_step(supply: u64) -> (usize, usize) {
        let stages = Self::emission_stages();
        for (i, stage) in stages.iter().enumerate() {
            if let Some(j) = stage.steps.iter().position(|step| supply <= step.end) {
                return (i, j);
            }
        }
        (stages.len() - 1, 8)
    }

    /// Position of this supply along the curve chart, 0.0 to 1.0: every stage gets the same width,
    /// and inside a stage supply is linear, so each of its 9 steps is equally wide
    pub fn curve_position(supply: u64) -> f64 {
        let stages = Self::emission_stages();
        let (i, _) = Self::locate_step(supply);
        let stage = &stages[i];
        let ratio = (supply.min(stage.end).saturating_sub(stage.start)) as f64
            / (stage.end - stage.start) as f64;
        (i as f64 + ratio) / stages.len() as f64
    }

    /// The next reward reduction: (supply it takes effect above, reward after it in units).
    /// None once the last step (1 unit) is reached.
    pub fn next_reduction(supply: u64) -> Option<(u64, u64)> {
        Self::emission_stages()
            .iter()
            .flat_map(|stage| stage.steps.iter())
            .find(|step| step.end >= supply)
            .and_then(|step| {
                let after = Self::calculate_mint_amount_units(step.end + 1);
                (step.end < MAX_SUPPLY_UNITS && after > 0).then_some((step.end, after))
            })
    }
    
    /// Calculate mint reward (in tokens) based on current supply
    pub fn calculate_mint_reward(supply: u64) -> f64 {
        Self::calculate_mint_amount_units(supply) as f64 / UNITS_PER_TOKEN as f64
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
}

// Mint RPC client implementation
impl RpcConnection {
    /// Build an unsigned mint transaction
    /// 
    /// # Parameters
    /// * `user_pubkey` - The user's public key
    /// * `memo` - The memo text (at least 69 bytes; this client caps it at 800, the transaction size limit applies)
    /// 
    /// # Returns
    /// An unsigned Transaction ready to be signed
    pub async fn build_mint_transaction(
        &self,
        user_pubkey: &Pubkey,
        memo: &str,
    ) -> Result<Transaction, RpcError> {
        // Validate memo length
        validate_memo_length_str(memo)?;
        
        log::info!("Building mint transaction for user: {} with memo length: {} bytes", user_pubkey, memo.len());
        
        // Get configuration values
        let mint_program_id = MintConfig::get_mint_program_id()?;
        let mint = get_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        
        // Calculate mint authority PDA
        let (mint_authority_pda, _) = MintConfig::get_mint_authority_pda()?;
        
        // Calculate user's token account (ATA) using Token 2022 program
        let token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey,
            &mint,
            &token_2022_program_id,
        );
        
        log::info!("Token account: {}", token_account);
        log::info!("Mint authority PDA: {}", mint_authority_pda);
        
        // Check if token account exists
        let token_account_info = self.get_account_info(&token_account.to_string(), Some("base64")).await?;
        let token_account_info: serde_json::Value = serde_json::from_str(&token_account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse token account info: {}", e)))?;
        
        // Build base instructions
        let mut base_instructions = vec![];
        
        // Add memo instruction first
        base_instructions.push(spl_memo::build_memo(
            memo.as_bytes(),
            &[user_pubkey],
        ));
        
        // If token account doesn't exist, add create ATA instruction for Token 2022
        if token_account_info["value"].is_null() {
            log::info!("Token account does not exist, will create it");
            base_instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                    user_pubkey,           // Funding account (fee payer)
                    user_pubkey,           // Wallet address  
                    &mint,                 // Mint address
                    &token_2022_program_id // Token 2022 program ID
                )
            );
        }
        
        // Create the mint instruction
        let instruction_data = MintConfig::get_process_mint_discriminator().to_vec();
        
        let accounts = vec![
            AccountMeta::new(*user_pubkey, true),                   // user (signer)
            AccountMeta::new(mint, false),                          // mint
            AccountMeta::new_readonly(mint_authority_pda, false),   // mint_authority PDA
            AccountMeta::new(token_account, false),                 // token_account
            AccountMeta::new_readonly(token_2022_program_id, false), // Token 2022 program
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions sysvar
            AccountMeta::new(super::rpc_base::get_mint_state_pda()?, false), // mint_state (last; one mint per transaction)
        ];
        
        // Add mint instruction
        base_instructions.push(Instruction::new_with_bytes(
            mint_program_id,
            &instruction_data,
            accounts,
        ));
        
        // Get latest blockhash
        let recent_blockhash = self.get_latest_blockhash().await?;
        
        // Create simulation transaction with dummy compute budget instructions
        // Contract requires memo at index 0, so compute budget must be after base instructions
        // Note: Compute budget instructions are processed by Solana runtime before instruction execution
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(400_000));
        
        // If user has set a price, include it in simulation to match final transaction
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
            }
        }
        
        let sim_message = Message::new(
            &sim_instructions,
            Some(user_pubkey),
        );
        
        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = recent_blockhash;
        
        // Note: For simulation, we need a signed transaction, but we'll use replaceRecentBlockhash
        // The actual signature will be replaced later
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);
        
        // Simulate transaction to get compute units consumption
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false  // Don't verify signature in simulation
        });
        
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse simulation result to extract compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Mint simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other(
                "Failed to get compute units from simulation".to_string()
            ));
        };
        
        // Build the final transaction with compute budget
        // Contract requires memo at index 0, so add base instructions first, then compute budget
        let mut final_instructions = base_instructions;
        
        // Add compute budget instructions (limit + optional price) using unified method
        let compute_budget_ixs = RpcConnection::build_compute_budget_instructions(
            simulated_cu,
            COMPUTE_UNIT_BUFFER
        );
        final_instructions.extend(compute_budget_ixs);
        
        // Create final unsigned transaction
        let final_message = Message::new(
            &final_instructions,
            Some(user_pubkey),
        );
        
        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = recent_blockhash;
        
        log::info!("Mint transaction built successfully, ready for signing");
        
        Ok(final_transaction)
    }

    /// Get the current supply of the mint token
    /// 
    /// # Returns
    /// The current supply as u64 (in lamports)
    pub async fn get_token_supply(&self) -> Result<u64, RpcError> {
        let mint = get_token_mint()?;
        
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

    /// Get token holders using getProgramAccounts
    /// Returns token accounts sorted by balance (descending)
    /// Note: For Token-2022 with extensions, account size varies
    /// 
    /// # Parameters
    /// * `limit` - Maximum number of top holders to return (e.g., 100)
    /// 
    /// # Returns
    /// Vector of (owner_address, balance) tuples, sorted by balance descending
    pub async fn get_token_holders(&self, limit: usize) -> Result<Vec<(String, f64)>, RpcError> {
        let token_mint = get_token_mint()?.to_string();
        let token_program_id = get_token_2022_program_id()?.to_string();
        
        log::info!("Fetching token holders for Token-2022 mint: {}", token_mint);
        
        // For Token-2022 with extensions, we use memcmp to filter by mint
        // and don't use dataSize filter since extensions make account size variable
        let params = serde_json::json!([
            token_program_id,
            {
                "encoding": "jsonParsed",
                "filters": [
                    {
                        "memcmp": {
                            "offset": 0,  // mint pubkey is at offset 0
                            "bytes": token_mint
                        }
                    }
                ]
            }
        ]);
        
        let result: serde_json::Value = self.send_request("getProgramAccounts", params).await?;
        
        // Parse the response
        let mut holders: Vec<(String, f64)> = Vec::new();
        
        if let Some(accounts) = result.as_array() {
            for account in accounts {
                if let Some(info) = account.get("account")
                    .and_then(|a| a.get("data"))
                    .and_then(|d| d.get("parsed"))
                    .and_then(|p| p.get("info"))
                {
                    // Get owner address
                    if let Some(owner) = info.get("owner").and_then(|o| o.as_str()) {
                        // Get token amount
                        if let Some(amount) = info
                            .get("tokenAmount")
                            .and_then(|t| t.get("uiAmount"))
                            .and_then(|a| a.as_f64())
                        {
                            if amount > 0.0 {
                                holders.push((owner.to_string(), amount));
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by balance (descending)
        holders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit to top N
        holders.truncate(limit);
        
        log::info!("Found {} token holders (limited to top {})", holders.len(), limit);
        
        Ok(holders)
    }
}