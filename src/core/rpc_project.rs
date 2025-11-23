use super::rpc_base::{
    RpcConnection, RpcError,
    get_token_2022_program_id, validate_memo_length_bytes
};
use super::network_config::get_program_ids;
use super::constants::*;
use serde::{Serialize, Deserialize};
use borsh::{BorshSerialize, BorshDeserialize};
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    transaction::Transaction,
    message::Message,
    compute_budget::ComputeBudgetInstruction,
};
use spl_memo;
use sha2::{Sha256, Digest};
use base64;
use bincode;
use spl_associated_token_account;

/// Project creation data version
pub const PROJECT_CREATION_DATA_VERSION: u8 = 1;

/// Project update data version
pub const PROJECT_UPDATE_DATA_VERSION: u8 = 1;

/// Memo-Project contract configuration and constants
pub struct ProjectConfig;

impl ProjectConfig {
    // Note: Program IDs and token mint are now retrieved dynamically from network configuration
    
    /// PDA Seeds for project contract
    pub const GLOBAL_COUNTER_SEED: &'static [u8] = b"global_counter";
    pub const PROJECT_SEED: &'static [u8] = b"project";
    pub const BURN_LEADERBOARD_SEED: &'static [u8] = b"burn_leaderboard";
    
    /// Minimum burn amount required to create a project (42,069 tokens = 42,069,000,000 lamports)
    pub const MIN_PROJECT_CREATION_BURN_AMOUNT: u64 = 42_069_000_000;
    
    /// Minimum burn amount required to update a project (42,069 tokens = 42,069,000,000 lamports)
    pub const MIN_PROJECT_UPDATE_BURN_AMOUNT: u64 = 42_069_000_000;
    
    /// Minimum burn amount for burning to project (420 tokens = 420,000,000 lamports)
    pub const MIN_PROJECT_BURN_AMOUNT: u64 = 420_000_000;
    
    // Note: Memo validation limits, payload length, and compute unit config
    // are now directly used from the constants module to avoid duplication
    
    /// Helper functions
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.project_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-project program ID: {}", e)))
    }
    
    /// Get memo-burn program ID
    pub fn get_memo_burn_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.burn_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-burn program ID: {}", e)))
    }
    
    // Note: get_token_2022_program_id() and validate_memo_length() 
    // are now provided by rpc_base module to avoid duplication
    
    /// Get memo token mint
    pub fn get_memo_token_mint() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.token_mint)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo token mint: {}", e)))
    }
    
    /// Calculate global counter PDA
    pub fn get_global_counter_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::GLOBAL_COUNTER_SEED],
            &program_id
        ))
    }
    
    /// Calculate project PDA for a specific project ID
    pub fn get_project_pda(project_id: u64) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::PROJECT_SEED, &project_id.to_le_bytes()],
            &program_id
        ))
    }
    
    /// Calculate burn leaderboard PDA
    pub fn get_burn_leaderboard_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::BURN_LEADERBOARD_SEED],
            &program_id
        ))
    }
    
    /// Get create_project instruction discriminator
    pub fn get_create_project_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:create_project");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Get update_project instruction discriminator
    pub fn get_update_project_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:update_project");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Get burn_for_project instruction discriminator
    pub fn get_burn_for_project_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:burn_for_project");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    // Note: validate_memo_length() is now provided by rpc_base module
    
    /// Calculate user global burn stats PDA (from memo-burn program)
    pub fn get_user_global_burn_stats_pda(user_pubkey: &Pubkey) -> Result<(Pubkey, u8), RpcError> {
        let memo_burn_program_id = Self::get_memo_burn_program_id()?;
        Ok(Pubkey::find_program_address(
            &[b"user_global_burn_stats", user_pubkey.as_ref()],
            &memo_burn_program_id
        ))
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

/// Project creation data structure (stored in BurnMemo.payload)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ProjectCreationData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "project" for memo-project contract)
    pub category: String,
    
    /// Operation type (must be "create_project" for project creation)
    pub operation: String,
    
    /// Project ID (must match expected_project_id)
    pub project_id: u64,
    
    /// Project name (required, 1-64 characters)
    pub name: String,
    
    /// Project description (optional, max 256 characters)
    pub description: String,
    
    /// Project image info (optional, max 256 characters)
    pub image: String,
    
    /// Project website URL (optional, max 128 characters)
    pub website: String,
    
    /// Tags (optional, max 4 tags, each max 32 characters)
    pub tags: Vec<String>,
}

impl ProjectCreationData {
    /// Create new project creation data
    pub fn new(
        project_id: u64,
        name: String,
        description: String,
        image: String,
        website: String,
        tags: Vec<String>,
    ) -> Self {
        Self {
            version: PROJECT_CREATION_DATA_VERSION,
            category: "project".to_string(),
            operation: "create_project".to_string(),
            project_id,
            name,
            description,
            image,
            website,
            tags,
        }
    }
    
