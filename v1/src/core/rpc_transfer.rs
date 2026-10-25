use super::rpc_base::{RpcConnection, RpcError, get_token_mint};
use solana_sdk::{
    message::Message,
    pubkey::Pubkey,
    transaction::Transaction,
    system_instruction,
    compute_budget::ComputeBudgetInstruction,
};
use spl_token_2022::instruction as token_instruction;
use std::str::FromStr;
use base64;
use bincode;

impl RpcConnection {
    /// Build a transfer transaction for native tokens (XNT/SOL)
    /// 
    /// # Parameters
    /// * `from_pubkey` - Sender's public key
    /// * `to_address` - Recipient's address string
    /// * `amount_lamports` - Amount to transfer in lamports
    /// 
    /// # Returns
    /// Unsigned transaction ready for signing
    pub async fn build_native_transfer_transaction(
        &self,
        from_pubkey: &Pubkey,
        to_address: &str,
        amount_lamports: u64,
    ) -> Result<Transaction, RpcError> {
        log::info!("Building native transfer transaction: {} lamports to {}", amount_lamports, to_address);
        
        // Parse recipient address
        let to_pubkey = Pubkey::from_str(to_address)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid recipient address: {}", e)))?;
        
        // Build base instructions (transfer instruction)
        let mut base_instructions = Vec::new();
        
        // Create transfer instruction
        let transfer_ix = system_instruction::transfer(from_pubkey, &to_pubkey, amount_lamports);
        base_instructions.push(transfer_ix);
        
        // Get latest blockhash
        let blockhash = self.get_latest_blockhash().await?;
        
        // Simulate with dummy compute budget instructions for accurate CU estimation
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(200_000u32));
        
        // If user has set a price, include it in simulation to match final transaction
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
            }
        }
        
        let sim_message = Message::new(&sim_instructions, Some(from_pubkey));
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
        
        log::info!("Simulating native transfer transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Native transfer simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction with compute budget
        let mut final_instructions = base_instructions;
        
        // Add compute budget instructions using unified method (1.1x buffer for safety)
        let compute_budget_ixs = RpcConnection::build_compute_budget_instructions(
            simulated_cu,
            1.1
        );
        final_instructions.extend(compute_budget_ixs);
        
        let message = Message::new(&final_instructions, Some(from_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        
        log::info!("Native transfer transaction built successfully");
        Ok(transaction)
    }
    
    /// Build a transfer transaction for SPL tokens (MEMO)
    /// 
    /// # Parameters
    /// * `from_pubkey` - Sender's public key
    /// * `to_address` - Recipient's address string
    /// * `amount` - Amount to transfer in token units (with decimals)
    /// 
    /// # Returns
    /// Unsigned transaction ready for signing
    pub async fn build_token_transfer_transaction(
        &self,
        from_pubkey: &Pubkey,
        to_address: &str,
        amount: u64,
    ) -> Result<Transaction, RpcError> {
        log::info!("Building token transfer transaction: {} tokens to {}", amount, to_address);
        
        // Parse recipient address
        let to_pubkey = Pubkey::from_str(to_address)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid recipient address: {}", e)))?;
        
        // Get token mint address
        let token_mint = get_token_mint()
            .map_err(|e| RpcError::Other(format!("Failed to get token mint: {}", e)))?;
        
        // Get token program ID (Token-2022)
        let token_program_id = spl_token_2022::id();
        
        // Derive source token account (Associated Token Account)
        let source_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            from_pubkey,
            &token_mint,
            &token_program_id,
        );
        
        // Derive destination token account (Associated Token Account)
        let dest_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &to_pubkey,
            &token_mint,
            &token_program_id,
        );
        
        // Build base instructions
        let mut base_instructions = Vec::new();
        
        // Check if destination token account exists
        log::info!("Checking if destination token account exists: {}", dest_token_account);
        let dest_account_info = self.get_account_info(&dest_token_account.to_string(), Some("base64")).await?;
        let dest_account_info: serde_json::Value = serde_json::from_str(&dest_account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        // If destination account doesn't exist, create it
        if dest_account_info["value"].is_null() {
            log::info!("Destination token account does not exist, creating it");
            let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
                from_pubkey,
                &to_pubkey,
                &token_mint,
                &token_program_id,
            );
            base_instructions.push(create_ata_ix);
        } else {
            log::info!("Destination token account already exists");
        }
        
        // Create transfer instruction
        let transfer_ix = token_instruction::transfer_checked(
            &token_program_id,
            &source_token_account,
            &token_mint,
            &dest_token_account,
            from_pubkey,
            &[],
            amount,
            6, // MEMO token has 6 decimals
        ).map_err(|e| RpcError::Other(format!("Failed to create transfer instruction: {}", e)))?;
        base_instructions.push(transfer_ix);
        
        // Get latest blockhash
        let blockhash = self.get_latest_blockhash().await?;
        
        // Simulate with dummy compute budget instructions for accurate CU estimation
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(400_000u32));
        
        // If user has set a price, include it in simulation to match final transaction
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
            }
        }
        
        let sim_message = Message::new(&sim_instructions, Some(from_pubkey));
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
        
        log::info!("Simulating token transfer transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Token transfer simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction with compute budget
        let mut final_instructions = base_instructions;
        
        // Add compute budget instructions using unified method (1.1x buffer for safety)
        let compute_budget_ixs = RpcConnection::build_compute_budget_instructions(
            simulated_cu,
            1.1
        );
        final_instructions.extend(compute_budget_ixs);
        
        let message = Message::new(&final_instructions, Some(from_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        
        log::info!("Token transfer transaction built successfully");
        Ok(transaction)
    }
}

