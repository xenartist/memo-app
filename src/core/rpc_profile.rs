use super::rpc_base::{RpcConnection, RpcError};
use serde::{Serialize, Deserialize};
use borsh::{BorshSerialize, BorshDeserialize};
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signer},
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
    message::Message,
    compute_budget::ComputeBudgetInstruction,
};
use spl_memo;
use base64;
use bincode;
use wasm_bindgen::prelude::*;
use spl_associated_token_account;
use log;
use sha2::{Digest, Sha256};

/// struct of user profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user: String,                // user pubkey (converted from Pubkey to String)
    pub username: String,            // max 32 chars
    pub image: String,               // profile image (hex string), max 256 chars
    pub created_at: i64,             // created timestamp (i64)
    pub last_updated: i64,           // last updated timestamp (i64)
    pub about_me: Option<String>,    // about me, max 128 characters, optional
    pub bump: u8,                    // PDA bump
}

/// Profile creation data structure
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ProfileCreationData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "profile" for memo-profile contract)
    pub category: String,
    
    /// Operation type (must be "create_profile" for profile creation)
    pub operation: String,
    
    /// User pubkey as string (must match the transaction signer)
    pub user_pubkey: String,
    
    /// Username (required, 1-32 characters)
    pub username: String,
    
    /// Profile image info (optional, max 256 characters)
    pub image: String,
    
    /// About me description (optional, max 128 characters)
    pub about_me: Option<String>,
}

impl ProfileCreationData {
    pub fn new(user_pubkey: String, username: String, image: String, about_me: Option<String>) -> Self {
        Self {
            version: 1, // PROFILE_CREATION_DATA_VERSION
            category: "profile".to_string(), // EXPECTED_CATEGORY
            operation: "create_profile".to_string(), // EXPECTED_OPERATION
            user_pubkey,
            username,
            image,
            about_me,
        }
    }
    
    /// Validate the structure fields
    pub fn validate(&self, expected_user: Pubkey) -> Result<(), RpcError> {
        // Validate version
        if self.version != 1 {
            return Err(RpcError::Other(format!("Unsupported profile creation data version: {} (expected: 1)", self.version)));
        }
        
        // Validate category (must be exactly "profile")
        if self.category != "profile" {
            return Err(RpcError::Other(format!("Invalid category: '{}' (expected: 'profile')", self.category)));
        }
        
        // Validate operation (must be exactly "create_profile")
        if self.operation != "create_profile" {
            return Err(RpcError::Other(format!("Invalid operation: '{}' (expected: 'create_profile')", self.operation)));
        }
        
        // Validate user_pubkey matches expected user
        let parsed_pubkey = Pubkey::from_str(&self.user_pubkey)
            .map_err(|_| RpcError::Other(format!("Invalid user_pubkey format: {}", self.user_pubkey)))?;
        
        if parsed_pubkey != expected_user {
            return Err(RpcError::Other(format!("User pubkey mismatch: memo {} vs expected {}", parsed_pubkey, expected_user)));
        }
        
        // Validate username (required, 1-32 characters)
        if self.username.is_empty() || self.username.len() > 32 {
            return Err(RpcError::Other(format!("Invalid username: '{}' (must be 1-32 characters)", self.username)));
        }
        
        // Validate image (optional, max 256 characters)
        if self.image.len() > 256 {
            return Err(RpcError::Other(format!("Invalid profile image: {} characters (max: 256)", self.image.len())));
        }
        
        // Validate about_me (optional, max 128 characters)
        if let Some(ref about_me) = self.about_me {
            if about_me.len() > 128 {
                return Err(RpcError::Other(format!("Invalid about_me: {} characters (max: 128)", about_me.len())));
            }
        }
        
        Ok(())
    }
}

/// Burn memo structure (consistent with memo-burn)
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct BurnMemo {
    pub version: u8,
    pub burn_amount: u64,
    pub payload: Vec<u8>, // ProfileCreationData
}