    /// Validate the project creation data
    pub fn validate(&self, expected_project_id: u64) -> Result<(), RpcError> {
        // Validate version
        if self.version != PROJECT_CREATION_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported project creation data version: {} (expected: {})", 
                self.version, PROJECT_CREATION_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "project" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'project')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "create_project" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'create_project')", self.operation
            )));
        }
        
        // Validate project ID
        if self.project_id != expected_project_id {
            return Err(RpcError::InvalidParameter(format!(
                "Project ID mismatch: data contains {}, expected {}", 
                self.project_id, expected_project_id
            )));
        }
        
        // Validate name (required, 1-64 characters)
        if self.name.is_empty() || self.name.len() > 64 {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid project name: '{}' (must be 1-64 characters)", self.name
            )));
        }
        
        // Validate description (optional, max 256 characters)
        if self.description.len() > 256 {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid project description: {} characters (max: 256)", self.description.len()
            )));
        }
        
        // Validate image (optional, max 256 characters)
        if self.image.len() > 256 {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid project image: {} characters (max: 256)", self.image.len()
            )));
        }
        
        // Validate website (optional, max 128 characters)
        if self.website.len() > 128 {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid project website: {} characters (max: 128)", self.website.len()
            )));
        }
        
        // Validate tags (optional, max 4 tags, each max 32 characters)
        if self.tags.len() > 4 {
            return Err(RpcError::InvalidParameter(format!(
                "Too many tags: {} (max: 4)", self.tags.len()
            )));
        }
        
        for (i, tag) in self.tags.iter().enumerate() {
            if tag.is_empty() || tag.len() > 32 {
                return Err(RpcError::InvalidParameter(format!(
                    "Invalid tag {}: '{}' (must be 1-32 characters)", i, tag
                )));
            }
        }
        
        Ok(())
    }
    
    /// Calculate the final memo size (Borsh + Base64) for this project creation data
    pub fn calculate_final_memo_size(&self, burn_amount: u64) -> Result<usize, String> {
        // Serialize ProjectCreationData to Borsh
        let payload_bytes = self.try_to_vec()
            .map_err(|e| format!("Failed to serialize ProjectCreationData: {}", e))?;
        
        // Create BurnMemo with the payload
        let burn_memo = BurnMemo {
            version: BURN_MEMO_VERSION,
            burn_amount,
            payload: payload_bytes,
        };
        
        // Serialize BurnMemo to Borsh
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| format!("Failed to serialize BurnMemo: {}", e))?;
        
        // Encode to Base64 (this is what actually gets sent)
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        Ok(memo_data_base64.len())
    }
    
    /// Check if the final memo size is within valid limits (69-800 bytes)
    pub fn is_valid_memo_size(&self, burn_amount: u64) -> Result<bool, String> {
        let final_size = self.calculate_final_memo_size(burn_amount)?;
        Ok(final_size >= MIN_MEMO_LENGTH && final_size <= MAX_MEMO_LENGTH)
    }
}

/// Project update data structure (stored in BurnMemo.payload)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ProjectUpdateData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "project" for memo-project contract)
    pub category: String,
    
    /// Operation type (must be "update_project" for project update)
    pub operation: String,
    
    /// Project ID (must match the target project)
    pub project_id: u64,
    
    /// Updated fields (all optional)
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub website: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl ProjectUpdateData {
    /// Create new project update data
    pub fn new(
        project_id: u64,
        name: Option<String>,
        description: Option<String>,
        image: Option<String>,
        website: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Self {
            version: PROJECT_UPDATE_DATA_VERSION,
            category: "project".to_string(),
            operation: "update_project".to_string(),
            project_id,
            name,
            description,
            image,
            website,
            tags,
        }
    }
    
    /// Validate the project update data
    pub fn validate(&self, expected_project_id: u64) -> Result<(), RpcError> {
        // Validate version
        if self.version != PROJECT_UPDATE_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported project update data version: {} (expected: {})", 
                self.version, PROJECT_UPDATE_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "project" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'project')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "update_project" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'update_project')", self.operation
            )));
        }
        
        // Validate project ID
        if self.project_id != expected_project_id {
            return Err(RpcError::InvalidParameter(format!(
                "Project ID mismatch: data contains {}, expected {}", 
                self.project_id, expected_project_id
            )));
        }
        
        // Validate optional fields
        if let Some(ref name) = self.name {
            if name.is_empty() || name.len() > 64 {
                return Err(RpcError::InvalidParameter(format!(
                    "Invalid project name: '{}' (must be 1-64 characters)", name
                )));
            }
        }
        
        if let Some(ref description) = self.description {
            if description.len() > 256 {
                return Err(RpcError::InvalidParameter(format!(
                    "Invalid project description: {} characters (max: 256)", description.len()
                )));
            }
        }
        
        if let Some(ref image) = self.image {
            if image.len() > 256 {
                return Err(RpcError::InvalidParameter(format!(
                    "Invalid project image: {} characters (max: 256)", image.len()
                )));
            }
        }
        
        if let Some(ref website) = self.website {
            if website.len() > 128 {
                return Err(RpcError::InvalidParameter(format!(
                    "Invalid project website: {} characters (max: 128)", website.len()
                )));
            }
        }
        
        if let Some(ref tags) = self.tags {
            if tags.len() > 4 {
                return Err(RpcError::InvalidParameter(format!(
                    "Too many tags: {} (max: 4)", tags.len()
                )));
            }
            
            for (i, tag) in tags.iter().enumerate() {
                if tag.is_empty() || tag.len() > 32 {
                    return Err(RpcError::InvalidParameter(format!(
                        "Invalid tag {}: '{}' (must be 1-32 characters)", i, tag
                    )));
                }
            }
        }
        
        Ok(())
    }
}