/// Profile update data structure
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct ProfileUpdateData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "profile" for memo-profile contract)
    pub category: String,
    
    /// Operation type (must be "update_profile" for profile update)
    pub operation: String,
    
    /// User pubkey as string (must match the transaction signer)
    pub user_pubkey: String,
    
    /// Updated fields (all optional)
    pub username: Option<String>,
    pub image: Option<String>,
    pub about_me: Option<Option<String>>,
}

impl ProfileUpdateData {
    pub fn new(
        user_pubkey: String,
        username: Option<String>,
        image: Option<String>,
        about_me: Option<Option<String>>,
    ) -> Self {
        Self {
            version: 1, // PROFILE_UPDATE_DATA_VERSION
            category: "profile".to_string(), // EXPECTED_CATEGORY
            operation: "update_profile".to_string(), // EXPECTED_UPDATE_OPERATION
            user_pubkey,
            username,
            image,
            about_me,
        }
    }
    
    /// Validate the structure fields
    pub fn validate(&self, expected_user: Pubkey) -> Result<(), RpcError> {
        // basic validation logic (similar to the validation in the contract)
        if self.version != 1 {
            return Err(RpcError::Other(format!("Unsupported profile update data version: {} (expected: 1)", self.version)));
        }
        
        if self.category != "profile" {
            return Err(RpcError::Other(format!("Invalid category: '{}' (expected: 'profile')", self.category)));
        }
        
        if self.operation != "update_profile" {
            return Err(RpcError::Other(format!("Invalid operation: '{}' (expected: 'update_profile')", self.operation)));
        }
        
        let parsed_pubkey = Pubkey::from_str(&self.user_pubkey)
            .map_err(|_| RpcError::Other(format!("Invalid user_pubkey format: {}", self.user_pubkey)))?;
        
        if parsed_pubkey != expected_user {
            return Err(RpcError::Other(format!("User pubkey mismatch: {} vs expected {}", parsed_pubkey, expected_user)));
        }
        
        // validate field length
        if let Some(ref username) = self.username {
            if username.is_empty() || username.len() > 32 {
                return Err(RpcError::Other("Invalid username length".to_string()));
            }
        }
        
        if let Some(ref image) = self.image {
            if image.len() > 256 {
                return Err(RpcError::Other("Image too long".to_string()));
            }
        }
        
        if let Some(ref about_me_opt) = self.about_me {
            if let Some(ref about_me_text) = about_me_opt {
                if about_me_text.len() > 128 {
                    return Err(RpcError::Other("About me too long".to_string()));
                }
            }
        }
        
        Ok(())
    }
}

/// Memo-Profile contract configuration and constants
pub struct ProfileConfig;

impl ProfileConfig {
    /// Memo-profile program ID
    pub const MEMO_PROFILE_PROGRAM_ID: &'static str = "BwQTxuShrwJR15U6Utdfmfr4kZ18VT6FA1fcp58sT8US";
    
    /// PDA Seeds for profile contract
    pub const PROFILE_SEED: &'static [u8] = b"profile";
    
    /// Memo-burn program ID (updated to the correct address)
    pub const MEMO_BURN_PROGRAM_ID: &'static str = "FEjJ9KKJETocmaStfsFteFrktPchDLAVNTMeTvndoxaP";
    
    /// Token 2022 program ID
    pub const TOKEN_2022_PROGRAM_ID: &'static str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    
    /// Memo token mint address (updated to the correct address)
    pub const MEMO_TOKEN_MINT: &'static str = "HLCoc7wNDavNMfWWw2Bwd7U7A24cesuhBSNkxZgvZm1";
    
    /// get program ID
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MEMO_PROFILE_PROGRAM_ID)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-profile program ID: {}", e)))
    }
    
    /// get memo-burn program ID
    pub fn get_memo_burn_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MEMO_BURN_PROGRAM_ID)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-burn program ID: {}", e)))
    }
    
    /// get Token 2022 program ID
    pub fn get_token_2022_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::TOKEN_2022_PROGRAM_ID)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid token 2022 program ID: {}", e)))
    }
    
    /// get memo token mint
    pub fn get_memo_token_mint() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MEMO_TOKEN_MINT)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo token mint: {}", e)))
    }
    
    /// calculate user profile PDA
    pub fn get_profile_pda(user_pubkey: &Pubkey) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::PROFILE_SEED, user_pubkey.as_ref()],
            &program_id
        ))
    }
    
    /// get create_profile discriminator (using the same method as the test client)
    pub fn get_create_profile_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:create_profile");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }

    /// get update_profile discriminator
    pub fn get_update_profile_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:update_profile");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
}