/// Project burn data structure (stored in BurnMemo.payload for burn_for_project)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ProjectBurnData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "project" for memo-project contract)
    pub category: String,
    
    /// Operation type (must be "burn_for_project" for burning tokens)
    pub operation: String,
    
    /// Project ID (must match the target project)
    pub project_id: u64,
    
    /// Burner pubkey as string (must match the transaction signer)
    pub burner: String,
    
    /// Burn message (optional, max 696 characters)
    pub message: String,
}

impl ProjectBurnData {
    /// Create new project burn data
    pub fn new(
        project_id: u64,
        burner: String,
        message: String,
    ) -> Self {
        Self {
            version: PROJECT_CREATION_DATA_VERSION,
            category: "project".to_string(),
            operation: "burn_for_project".to_string(),
            project_id,
            burner,
            message,
        }
    }
    
    /// Validate the project burn data
    pub fn validate(&self, expected_project_id: u64, expected_burner: &str) -> Result<(), RpcError> {
        // Validate version
        if self.version != PROJECT_CREATION_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported project burn data version: {} (expected: {})", 
                self.version, PROJECT_CREATION_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "project" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'project')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "burn_for_project" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'burn_for_project')", self.operation
            )));
        }
        
        // Validate project ID
        if self.project_id != expected_project_id {
            return Err(RpcError::InvalidParameter(format!(
                "Project ID mismatch: data contains {}, expected {}", 
                self.project_id, expected_project_id
            )));
        }
        
        // Validate burner
        if self.burner != expected_burner {
            return Err(RpcError::InvalidParameter(format!(
                "Burner mismatch: data contains {}, expected {}", 
                self.burner, expected_burner
            )));
        }
        
        // Validate message (optional, max 696 characters)
        if self.message.len() > 696 {
            return Err(RpcError::InvalidParameter(format!(
                "Burn message too long: {} characters (max: 696)", self.message.len()
            )));
        }
        
        Ok(())
    }
}

/// Represents global project statistics from the memo-project contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGlobalStatistics {
    pub total_projects: u64,
}

/// Represents a project's information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub project_id: u64,
    pub creator: String,  // Base58 encoded pubkey
    pub created_at: i64,
    pub last_updated: i64,
    pub name: String,
    pub description: String,
    pub image: String,
    pub website: String,
    pub tags: Vec<String>,
    pub memo_count: u64,
    pub burned_amount: u64,
    pub last_memo_time: i64,
    pub bump: u8,
}

/// Summary statistics for all projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatistics {
    pub total_projects: u64,
    pub valid_projects: u64,
    pub total_memos: u64,
    pub total_burned_tokens: u64,
    pub projects: Vec<ProjectInfo>,
}

/// Project leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectLeaderboardEntry {
    pub project_id: u64,
    pub burned_amount: u64,
    pub rank: u8, // rank (1-100)
}

/// Project burn leaderboard response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectBurnLeaderboardResponse {
    pub entries: Vec<ProjectLeaderboardEntry>,
    pub total_burned_tokens: u64, // total burned amount of all leaderboard entries
}

/// Represents a single burn action to a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBurnMessage {
    pub signature: String,      // Transaction signature
    pub burner: String,         // Burner's public key
    pub message: String,        // The burn message
    pub timestamp: i64,         // Block time
    pub slot: u64,             // Slot number
    pub burn_amount: u64,      // Amount burned (in lamports)
}

/// Response containing burn messages for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBurnMessagesResponse {
    pub project_id: u64,
    pub messages: Vec<ProjectBurnMessage>,
    pub total_found: usize,
    pub has_more: bool,        // Indicates if there are more messages available
}

/// Parse Base64+Borsh-formatted memo data to extract project burn message
fn parse_borsh_project_burn_message(memo_data: &[u8]) -> Option<(String, String, u64)> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to BurnMemo
    match BurnMemo::try_from_slice(&borsh_bytes) {
        Ok(burn_memo) => {
            // Deserialize payload to ProjectBurnData
            match ProjectBurnData::try_from_slice(&burn_memo.payload) {
                Ok(burn_data) => {
                    // Validate category and operation
                    if burn_data.category == "project" && burn_data.operation == "burn_for_project" {
                        Some((burn_data.burner, burn_data.message, burn_memo.burn_amount))
                    } else {
                        None
                    }
                },
                Err(_) => None
            }
        },
        Err(_) => None
    }
}

/// Parse memo data to determine message type for projects
fn parse_project_memo_data(memo_data: &[u8]) -> Option<(String, String, String, Option<u64>)> {
    // Try parsing as project burn message
    if let Some((burner, message, burn_amount)) = parse_borsh_project_burn_message(memo_data) {
        return Some((burner, message, "burn".to_string(), Some(burn_amount)));
    }
    
    None
}