// Profile RPC implementation
impl RpcConnection {
    /// create user profile
    pub async fn create_profile(
        &self,
        keypair: &Keypair,
        burn_amount: u64,
        username: &str,
        profile_image: &str,
        about_me: &str,
    ) -> Result<String, RpcError> {
        let about_me_opt = if about_me.is_empty() { None } else { Some(about_me.to_string()) };
        self.create_profile_full(keypair.to_bytes().as_ref(), username, profile_image, about_me_opt, burn_amount).await
    }

    /// full create user profile method
    async fn create_profile_full(
        &self,
        keypair_bytes: &[u8],
        username: &str,
        profile_image: &str,
        about_me: Option<String>,
        burn_amount: u64,
    ) -> Result<String, RpcError> {
        log::info!("Creating profile for user with burn amount: {} tokens", burn_amount);
        
        // validate minimum burn amount
        if burn_amount < 420 {
            return Err(RpcError::Other("Burn amount too small (minimum: 420 tokens)".to_string()));
        }
        
        // create keypair
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let user_pubkey = keypair.pubkey();
        
        // get necessary addresses
        let program_id = ProfileConfig::get_program_id()?;
        let memo_burn_program_id = ProfileConfig::get_memo_burn_program_id()?;
        let token_2022_program_id = ProfileConfig::get_token_2022_program_id()?;
        let memo_token_mint = ProfileConfig::get_memo_token_mint()?;
        
        // calculate PDA
        let (profile_pda, _) = ProfileConfig::get_profile_pda(&user_pubkey)?;
        
        // calculate user token account
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        // prepare profile creation data (using the correct structure)
        let profile_creation_data = ProfileCreationData::new(
            user_pubkey.to_string(),
            username.to_string(),
            profile_image.to_string(),
            about_me,
        );
        
        // validate data
        profile_creation_data.validate(user_pubkey)?;
        
        // create BurnMemo
        let burn_amount_units = burn_amount * 1_000_000; // convert to units
        let burn_memo = BurnMemo {
            version: 1, // BURN_MEMO_VERSION
            burn_amount: burn_amount_units,
            payload: profile_creation_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize profile data: {}", e)))?,
        };
        
        // serialize and encode to Base64
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        log::info!("Profile creation memo data size: {} bytes", memo_data_base64.len());
        
        // validate memo length
        if memo_data_base64.len() < 69 {
            return Err(RpcError::Other("Memo too short (min 69 chars)".to_string()));
        }
        if memo_data_base64.len() > 800 {
            return Err(RpcError::Other("Memo too long (max 800 chars)".to_string()));
        }
        
        // **create memo instruction (using the same method as the test client)**
        let memo_instruction = solana_sdk::instruction::Instruction {
            program_id: spl_memo::id(),
            accounts: vec![],
            data: memo_data_base64.into_bytes(),
        };
        
        // **create profile instruction (using the same method as the test client)**
        let mut instruction_data = ProfileConfig::get_create_profile_discriminator().to_vec();
        instruction_data.extend_from_slice(&burn_amount_units.to_le_bytes());
        
        let profile_instruction = solana_sdk::instruction::Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(user_pubkey, true),                      // user (signer)
                AccountMeta::new(profile_pda, false),                     // profile
                AccountMeta::new(memo_token_mint, false),                 // mint
                AccountMeta::new(user_token_account, false),              // user_token_account
                AccountMeta::new_readonly(token_2022_program_id, false),  // token_program
                AccountMeta::new_readonly(memo_burn_program_id, false),   // memo_burn_program
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system_program
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions
            ],
        );
        
        // get blockhash
        let blockhash_response: serde_json::Value = self.send_request("getLatestBlockhash", serde_json::json!([])).await?;
        let blockhash_str = blockhash_response["value"]["blockhash"].as_str()
            .ok_or_else(|| RpcError::Other("Failed to get blockhash".to_string()))?;
        let blockhash = solana_sdk::hash::Hash::from_str(blockhash_str)
            .map_err(|e| RpcError::Other(format!("Failed to parse blockhash: {}", e)))?;
        
        // **create transaction (using the same method as the test client)**
        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(400_000);
        
        let transaction = Transaction::new_signed_with_payer(
            &[
                // Index 0: Compute budget instruction (required for CU optimization)
                compute_budget_ix,
                // Index 1: SPL Memo instruction (REQUIRED at this position)
                memo_instruction,
                // Index 2: Profile creation instruction
                profile_instruction,
            ],
            Some(&user_pubkey),
            &[&keypair],
            blockhash,
        );
        
        // serialize transaction
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

    /// update user profile (simplified interface)
    pub async fn update_profile(
        &self,
        keypair: &Keypair,
        burn_amount: u64,
        username: Option<String>,
        image: Option<String>,
        about_me: Option<String>, // simplified to single layer Option
    ) -> Result<String, RpcError> {
        // convert to nested Option here
        let about_me_nested = match about_me {
            None => None,           // not update
            Some(text) if text.is_empty() => Some(None), // clear
            Some(text) => Some(Some(text)), // set new value
        };
        
        self.update_profile_full(
            keypair.to_bytes().as_ref(),
            username,
            image,
            about_me_nested, // pass nested Option
            burn_amount
        ).await
    }

    /// full update user profile method
    async fn update_profile_full(
        &self,
        keypair_bytes: &[u8],
        username: Option<String>,
        image: Option<String>,
        about_me: Option<Option<String>>,
        burn_amount: u64,
    ) -> Result<String, RpcError> {
        log::info!("Updating profile with burn amount: {} tokens", burn_amount);
        
        // validate minimum burn amount (contract requires update also needs 420 tokens)
        if burn_amount < 420 {
            return Err(RpcError::Other("Burn amount too small (minimum: 420 tokens)".to_string()));
        }
        
        // create keypair
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let user_pubkey = keypair.pubkey();
        
        // get necessary addresses
        let program_id = ProfileConfig::get_program_id()?;
        let memo_burn_program_id = ProfileConfig::get_memo_burn_program_id()?;
        let token_2022_program_id = ProfileConfig::get_token_2022_program_id()?;
        let memo_token_mint = ProfileConfig::get_memo_token_mint()?;
        
        // calculate PDA
        let (profile_pda, _) = ProfileConfig::get_profile_pda(&user_pubkey)?;
        
        // calculate user token account
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        // prepare profile update data
        let profile_update_data = ProfileUpdateData::new(
            user_pubkey.to_string(),
            username.clone(),
            image.clone(),
            about_me.clone(),
        );
        
        // validate data
        profile_update_data.validate(user_pubkey)?;
        
        // create BurnMemo
        let burn_amount_units = burn_amount * 1_000_000; // convert to units
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount: burn_amount_units,
            payload: profile_update_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize profile update data: {}", e)))?,
        };
        
        // serialize and encode to Base64
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        log::info!("Profile update memo data size: {} bytes", memo_data_base64.len());
        
        // **create memo instruction (following the contract requirements)**
        let memo_instruction = solana_sdk::instruction::Instruction {
            program_id: spl_memo::id(),
            accounts: vec![],
            data: memo_data_base64.into_bytes(),
        };
        
        // **create update_profile instruction (note: contract's update_profile requires parameters)**
        let mut instruction_data = ProfileConfig::get_update_profile_discriminator().to_vec();
        // according to the contract, update_profile instruction requires these parameters:
        // burn_amount: u64, username: Option<String>, image: Option<String>, about_me: Option<Option<String>>
        instruction_data.extend_from_slice(&burn_amount_units.to_le_bytes());
        
        // serialize Option<String> parameters (using Anchor/Borsh format)
        instruction_data.extend_from_slice(&username.try_to_vec().map_err(|e| RpcError::Other(format!("Failed to serialize username: {}", e)))?);
        instruction_data.extend_from_slice(&image.try_to_vec().map_err(|e| RpcError::Other(format!("Failed to serialize image: {}", e)))?);
        instruction_data.extend_from_slice(&about_me.try_to_vec().map_err(|e| RpcError::Other(format!("Failed to serialize about_me: {}", e)))?);
        
        let update_instruction = solana_sdk::instruction::Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(user_pubkey, true),                      // user (signer)
                AccountMeta::new(memo_token_mint, false),                 // mint
                AccountMeta::new(user_token_account, false),              // user_token_account
                AccountMeta::new(profile_pda, false),                     // profile
                AccountMeta::new_readonly(token_2022_program_id, false),  // token_program
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions
                AccountMeta::new_readonly(memo_burn_program_id, false),   // memo_burn_program
            ],
        );
        
        // get blockhash
        let blockhash_response: serde_json::Value = self.send_request("getLatestBlockhash", serde_json::json!([])).await?;
        let blockhash_str = blockhash_response["value"]["blockhash"].as_str()
            .ok_or_else(|| RpcError::Other("Failed to get blockhash".to_string()))?;
        let blockhash = solana_sdk::hash::Hash::from_str(blockhash_str)
            .map_err(|e| RpcError::Other(format!("Failed to parse blockhash: {}", e)))?;
        
        // **create transaction (following the contract requirements)**
        let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(400_000);
        
        let transaction = Transaction::new_signed_with_payer(
            &[
                // Index 0: Compute budget instruction
                compute_budget_ix,
                // Index 1: SPL Memo instruction (REQUIRED at this position)
                memo_instruction,
                // Index 2: Update profile instruction
                update_instruction,
            ],
            Some(&user_pubkey),
            &[&keypair],
            blockhash,
        );
        
        // serialize transaction
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

    /// delete user profile
    pub async fn delete_profile(&self, keypair: &Keypair) -> Result<String, RpcError> {
        let program_id = ProfileConfig::get_program_id()?;
        let target_pubkey = keypair.pubkey();
        
        // Calculate user profile PDA - should match contract seeds: [b"profile", user.key().as_ref()]
        let (user_profile_pda, bump) = Pubkey::find_program_address(
            &[b"profile", target_pubkey.as_ref()],
            &program_id
        );
        
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

        // Create delete_profile instruction data (discriminator)
        // For Anchor programs, the discriminator is the first 8 bytes of sha256("global:delete_profile")
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"global:delete_profile");
        let hash = hasher.finalize();
        let instruction_data = hash[..8].to_vec();

        // Create the instruction with correct accounts for new contract
        let instruction = Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(target_pubkey, true),      // user (signer, mutable)
                AccountMeta::new(user_profile_pda, false),  // profile (mutable, PDA)
            ],
        );

        // Create transaction
        let message = Message::new(
            &[instruction],
            Some(&target_pubkey), // fee payer
        );

        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = recent_blockhash.parse()
            .map_err(|_| RpcError::Other("Invalid blockhash format".to_string()))?;

        // Sign transaction
        transaction.sign(&[keypair], transaction.message.recent_blockhash);

        // Serialize and send transaction
        let serialized = bincode::serialize(&transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize transaction: {}", e)))?;

        let base64_tx = base64::encode(&serialized);

        let response: serde_json::Value = self.send_request(
            "sendTransaction",
            serde_json::json!([
                base64_tx,
                {
                    "skipPreflight": false,
                    "preflightCommitment": "confirmed",
                    "encoding": "base64",
                    "maxRetries": 3
                }
            ])
        ).await?;

        if let Some(error) = response.get("error") {
            return Err(RpcError::Other(format!("Transaction failed: {}", error)));
        }

        let signature = response.as_str()
            .ok_or_else(|| RpcError::Other("Invalid response format".to_string()))?;

        log::info!("Delete profile transaction sent: {}", signature);
        Ok(signature.to_string())
    }

    /// get user profile
    pub async fn get_profile(&self, user_pubkey: &str) -> Result<Option<UserProfile>, RpcError> {
        log::info!("Fetching profile for user: {}", user_pubkey);
        
        let pubkey = Pubkey::from_str(user_pubkey)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid pubkey: {}", e)))?;
        
        // calculate profile PDA
        let (profile_pda, _) = ProfileConfig::get_profile_pda(&pubkey)?;
        
        // get account info
        let account_info = self.get_account_info(&profile_pda.to_string(), Some("base64")).await?;
        
        // parse account data
        match self.parse_profile_account(&account_info) {
            Ok(profile) => Ok(Some(profile)),
            Err(RpcError::Other(msg)) if msg.contains("null") => Ok(None), // account not found
            Err(e) => Err(e),
        }
    }

    /// parse profile account data
    fn parse_profile_account(&self, account_data: &str) -> Result<UserProfile, RpcError> {
        let value: serde_json::Value = serde_json::from_str(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to parse account data: {}", e)))?;
        
        // check if account exists
        if value["value"].is_null() {
            return Err(RpcError::Other("Account not found".to_string()));
        }
        
        // get data field
        let data = value["value"]["data"][0].as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        // Base64 decode
        let decoded = base64::decode(data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // parse data (skip 8 bytes discriminator)
        if decoded.len() < 8 {
            return Err(RpcError::Other("Account data too short".to_string()));
        }
        
        let mut offset = 8; // skip discriminator
        
        // parse user pubkey (32 bytes)
        if decoded.len() < offset + 32 {
            return Err(RpcError::Other("Invalid profile data: missing user pubkey".to_string()));
        }
        let user_bytes = &decoded[offset..offset + 32];
        let user = Pubkey::new_from_array(user_bytes.try_into().unwrap()).to_string();
        offset += 32;
        
        // parse username (String)
        let (username, new_offset) = self.parse_string(&decoded, offset)?;
        offset = new_offset;
        
        // parse image (String)
        let (image, new_offset) = self.parse_string(&decoded, offset)?;
        offset = new_offset;
        
        // parse created_at (i64)
        if decoded.len() < offset + 8 {
            return Err(RpcError::Other("Invalid profile data: missing created_at".to_string()));
        }
        let created_at = i64::from_le_bytes(decoded[offset..offset + 8].try_into().unwrap());
        offset += 8;
        
        // parse last_updated (i64)
        if decoded.len() < offset + 8 {
            return Err(RpcError::Other("Invalid profile data: missing last_updated".to_string()));
        }
        let last_updated = i64::from_le_bytes(decoded[offset..offset + 8].try_into().unwrap());
        offset += 8;
        
        // parse about_me (Option<String>)
        let (about_me, new_offset) = self.parse_option_string(&decoded, offset)?;
        offset = new_offset;
        
        // parse bump (u8)
        if decoded.len() < offset + 1 {
            return Err(RpcError::Other("Invalid profile data: missing bump".to_string()));
        }
        let bump = decoded[offset];
        
        Ok(UserProfile {
            user,
            username,
            image,
            created_at,
            last_updated,
            about_me,
            bump,
        })
    }
    
    /// parse string (Borsh format: 4 bytes length + content)
    fn parse_string(&self, data: &[u8], offset: usize) -> Result<(String, usize), RpcError> {
        if data.len() < offset + 4 {
            return Err(RpcError::Other("Invalid string data: missing length".to_string()));
        }
        
        let len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let new_offset = offset + 4;
        
        if data.len() < new_offset + len {
            return Err(RpcError::Other("Invalid string data: truncated content".to_string()));
        }
        
        let string_bytes = &data[new_offset..new_offset + len];
        let string = String::from_utf8(string_bytes.to_vec())
            .map_err(|e| RpcError::Other(format!("Invalid UTF-8 string: {}", e)))?;
        
        Ok((string, new_offset + len))
    }
    
    /// parse optional string (Borsh format: 1 byte option flag + optional string data)
    fn parse_option_string(&self, data: &[u8], offset: usize) -> Result<(Option<String>, usize), RpcError> {
        if data.len() < offset + 1 {
            return Err(RpcError::Other("Invalid option data: missing option flag".to_string()));
        }
        
        let option_flag = data[offset];
        let mut new_offset = offset + 1;
        
        if option_flag == 0 {
            // None case
            Ok((None, new_offset))
        } else if option_flag == 1 {
            // Some case - parse the string
            let (string, final_offset) = self.parse_string(data, new_offset)?;
            Ok((Some(string), final_offset))
        } else {
            Err(RpcError::Other(format!("Invalid option flag: {}", option_flag)))
        }
    }

    /// get username by pubkey (lightweight interface for chat display)
    pub async fn get_username_by_pubkey(&self, user_pubkey: &str) -> Result<Option<String>, RpcError> {
        log::info!("Fetching username for user: {}", user_pubkey);
        
        match self.get_profile(user_pubkey).await? {
            Some(profile) => Ok(Some(profile.username)),
            None => Ok(None),
        }
    }

    /// batch get profiles for multiple users (optimized for chat loading)
    pub async fn get_profiles_batch(&self, user_pubkeys: &[&str]) -> Result<Vec<(String, Option<UserProfile>)>, RpcError> {
        log::info!("Batch fetching profiles for {} users", user_pubkeys.len());
        
        let mut results = Vec::new();
        
        // Note: This is a simple sequential implementation
        // For better performance, could be optimized with concurrent requests
        for pubkey in user_pubkeys {
            match self.get_profile(pubkey).await {
                Ok(profile) => results.push((pubkey.to_string(), profile)),
                Err(e) => {
                    log::warn!("Failed to fetch profile for {}: {}", pubkey, e);
                    results.push((pubkey.to_string(), None));
                }
            }
        }
        
        Ok(results)
    }

    /// batch get usernames for multiple users (lightweight for chat display)
    pub async fn get_usernames_batch(&self, user_pubkeys: &[&str]) -> Result<Vec<(String, Option<String>)>, RpcError> {
        log::info!("Batch fetching usernames for {} users", user_pubkeys.len());
        
        let mut results = Vec::new();
        
        for pubkey in user_pubkeys {
            match self.get_username_by_pubkey(pubkey).await {
                Ok(username) => results.push((pubkey.to_string(), username)),
                Err(e) => {
                    log::warn!("Failed to fetch username for {}: {}", pubkey, e);
                    results.push((pubkey.to_string(), None));
                }
            }
        }
        
        Ok(results)
    }

    /// get user display info (pubkey + username) for chat display
    pub async fn get_user_display_info(&self, user_pubkey: &str) -> Result<UserDisplayInfo, RpcError> {
        let username = self.get_username_by_pubkey(user_pubkey).await?;
        
        // Save the has_profile info before consuming username
        let has_profile = username.is_some();
        
        Ok(UserDisplayInfo {
            pubkey: user_pubkey.to_string(),
            username: username.unwrap_or_else(|| {
                // If no username found, show shortened pubkey as fallback
                if user_pubkey.len() > 8 {
                    format!("{}...{}", &user_pubkey[..4], &user_pubkey[user_pubkey.len()-4..])
                } else {
                    user_pubkey.to_string()
                }
            }),
            has_profile,
        })
    }

    /// batch get user display info for chat
    pub async fn get_user_display_info_batch(&self, user_pubkeys: &[&str]) -> Result<Vec<UserDisplayInfo>, RpcError> {
        log::info!("Batch fetching display info for {} users", user_pubkeys.len());
        
        let usernames = self.get_usernames_batch(user_pubkeys).await?;
        
        let mut results = Vec::new();
        for (pubkey, username) in usernames {
            // Save the has_profile info before consuming username
            let has_profile = username.is_some();
            
            results.push(UserDisplayInfo {
                pubkey: pubkey.clone(),
                username: username.unwrap_or_else(|| {
                    // If no username found, show shortened pubkey as fallback
                    if pubkey.len() > 8 {
                        format!("{}...{}", &pubkey[..4], &pubkey[pubkey.len()-4..])
                    } else {
                        pubkey.clone()
                    }
                }),
                has_profile,
            });
        }
        
        Ok(results)
    }
}

/// User display information for chat interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDisplayInfo {
    pub pubkey: String,
    pub username: String,
    pub has_profile: bool,
}

/// exported helper function for session use
pub fn parse_user_profile_new(account_data: &str) -> Result<UserProfile, RpcError> {
    // create a temporary RpcConnection instance to use the parsing method
    let rpc = RpcConnection::new();
    rpc.parse_profile_account(account_data)
}