impl RpcConnection {
    /// Build an unsigned transaction to create a project
    pub async fn build_create_project_transaction(
        &self,
        user_pubkey: &Pubkey,
        name: &str,
        description: &str,
        image: &str,
        website: &str,
        tags: Vec<String>,
        burn_amount: u64,
    ) -> Result<(Transaction, u64), RpcError> {
        // Basic parameter validation
        if name.is_empty() || name.len() > 64 {
            return Err(RpcError::InvalidParameter(format!("Project name must be 1-64 characters, got {}", name.len())));
        }
        if description.len() > 256 {
            return Err(RpcError::InvalidParameter(format!("Project description must be at most 256 characters, got {}", description.len())));
        }
        if image.len() > 256 {
            return Err(RpcError::InvalidParameter(format!("Project image must be at most 256 characters, got {}", image.len())));
        }
        if website.len() > 128 {
            return Err(RpcError::InvalidParameter(format!("Project website must be at most 128 characters, got {}", website.len())));
        }
        if tags.len() > 4 {
            return Err(RpcError::InvalidParameter(format!("Too many tags: {} (max: 4)", tags.len())));
        }
        for (i, tag) in tags.iter().enumerate() {
            if tag.is_empty() || tag.len() > 32 {
                return Err(RpcError::InvalidParameter(format!("Invalid tag {}: '{}' (must be 1-32 characters)", i, tag)));
            }
        }
        if burn_amount < ProjectConfig::MIN_PROJECT_CREATION_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!("Burn amount must be at least {} MEMO tokens", ProjectConfig::MIN_PROJECT_CREATION_BURN_AMOUNT / 1_000_000)));
        }
        if burn_amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens".to_string()));
        }
        
        log::info!("Building create project transaction '{}': {} tokens", name, burn_amount / 1_000_000);
        
        // Get next project_id
        let global_stats = self.get_project_global_statistics().await?;
        let expected_project_id = global_stats.total_projects;
        
        let project_program_id = ProjectConfig::get_program_id()?;
        let memo_token_mint = ProjectConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_burn_program_id = ProjectConfig::get_memo_burn_program_id()?;
        
        let (global_counter_pda, _) = ProjectConfig::get_global_counter_pda()?;
        let (project_pda, _) = ProjectConfig::get_project_pda(expected_project_id)?;
        let (burn_leaderboard_pda, _) = ProjectConfig::get_burn_leaderboard_pda()?;
        let (user_global_burn_stats_pda, _) = ProjectConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey, &memo_token_mint, &token_2022_program_id,
        );
        
        let project_creation_data = ProjectCreationData::new(
            expected_project_id, name.to_string(), description.to_string(), 
            image.to_string(), website.to_string(), tags,
        );
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload: project_creation_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize project data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        validate_memo_length_bytes(memo_data_base64.as_bytes())?;
        
        // Build base instructions (without compute budget)
        let mut base_instructions = vec![];
        
        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(memo_data_base64.as_bytes(), &[user_pubkey]));
        
        // Create project instruction
        let mut instruction_data = ProjectConfig::get_create_project_discriminator().to_vec();
        instruction_data.extend_from_slice(&expected_project_id.to_le_bytes());
        instruction_data.extend_from_slice(&burn_amount.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            project_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(global_counter_pda, false),
                AccountMeta::new(project_pda, false),
                AccountMeta::new(burn_leaderboard_pda, false),
                AccountMeta::new(memo_token_mint, false),
                AccountMeta::new(user_token_account, false),
                AccountMeta::new(user_global_burn_stats_pda, false),
                AccountMeta::new_readonly(token_2022_program_id, false),
                AccountMeta::new_readonly(memo_burn_program_id, false),
                AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
            ],
        ));
        
        let blockhash = self.get_latest_blockhash().await?;
        
        // Simulate with dummy compute budget instructions for accurate CU estimation
        // Note: Keep same instruction order as final transaction (memo at index 0)
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000u32));
        
        // If user has set a price, include it in simulation to match final transaction
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
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
        
        log::info!("Simulating create project transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Create project simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction: memo at index 0, then other instructions, compute budget at end
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
        
        Ok((transaction, expected_project_id))
    }

    /// Build an unsigned transaction to update a project
    pub async fn build_update_project_transaction(
        &self,
        user_pubkey: &Pubkey,
        project_id: u64,
        name: Option<String>,
        description: Option<String>,
        image: Option<String>,
        website: Option<String>,
        tags: Option<Vec<String>>,
        burn_amount: u64,
    ) -> Result<Transaction, RpcError> {
        // Basic parameter validation
        if let Some(ref n) = name {
            if n.is_empty() || n.len() > 64 {
                return Err(RpcError::InvalidParameter(format!("Project name must be 1-64 characters, got {}", n.len())));
            }
        }
        if let Some(ref d) = description {
            if d.len() > 256 {
                return Err(RpcError::InvalidParameter(format!("Project description must be at most 256 characters, got {}", d.len())));
            }
        }
        if let Some(ref i) = image {
            if i.len() > 256 {
                return Err(RpcError::InvalidParameter(format!("Project image must be at most 256 characters, got {}", i.len())));
            }
        }
        if let Some(ref w) = website {
            if w.len() > 128 {
                return Err(RpcError::InvalidParameter(format!("Project website must be at most 128 characters, got {}", w.len())));
            }
        }
        if let Some(ref t) = tags {
            if t.len() > 4 {
                return Err(RpcError::InvalidParameter(format!("Too many tags: {} (max: 4)", t.len())));
            }
            for (i, tag) in t.iter().enumerate() {
                if tag.is_empty() || tag.len() > 32 {
                    return Err(RpcError::InvalidParameter(format!("Invalid tag {}: '{}' (must be 1-32 characters)", i, tag)));
                }
            }
        }
        if burn_amount < ProjectConfig::MIN_PROJECT_UPDATE_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!("Burn amount must be at least {} MEMO tokens", ProjectConfig::MIN_PROJECT_UPDATE_BURN_AMOUNT / 1_000_000)));
        }
        if burn_amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens".to_string()));
        }
        
        log::info!("Building update project transaction for project {}: {} tokens", project_id, burn_amount / 1_000_000);
        
        let project_program_id = ProjectConfig::get_program_id()?;
        let memo_token_mint = ProjectConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_burn_program_id = ProjectConfig::get_memo_burn_program_id()?;
        
        let (project_pda, _) = ProjectConfig::get_project_pda(project_id)?;
        let (burn_leaderboard_pda, _) = ProjectConfig::get_burn_leaderboard_pda()?;
        let (user_global_burn_stats_pda, _) = ProjectConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey, &memo_token_mint, &token_2022_program_id,
        );
        
        let project_update_data = ProjectUpdateData::new(project_id, name, description, image, website, tags);
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload: project_update_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize project data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        validate_memo_length_bytes(memo_data_base64.as_bytes())?;
        
        // Build base instructions (without compute budget)
        let mut base_instructions = vec![];
        
        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(memo_data_base64.as_bytes(), &[user_pubkey]));
        
        // Update project instruction
        let mut instruction_data = ProjectConfig::get_update_project_discriminator().to_vec();
        instruction_data.extend_from_slice(&project_id.to_le_bytes());
        instruction_data.extend_from_slice(&burn_amount.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            project_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(project_pda, false),
                AccountMeta::new(burn_leaderboard_pda, false),
                AccountMeta::new(memo_token_mint, false),
                AccountMeta::new(user_token_account, false),
                AccountMeta::new(user_global_burn_stats_pda, false),
                AccountMeta::new_readonly(token_2022_program_id, false),
                AccountMeta::new_readonly(memo_burn_program_id, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
            ],
        ));
        
        let blockhash = self.get_latest_blockhash().await?;
        
        // Simulate with dummy compute budget instructions for accurate CU estimation
        // Note: Keep same instruction order as final transaction (memo at index 0)
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000u32));
        
        // If user has set a price, include it in simulation to match final transaction
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
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
        
        log::info!("Simulating update project transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Update project simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction: memo at index 0, then other instructions, compute budget at end
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

    /// Build an unsigned transaction to burn tokens for a project
    pub async fn build_burn_tokens_for_project_transaction(
        &self,
        user_pubkey: &Pubkey,
        project_id: u64,
        amount: u64,
        message: &str,
    ) -> Result<Transaction, RpcError> {
        // Validate amount
        if amount < ProjectConfig::MIN_PROJECT_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!(
                "Burn amount must be at least {} MEMO tokens", 
                ProjectConfig::MIN_PROJECT_BURN_AMOUNT / 1_000_000
            )));
        }
        
        if amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter(
                "Burn amount must be a whole number of tokens".to_string()
            ));
        }
        
        // Validate message length
        if message.len() > 696 {
            return Err(RpcError::InvalidParameter(
                "Burn message too long (max 696 characters)".to_string()
            ));
        }
        
        log::info!("Building burn tokens for project transaction: {} tokens for project {}", 
                  amount / 1_000_000, project_id);
        
        let project_program_id = ProjectConfig::get_program_id()?;
        let memo_token_mint = ProjectConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_burn_program_id = ProjectConfig::get_memo_burn_program_id()?;
        
        let (project_pda, _) = ProjectConfig::get_project_pda(project_id)?;
        let (burn_leaderboard_pda, _) = ProjectConfig::get_burn_leaderboard_pda()?;
        let (user_global_burn_stats_pda, _) = ProjectConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        let burn_data = ProjectBurnData::new(project_id, user_pubkey.to_string(), message.to_string());
        burn_data.validate(project_id, &user_pubkey.to_string())?;
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount: amount,
            payload: burn_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize burn data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        validate_memo_length_bytes(memo_data_base64.as_bytes())?;
        
        // Build base instructions (without compute budget)
        let mut base_instructions = vec![];
        
        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(memo_data_base64.as_bytes(), &[user_pubkey]));
        
        // Burn instruction
        let mut instruction_data = ProjectConfig::get_burn_for_project_discriminator().to_vec();
        instruction_data.extend_from_slice(&project_id.to_le_bytes());
        instruction_data.extend_from_slice(&amount.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            project_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(project_pda, false),
                AccountMeta::new(burn_leaderboard_pda, false),
                AccountMeta::new(memo_token_mint, false),
                AccountMeta::new(user_token_account, false),
                AccountMeta::new(user_global_burn_stats_pda, false),
                AccountMeta::new_readonly(token_2022_program_id, false),
                AccountMeta::new_readonly(memo_burn_program_id, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
            ],
        ));
        
        let blockhash = self.get_latest_blockhash().await?;
        
        // Simulate with dummy compute budget instructions for accurate CU estimation
        // Note: Keep same instruction order as final transaction (memo at index 0)
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000u32));
        
        // If user has set a price, include it in simulation to match final transaction
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
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
        
        log::info!("Simulating burn tokens for project transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Burn tokens for project simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction: memo at index 0, then other instructions, compute budget at end
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

    /// Get global project statistics from the memo-project contract
    /// 
    /// # Returns
    /// Global statistics including total number of projects created
    pub async fn get_project_global_statistics(&self) -> Result<ProjectGlobalStatistics, RpcError> {
        let (global_counter_pda, _) = ProjectConfig::get_global_counter_pda()?;
        
        log::info!("Fetching global project statistics from PDA: {}", global_counter_pda);
        
        let account_info = self.get_account_info(&global_counter_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(
                "Global project counter not found. Please initialize the memo-project system first.".to_string()
            ));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // Verify the account is owned by memo-project program
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account owner".to_string()))?;
        
        let expected_program_id = ProjectConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-project program. Expected: {}, Got: {}", 
                expected_program_id, owner
            )));
        }
        
        // Parse total projects count (skip 8-byte discriminator, read next 8 bytes)
        if data.len() < 16 {
            return Err(RpcError::Other("Invalid account data size".to_string()));
        }
        
        let total_projects_bytes = &data[8..16];
        let total_projects = u64::from_le_bytes(
            total_projects_bytes.try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse total projects: {:?}", e)))?
        );
        
        log::info!("Global project statistics: {} total projects", total_projects);
        
        Ok(ProjectGlobalStatistics { total_projects })
    }
    
    /// Get information for a specific project
    /// 
    /// # Parameters
    /// * `project_id` - The ID of the project to fetch
    /// 
    /// # Returns
    /// Project information if it exists
    pub async fn get_project_info(&self, project_id: u64) -> Result<ProjectInfo, RpcError> {
        let (project_pda, _) = ProjectConfig::get_project_pda(project_id)?;
        
        log::info!("Fetching project {} info from PDA: {}", project_id, project_pda);
        
        let account_info = self.get_account_info(&project_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(format!("Project {} not found", project_id)));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // Verify the account is owned by memo-project program
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account owner".to_string()))?;
        
        let expected_program_id = ProjectConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-project program. Expected: {}, Got: {}", 
                expected_program_id, owner
            )));
        }
        
        // Parse project data
        self.parse_project_data(&data)
    }
    
    /// Parse Project account data according to the contract's data structure
    fn parse_project_data(&self, data: &[u8]) -> Result<ProjectInfo, RpcError> {
        if data.len() < 8 {
            return Err(RpcError::Other("Data too short for discriminator".to_string()));
        }
        
        let mut offset = 8; // Skip discriminator
        
        // Read project_id (u64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for project_id".to_string()));
        }
        let project_id = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse project_id: {:?}", e)))?
        );
        offset += 8;
        
        // Read creator (Pubkey = 32 bytes)
        if data.len() < offset + 32 {
            return Err(RpcError::Other("Data too short for creator".to_string()));
        }
        let creator_bytes: [u8; 32] = data[offset..offset + 32].try_into()
            .map_err(|e| RpcError::Other(format!("Failed to parse creator bytes: {:?}", e)))?;
        let creator = Pubkey::from(creator_bytes).to_string();
        offset += 32;
        
        // Read created_at (i64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for created_at".to_string()));
        }
        let created_at = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse created_at: {:?}", e)))?
        );
        offset += 8;
        
        // Read last_updated (i64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for last_updated".to_string()));
        }
        let last_updated = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse last_updated: {:?}", e)))?
        );
        offset += 8;
        
        // Read name (String)
        let (name, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read description (String)
        let (description, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read image (String)
        let (image, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read website (String)
        let (website, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read tags (Vec<String>)
        let (tags, new_offset) = self.read_string_vec_from_data(data, offset)?;
        offset = new_offset;
        
        // Read memo_count (u64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for memo_count".to_string()));
        }
        let memo_count = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse memo_count: {:?}", e)))?
        );
        offset += 8;
        
        // Read burned_amount (u64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for burned_amount".to_string()));
        }
        let burned_amount = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse burned_amount: {:?}", e)))?
        );
        offset += 8;
        
        // Read last_memo_time (i64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for last_memo_time".to_string()));
        }
        let last_memo_time = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse last_memo_time: {:?}", e)))?
        );
        offset += 8;
        
        // Read bump (u8)
        if data.len() < offset + 1 {
            return Err(RpcError::Other("Data too short for bump".to_string()));
        }
        let bump = data[offset];
        
        Ok(ProjectInfo {
            project_id,
            creator,
            created_at,
            last_updated,
            name,
            description,
            image,
            website,
            tags,
            memo_count,
            burned_amount,
            last_memo_time,
            bump,
        })
    }
    
    /// Get comprehensive statistics for all projects
    /// 
    /// # Returns
    /// Complete statistics including all project information
    pub async fn get_all_project_statistics(&self) -> Result<ProjectStatistics, RpcError> {
        log::info!("Fetching comprehensive project statistics...");
        
        // First get global statistics
        let global_stats = self.get_project_global_statistics().await?;
        let total_projects = global_stats.total_projects;
        
        if total_projects == 0 {
            log::info!("No projects found");
            return Ok(ProjectStatistics {
                total_projects: 0,
                valid_projects: 0,
                total_memos: 0,
                total_burned_tokens: 0,
                projects: vec![],
            });
        }
        
        let mut valid_projects = 0;
        let mut total_memos = 0;
        let mut total_burned_tokens: u64 = 0;
        let mut projects = Vec::new();
        
        // Iterate through all projects
        for project_id in 0..total_projects {
            match self.get_project_info(project_id).await {
                Ok(project_info) => {
                    valid_projects += 1;
                    total_memos += project_info.memo_count;
                    total_burned_tokens += project_info.burned_amount;
                    projects.push(project_info);
                    
                    log::info!("Successfully fetched project {}", project_id);
                },
                Err(e) => {
                    log::warn!("Failed to fetch project {}: {}", project_id, e);
                }
            }
        }
        
        log::info!("Project statistics summary: {}/{} valid projects, {} total memos, {} total burned tokens", 
                  valid_projects, total_projects, total_memos, total_burned_tokens);
        
        Ok(ProjectStatistics {
            total_projects,
            valid_projects,
            total_memos,
            total_burned_tokens,
            projects,
        })
    }

    /// Get burn leaderboard for projects
    /// 
    /// # Returns
    /// Project burn leaderboard data, including the top 100 projects
    pub async fn get_project_burn_leaderboard(&self) -> Result<ProjectBurnLeaderboardResponse, RpcError> {
        let (burn_leaderboard_pda, _) = ProjectConfig::get_burn_leaderboard_pda()?;
        
        log::info!("Fetching project burn leaderboard from PDA: {}", burn_leaderboard_pda);
        
        let account_info = self.get_account_info(&burn_leaderboard_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse leaderboard account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(
                "Project burn leaderboard not found. Please initialize the memo-project system first.".to_string()
            ));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get leaderboard account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode leaderboard account data: {}", e)))?;
        
        // Verify account owner
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get leaderboard account owner".to_string()))?;
        
        let expected_program_id = ProjectConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-project program. Expected: {}, Got: {}", 
                expected_program_id, owner
            )));
        }
        
        // Parse leaderboard data
        if data.len() < 9 {
            return Err(RpcError::Other("Invalid leaderboard data size".to_string()));
        }
        
        let mut offset = 8; // Skip discriminator
        
        // Read entries vector length (u32) - directly after discriminator
        if data.len() < offset + 4 {
            return Err(RpcError::Other("Data too short for entries vector length".to_string()));
        }
        let entries_len = u32::from_le_bytes(
            data[offset..offset + 4].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse entries length: {:?}", e)))?
        ) as usize;
        offset += 4;
        
        let mut entries = Vec::new();
        let mut total_burned_tokens: u64 = 0;
        
        // Read each entry (project_id: u64, burned_amount: u64)
        for i in 0..entries_len {
            if data.len() < offset + 16 {
                log::warn!("Data too short for entry {}, stopping parse", i);
                break;
            }
            
            let project_id = u64::from_le_bytes(
                data[offset..offset + 8].try_into()
                    .map_err(|e| RpcError::Other(format!("Failed to parse project_id: {:?}", e)))?
            );
            offset += 8;
            
            let burned_amount = u64::from_le_bytes(
                data[offset..offset + 8].try_into()
                    .map_err(|e| RpcError::Other(format!("Failed to parse burned_amount: {:?}", e)))?
            );
            offset += 8;
            
            total_burned_tokens = total_burned_tokens.saturating_add(burned_amount);
            
            entries.push(ProjectLeaderboardEntry {
                project_id,
                burned_amount,
                rank: (i + 1) as u8, // rank starts from 1
            });
        }
        
        log::info!("Parsed project burn leaderboard: {} entries, total burned: {:.2} MEMO", 
                  entries.len(), total_burned_tokens as f64 / 1_000_000.0);
        
        Ok(ProjectBurnLeaderboardResponse {
            entries,
            total_burned_tokens,
        })
    }
    
    /// Get the rank of a specific project in the burn leaderboard
    /// 
    /// # Parameters
    /// * `project_id` - Project ID
    /// 
    /// # Returns
    /// Rank (1-100), return None if the project is not in the leaderboard
    pub async fn get_project_burn_rank(&self, project_id: u64) -> Result<Option<u8>, RpcError> {
        let leaderboard = self.get_project_burn_leaderboard().await?;
        
        for entry in &leaderboard.entries {
            if entry.project_id == project_id {
                return Ok(Some(entry.rank));
            }
        }
        
        Ok(None)
    }

    /// Check if a specific project exists
    /// 
    /// # Parameters
    /// * `project_id` - The ID of the project to check
    /// 
    /// # Returns
    /// True if the project exists, false otherwise
    pub async fn project_exists(&self, project_id: u64) -> Result<bool, RpcError> {
        match self.get_project_info(project_id).await {
            Ok(_) => Ok(true),
            Err(RpcError::Other(msg)) if msg.contains("not found") => Ok(false),
            Err(e) => Err(e),
        }
    }
    
    /// Get the total number of projects that have been created
    /// 
    /// # Returns
    /// The total number of projects from the global counter
    pub async fn get_total_projects(&self) -> Result<u64, RpcError> {
        let stats = self.get_project_global_statistics().await?;
        Ok(stats.total_projects)
    }
    
    /// Get projects within a specific range
    /// 
    /// # Parameters
    /// * `start_id` - Starting project ID (inclusive)
    /// * `end_id` - Ending project ID (exclusive)
    /// 
    /// # Returns
    /// Vector of project information for existing projects in the range
    pub async fn get_projects_range(&self, start_id: u64, end_id: u64) -> Result<Vec<ProjectInfo>, RpcError> {
        if start_id >= end_id {
            return Err(RpcError::InvalidParameter(format!("Invalid range: start_id {} >= end_id {}", start_id, end_id)));
        }
        
        let mut projects = Vec::new();
        
        for project_id in start_id..end_id {
            match self.get_project_info(project_id).await {
                Ok(project_info) => {
                    projects.push(project_info);
                },
                Err(e) => {
                    log::debug!("Failed to fetch project {}: {}", project_id, e);
                    // Continue to next project instead of failing
                }
            }
        }
        
        log::info!("Fetched {} projects from range {}-{}", projects.len(), start_id, end_id);
        Ok(projects)
    }

    /// Get burn messages for a project with pagination support
    /// 
    /// # Parameters
    /// * `project_id` - The ID of the project to get messages for
    /// * `limit` - Maximum number of messages to retrieve (1-1000)
    /// * `before` - Optional signature to get messages before this transaction
    /// 
    /// # Returns
    /// Project burn messages response with pagination info
    pub async fn get_project_burn_messages(
        &self,
        project_id: u64,
        limit: usize,
        before: Option<String>,
    ) -> Result<ProjectBurnMessagesResponse, RpcError> {
        // Validate parameters
        if limit == 0 || limit > 1000 {
            return Err(RpcError::InvalidParameter("Limit must be between 1 and 1000".to_string()));
        }
        
        // Get project PDA
        let (project_pda, _) = ProjectConfig::get_project_pda(project_id)?;
        
        // Check if project exists
        if !self.project_exists(project_id).await? {
            return Err(RpcError::Other(format!("Project {} not found", project_id)));
        }
        
        // Get signatures for the project account
        let mut params = serde_json::json!([
            project_pda.to_string(),
            {
                "limit": limit,
                "commitment": "confirmed"
            }
        ]);
        
        // Add 'before' parameter if specified
        if let Some(before_sig) = before {
            params[1]["before"] = serde_json::Value::String(before_sig);
        }
        
        log::info!("Fetching signatures for project {}: {}", project_id, project_pda);
        
        // Get signatures for address
        let signatures_response: serde_json::Value = self.send_request("getSignaturesForAddress", params).await?;
        let signatures = signatures_response.as_array()
            .ok_or_else(|| RpcError::Other("Invalid signatures response format".to_string()))?;
        
        log::info!("Found {} signatures for project {}", signatures.len(), project_id);
        
        let mut messages = Vec::new();
        
        // Process each signature - memo data is already included in the response!
        for sig_info in signatures {
            let signature = sig_info["signature"]
                .as_str()
                .unwrap_or("")
                .to_string();
            
            if signature.is_empty() {
                continue;
            }
            
            // Extract timestamp and slot directly from signature info
            let block_time = sig_info["blockTime"].as_i64().unwrap_or(0);
            let slot = sig_info["slot"].as_u64().unwrap_or(0);
            
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
                
                // Parse memo data for project burns
                if let Some((burner, message, msg_type, burn_amount)) = parse_project_memo_data(memo_bytes) {
                    // Only include burn messages
                    if msg_type == "burn" && !message.trim().is_empty() {
                        if let Some(amount) = burn_amount {
                            messages.push(ProjectBurnMessage {
                                signature: signature.clone(),
                                burner,
                                message,
                                timestamp: block_time,
                                slot,
                                burn_amount: amount,
                            });
                        }
                    }
                }
            }
        }
        
        // Sort messages by timestamp from oldest to newest (ascending order)
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        let has_more = signatures.len() == limit;
        let total_found = messages.len();
        
        log::info!("Found {} burn messages for project {}", total_found, project_id);
        
        Ok(ProjectBurnMessagesResponse {
            project_id,
            messages,
            total_found,
            has_more,
        })
    }
}
