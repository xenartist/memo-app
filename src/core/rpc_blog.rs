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

/// Blog creation data version
pub const BLOG_CREATION_DATA_VERSION: u8 = 1;

/// Blog update data version
pub const BLOG_UPDATE_DATA_VERSION: u8 = 1;

/// Memo-Blog contract configuration and constants
pub struct BlogConfig;

impl BlogConfig {
    // Note: Program IDs and token mint are now retrieved dynamically from network configuration
    
    /// PDA Seeds for blog contract
    pub const GLOBAL_BLOG_COUNTER_SEED: &'static [u8] = b"global_blog_counter";
    pub const BLOG_SEED: &'static [u8] = b"blog";
    
    /// Minimum burn amount required to create/update/burn for a blog (1 token = 1,000,000 lamports)
    pub const MIN_BLOG_BURN_AMOUNT: u64 = 1_000_000;
    
    // Note: Memo validation limits, payload length, and compute unit config
    // are now directly used from the constants module to avoid duplication
    
    /// Helper functions
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.blog_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-blog program ID: {}", e)))
    }
    
    /// Get memo-burn program ID
    pub fn get_memo_burn_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.burn_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-burn program ID: {}", e)))
    }
    
    /// Get memo-mint program ID
    pub fn get_memo_mint_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.mint_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-mint program ID: {}", e)))
    }
    
    // Note: get_token_2022_program_id() and validate_memo_length() 
    // are now provided by rpc_base module to avoid duplication
    
    /// Get memo token mint
    pub fn get_memo_token_mint() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.token_mint)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo token mint: {}", e)))
    }
    
    /// Calculate global blog counter PDA
    pub fn get_global_blog_counter_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::GLOBAL_BLOG_COUNTER_SEED],
            &program_id
        ))
    }
    
    /// Calculate blog PDA for a specific blog ID
    pub fn get_blog_pda(blog_id: u64) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::BLOG_SEED, &blog_id.to_le_bytes()],
            &program_id
        ))
    }
    
    /// Get create_blog instruction discriminator
    pub fn get_create_blog_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:create_blog");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Get update_blog instruction discriminator
    pub fn get_update_blog_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:update_blog");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Get burn_for_blog instruction discriminator
    pub fn get_burn_for_blog_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:burn_for_blog");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Get mint_for_blog instruction discriminator
    pub fn get_mint_for_blog_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:mint_for_blog");
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
    
    /// Calculate mint authority PDA (from memo-mint program)
    pub fn get_mint_authority_pda() -> Result<(Pubkey, u8), RpcError> {
        let memo_mint_program_id = Self::get_memo_mint_program_id()?;
        Ok(Pubkey::find_program_address(
            &[b"mint_authority"],
            &memo_mint_program_id
        ))
    }
}

/// BurnMemo structure (compatible with memo-burn contract)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BurnMemo {
    /// Version of the BurnMemo structure (for future compatibility)
    pub version: u8,
    
    /// Burn amount (must match actual burn amount, 0 for mint operations)
    pub burn_amount: u64,
    
    /// Application payload (variable length, max 787 bytes)
    pub payload: Vec<u8>,
}

/// Blog creation data structure (stored in BurnMemo.payload)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BlogCreationData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "blog" for memo-blog contract)
    pub category: String,
    
    /// Operation type (must be "create_blog" for blog creation)
    pub operation: String,
    
    /// Blog ID (must match expected_blog_id)
    pub blog_id: u64,
    
    /// Blog name (required, 1-64 characters)
    pub name: String,
    
    /// Blog description (optional, max 256 characters)
    pub description: String,
    
    /// Blog image info (optional, max 256 characters)
    pub image: String,
}

impl BlogCreationData {
    /// Create new blog creation data
    pub fn new(
        blog_id: u64,
        name: String,
        description: String,
        image: String,
    ) -> Self {
        Self {
            version: BLOG_CREATION_DATA_VERSION,
            category: "blog".to_string(),
            operation: "create_blog".to_string(),
            blog_id,
            name,
            description,
            image,
        }
    }
    
    /// Validate the blog creation data
    pub fn validate(&self, expected_blog_id: u64) -> Result<(), RpcError> {
        // Validate version
        if self.version != BLOG_CREATION_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported blog creation data version: {} (expected: {})", 
                self.version, BLOG_CREATION_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "blog" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'blog')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "create_blog" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'create_blog')", self.operation
            )));
        }
        
        // Validate blog ID
        if self.blog_id != expected_blog_id {
            return Err(RpcError::InvalidParameter(format!(
                "Blog ID mismatch: data contains {}, expected {}", 
                self.blog_id, expected_blog_id
            )));
        }
        
        // Validate name (required, 1-64 characters)
        if self.name.is_empty() || self.name.len() > 64 {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid blog name: '{}' (must be 1-64 characters)", self.name
            )));
        }
        
        // Validate description (optional, max 256 characters)
        if self.description.len() > 256 {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid blog description: {} characters (max: 256)", self.description.len()
            )));
        }
        
        // Validate image (optional, max 256 characters)
        if self.image.len() > 256 {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid blog image: {} characters (max: 256)", self.image.len()
            )));
        }
        
        Ok(())
    }
    
    /// Calculate the final memo size after Borsh serialization and Base64 encoding
    /// Returns the size in bytes that will be used for the SPL memo instruction
    pub fn calculate_final_memo_size(&self, burn_amount: u64) -> Result<usize, RpcError> {
        // Create BurnMemo structure
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload: self.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize blog data: {}", e)))?,
        };
        
        // Serialize BurnMemo to Borsh bytes
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        
        // Encode to Base64 (this is what gets sent as the memo)
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        Ok(memo_data_base64.len())
    }
}

/// Blog update data structure (stored in BurnMemo.payload)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BlogUpdateData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "blog" for memo-blog contract)
    pub category: String,
    
    /// Operation type (must be "update_blog" for blog update)
    pub operation: String,
    
    /// Blog ID (must match the target blog)
    pub blog_id: u64,
    
    /// Updated fields (all optional)
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
}

impl BlogUpdateData {
    /// Create new blog update data
    pub fn new(
        blog_id: u64,
        name: Option<String>,
        description: Option<String>,
        image: Option<String>,
    ) -> Self {
        Self {
            version: BLOG_UPDATE_DATA_VERSION,
            category: "blog".to_string(),
            operation: "update_blog".to_string(),
            blog_id,
            name,
            description,
            image,
        }
    }
    
    /// Validate the blog update data
    pub fn validate(&self, expected_blog_id: u64) -> Result<(), RpcError> {
        // Validate version
        if self.version != BLOG_UPDATE_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported blog update data version: {} (expected: {})", 
                self.version, BLOG_UPDATE_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "blog" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'blog')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "update_blog" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'update_blog')", self.operation
            )));
        }
        
        // Validate blog ID
        if self.blog_id != expected_blog_id {
            return Err(RpcError::InvalidParameter(format!(
                "Blog ID mismatch: data contains {}, expected {}", 
                self.blog_id, expected_blog_id
            )));
        }
        
        // Validate optional fields
        if let Some(ref name) = self.name {
            if name.is_empty() || name.len() > 64 {
                return Err(RpcError::InvalidParameter(format!(
                    "Invalid blog name: '{}' (must be 1-64 characters)", name
                )));
            }
        }
        
        if let Some(ref description) = self.description {
            if description.len() > 256 {
                return Err(RpcError::InvalidParameter(format!(
                    "Invalid blog description: {} characters (max: 256)", description.len()
                )));
            }
        }
        
        if let Some(ref image) = self.image {
            if image.len() > 256 {
                return Err(RpcError::InvalidParameter(format!(
                    "Invalid blog image: {} characters (max: 256)", image.len()
                )));
            }
        }
        
        Ok(())
    }
}

/// Blog burn data structure (stored in BurnMemo.payload for burn_for_blog)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BlogBurnData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "blog" for memo-blog contract)
    pub category: String,
    
    /// Operation type (must be "burn_for_blog" for burning tokens)
    pub operation: String,
    
    /// Blog ID (must match the target blog)
    pub blog_id: u64,
    
    /// Burner pubkey as string (must match the transaction signer)
    pub burner: String,
    
    /// Burn message (optional, max 696 characters)
    pub message: String,
}

impl BlogBurnData {
    /// Create new blog burn data
    pub fn new(
        blog_id: u64,
        burner: String,
        message: String,
    ) -> Self {
        Self {
            version: BLOG_CREATION_DATA_VERSION,
            category: "blog".to_string(),
            operation: "burn_for_blog".to_string(),
            blog_id,
            burner,
            message,
        }
    }
    
    /// Validate the blog burn data
    pub fn validate(&self, expected_blog_id: u64, expected_burner: &str) -> Result<(), RpcError> {
        // Validate version
        if self.version != BLOG_CREATION_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported blog burn data version: {} (expected: {})", 
                self.version, BLOG_CREATION_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "blog" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'blog')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "burn_for_blog" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'burn_for_blog')", self.operation
            )));
        }
        
        // Validate blog ID
        if self.blog_id != expected_blog_id {
            return Err(RpcError::InvalidParameter(format!(
                "Blog ID mismatch: data contains {}, expected {}", 
                self.blog_id, expected_blog_id
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

/// Blog mint data structure (stored in BurnMemo.payload for mint_for_blog)
/// Note: For mint operations, the burn_amount in BurnMemo should be 0
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BlogMintData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "blog" for memo-blog contract)
    pub category: String,
    
    /// Operation type (must be "mint_for_blog" for minting tokens)
    pub operation: String,
    
    /// Blog ID (must match the target blog)
    pub blog_id: u64,
    
    /// Minter pubkey as string (must match the transaction signer / blog creator)
    pub minter: String,
    
    /// Mint message (optional, max 696 characters)
    pub message: String,
}

impl BlogMintData {
    /// Create new blog mint data
    pub fn new(
        blog_id: u64,
        minter: String,
        message: String,
    ) -> Self {
        Self {
            version: BLOG_CREATION_DATA_VERSION,
            category: "blog".to_string(),
            operation: "mint_for_blog".to_string(),
            blog_id,
            minter,
            message,
        }
    }
    
    /// Validate the blog mint data
    pub fn validate(&self, expected_blog_id: u64, expected_minter: &str) -> Result<(), RpcError> {
        // Validate version
        if self.version != BLOG_CREATION_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported blog mint data version: {} (expected: {})", 
                self.version, BLOG_CREATION_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "blog" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'blog')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "mint_for_blog" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'mint_for_blog')", self.operation
            )));
        }
        
        // Validate blog ID
        if self.blog_id != expected_blog_id {
            return Err(RpcError::InvalidParameter(format!(
                "Blog ID mismatch: data contains {}, expected {}", 
                self.blog_id, expected_blog_id
            )));
        }
        
        // Validate minter
        if self.minter != expected_minter {
            return Err(RpcError::InvalidParameter(format!(
                "Minter mismatch: data contains {}, expected {}", 
                self.minter, expected_minter
            )));
        }
        
        // Validate message (optional, max 696 characters)
        if self.message.len() > 696 {
            return Err(RpcError::InvalidParameter(format!(
                "Mint message too long: {} characters (max: 696)", self.message.len()
            )));
        }
        
        Ok(())
    }
}

/// Represents global blog statistics from the memo-blog contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogGlobalStatistics {
    pub total_blogs: u64,
}

/// Represents a blog's information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogInfo {
    pub blog_id: u64,
    pub creator: String,  // Base58 encoded pubkey
    pub created_at: i64,
    pub last_updated: i64,
    pub name: String,
    pub description: String,
    pub image: String,
    pub memo_count: u64,
    pub burned_amount: u64,
    pub minted_amount: u64,
    pub last_memo_time: i64,
    pub bump: u8,
}

/// Summary statistics for all blogs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogStatistics {
    pub total_blogs: u64,
    pub valid_blogs: u64,
    pub total_memos: u64,
    pub total_burned_tokens: u64,
    pub total_mint_operations: u64,
    pub blogs: Vec<BlogInfo>,
}

/// Represents a single burn/mint action to a blog
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlogMemoMessage {
    pub signature: String,      // Transaction signature
    pub user: String,           // User's public key (burner or minter)
    pub message: String,        // The memo message
    pub timestamp: i64,         // Block time
    pub slot: u64,             // Slot number
    pub amount: u64,           // Amount (burned tokens or 0 for mint)
    pub memo_type: String,     // "burn" or "mint"
}

/// Response containing memo messages for a blog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogMemoMessagesResponse {
    pub blog_id: u64,
    pub messages: Vec<BlogMemoMessage>,
    pub total_found: usize,
    pub has_more: bool,        // Indicates if there are more messages available
}

/// Operation type for blog contract transactions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlogOperationType {
    CreateBlog,
    UpdateBlog,
    BurnForBlog,
    MintForBlog,
}

/// Detailed information for different operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlogOperationDetails {
    /// Create blog operation with full blog details
    Create {
        blog_id: u64,
        name: String,
        description: String,
        image: String,
    },
    /// Update blog operation with updated fields
    Update {
        blog_id: u64,
        name: Option<String>,
        description: Option<String>,
        image: Option<String>,
    },
    /// Burn for blog operation with message
    Burn {
        blog_id: u64,
        message: String,
    },
    /// Mint for blog operation with message
    Mint {
        blog_id: u64,
        message: String,
    },
}

/// Transaction info for blog contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogContractTransaction {
    pub signature: String,          // Transaction signature
    pub user: String,               // User's public key (creator/burner/minter)
    pub timestamp: i64,             // Block time
    pub slot: u64,                  // Slot number
    pub burn_amount: u64,           // Amount burned (in lamports, 0 for mint)
    pub operation_type: BlogOperationType,  // Type of operation
    pub details: BlogOperationDetails,      // Operation-specific details
}

/// Response containing recent transactions for blog contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogContractTransactionsResponse {
    pub transactions: Vec<BlogContractTransaction>,
    pub total_found: usize,
}

/// Parse Base64+Borsh-formatted memo data to extract blog burn message
fn parse_borsh_blog_burn_message(memo_data: &[u8]) -> Option<(String, String, u64)> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to BurnMemo
    match BurnMemo::try_from_slice(&borsh_bytes) {
        Ok(burn_memo) => {
            // Deserialize payload to BlogBurnData
            match BlogBurnData::try_from_slice(&burn_memo.payload) {
                Ok(burn_data) => {
                    // Validate category and operation
                    if burn_data.category == "blog" && burn_data.operation == "burn_for_blog" {
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

/// Parse Base64+Borsh-formatted memo data to extract blog mint message
fn parse_borsh_blog_mint_message(memo_data: &[u8]) -> Option<(String, String)> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to BurnMemo
    match BurnMemo::try_from_slice(&borsh_bytes) {
        Ok(burn_memo) => {
            // For mint operations, burn_amount should be 0
            if burn_memo.burn_amount != 0 {
                return None;
            }
            
            // Deserialize payload to BlogMintData
            match BlogMintData::try_from_slice(&burn_memo.payload) {
                Ok(mint_data) => {
                    // Validate category and operation
                    if mint_data.category == "blog" && mint_data.operation == "mint_for_blog" {
                        Some((mint_data.minter, mint_data.message))
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

/// Parse memo data for all blog operations (create, update, burn, mint)
/// Returns (user, operation_type, details, burn_amount)
fn parse_blog_operation_memo(memo_data: &[u8]) -> Option<(String, BlogOperationType, BlogOperationDetails, u64)> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to BurnMemo
    let burn_memo = BurnMemo::try_from_slice(&borsh_bytes).ok()?;
    let burn_amount = burn_memo.burn_amount;
    
    // Try to parse as BlogCreationData
    if let Ok(creation_data) = BlogCreationData::try_from_slice(&burn_memo.payload) {
        if creation_data.category == "blog" && creation_data.operation == "create_blog" {
            return Some((
                creation_data.name.clone(), // Use blog name as "user" identifier
                BlogOperationType::CreateBlog,
                BlogOperationDetails::Create {
                    blog_id: creation_data.blog_id,
                    name: creation_data.name,
                    description: creation_data.description,
                    image: creation_data.image,
                },
                burn_amount,
            ));
        }
    }
    
    // Try to parse as BlogUpdateData
    if let Ok(update_data) = BlogUpdateData::try_from_slice(&burn_memo.payload) {
        if update_data.category == "blog" && update_data.operation == "update_blog" {
            return Some((
                update_data.name.clone().unwrap_or_else(|| "Blog Update".to_string()),
                BlogOperationType::UpdateBlog,
                BlogOperationDetails::Update {
                    blog_id: update_data.blog_id,
                    name: update_data.name,
                    description: update_data.description,
                    image: update_data.image,
                },
                burn_amount,
            ));
        }
    }
    
    // Try to parse as BlogBurnData
    if let Ok(burn_data) = BlogBurnData::try_from_slice(&burn_memo.payload) {
        if burn_data.category == "blog" && burn_data.operation == "burn_for_blog" {
            return Some((
                burn_data.burner.clone(),
                BlogOperationType::BurnForBlog,
                BlogOperationDetails::Burn {
                    blog_id: burn_data.blog_id,
                    message: burn_data.message,
                },
                burn_amount,
            ));
        }
    }
    
    // Try to parse as BlogMintData
    if let Ok(mint_data) = BlogMintData::try_from_slice(&burn_memo.payload) {
        if mint_data.category == "blog" && mint_data.operation == "mint_for_blog" {
            return Some((
                mint_data.minter.clone(),
                BlogOperationType::MintForBlog,
                BlogOperationDetails::Mint {
                    blog_id: mint_data.blog_id,
                    message: mint_data.message,
                },
                0, // Mint operations have 0 burn amount
            ));
        }
    }
    
    None
}

/// Parse memo data to determine message type for blogs
fn parse_blog_memo_data(memo_data: &[u8]) -> Option<(String, String, String, u64)> {
    // Try parsing as blog burn message
    if let Some((burner, message, burn_amount)) = parse_borsh_blog_burn_message(memo_data) {
        return Some((burner, message, "burn".to_string(), burn_amount));
    }
    
    // Try parsing as blog mint message
    if let Some((minter, message)) = parse_borsh_blog_mint_message(memo_data) {
        return Some((minter, message, "mint".to_string(), 0));
    }
    
    None
}

impl RpcConnection {
    /// Build an unsigned transaction to create a blog
    pub async fn build_create_blog_transaction(
        &self,
        user_pubkey: &Pubkey,
        name: &str,
        description: &str,
        image: &str,
        burn_amount: u64,
    ) -> Result<(Transaction, u64), RpcError> {
        // Basic parameter validation
        if name.is_empty() || name.len() > 64 {
            return Err(RpcError::InvalidParameter(format!("Blog name must be 1-64 characters, got {}", name.len())));
        }
        if description.len() > 256 {
            return Err(RpcError::InvalidParameter(format!("Blog description must be at most 256 characters, got {}", description.len())));
        }
        if image.len() > 256 {
            return Err(RpcError::InvalidParameter(format!("Blog image must be at most 256 characters, got {}", image.len())));
        }
        if burn_amount < BlogConfig::MIN_BLOG_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!("Burn amount must be at least {} MEMO tokens", BlogConfig::MIN_BLOG_BURN_AMOUNT / 1_000_000)));
        }
        if burn_amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens".to_string()));
        }
        
        log::info!("Building create blog transaction '{}': {} tokens", name, burn_amount / 1_000_000);
        
        // Get next blog_id
        let global_stats = self.get_blog_global_statistics().await?;
        let expected_blog_id = global_stats.total_blogs;
        
        let blog_program_id = BlogConfig::get_program_id()?;
        let memo_token_mint = BlogConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_burn_program_id = BlogConfig::get_memo_burn_program_id()?;
        
        let (global_counter_pda, _) = BlogConfig::get_global_blog_counter_pda()?;
        let (blog_pda, _) = BlogConfig::get_blog_pda(expected_blog_id)?;
        let (user_global_burn_stats_pda, _) = BlogConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey, &memo_token_mint, &token_2022_program_id,
        );
        
        let blog_creation_data = BlogCreationData::new(
            expected_blog_id, name.to_string(), description.to_string(), 
            image.to_string(),
        );
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload: blog_creation_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize blog data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        validate_memo_length_bytes(memo_data_base64.as_bytes())?;
        
        // Build base instructions (without compute budget)
        let mut base_instructions = vec![];
        
        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(memo_data_base64.as_bytes(), &[user_pubkey]));
        
        // Create blog instruction
        let mut instruction_data = BlogConfig::get_create_blog_discriminator().to_vec();
        instruction_data.extend_from_slice(&expected_blog_id.to_le_bytes());
        instruction_data.extend_from_slice(&burn_amount.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            blog_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(global_counter_pda, false),
                AccountMeta::new(blog_pda, false),
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
        
        log::info!("Simulating create blog transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Create blog simulation consumed {} compute units", units_consumed);
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
        
        Ok((transaction, expected_blog_id))
    }

    /// Build an unsigned transaction to update a blog
    pub async fn build_update_blog_transaction(
        &self,
        user_pubkey: &Pubkey,
        blog_id: u64,
        name: Option<String>,
        description: Option<String>,
        image: Option<String>,
        burn_amount: u64,
    ) -> Result<Transaction, RpcError> {
        // Basic parameter validation
        if let Some(ref n) = name {
            if n.is_empty() || n.len() > 64 {
                return Err(RpcError::InvalidParameter(format!("Blog name must be 1-64 characters, got {}", n.len())));
            }
        }
        if let Some(ref d) = description {
            if d.len() > 256 {
                return Err(RpcError::InvalidParameter(format!("Blog description must be at most 256 characters, got {}", d.len())));
            }
        }
        if let Some(ref i) = image {
            if i.len() > 256 {
                return Err(RpcError::InvalidParameter(format!("Blog image must be at most 256 characters, got {}", i.len())));
            }
        }
        if burn_amount < BlogConfig::MIN_BLOG_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!("Burn amount must be at least {} MEMO tokens", BlogConfig::MIN_BLOG_BURN_AMOUNT / 1_000_000)));
        }
        if burn_amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens".to_string()));
        }
        
        log::info!("Building update blog transaction for blog {}: {} tokens", blog_id, burn_amount / 1_000_000);
        
        let blog_program_id = BlogConfig::get_program_id()?;
        let memo_token_mint = BlogConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_burn_program_id = BlogConfig::get_memo_burn_program_id()?;
        
        let (blog_pda, _) = BlogConfig::get_blog_pda(blog_id)?;
        let (user_global_burn_stats_pda, _) = BlogConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey, &memo_token_mint, &token_2022_program_id,
        );
        
        let blog_update_data = BlogUpdateData::new(blog_id, name, description, image);
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload: blog_update_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize blog data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        validate_memo_length_bytes(memo_data_base64.as_bytes())?;
        
        // Build base instructions (without compute budget)
        let mut base_instructions = vec![];
        
        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(memo_data_base64.as_bytes(), &[user_pubkey]));
        
        // Update blog instruction
        let mut instruction_data = BlogConfig::get_update_blog_discriminator().to_vec();
        instruction_data.extend_from_slice(&blog_id.to_le_bytes());
        instruction_data.extend_from_slice(&burn_amount.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            blog_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(blog_pda, false),
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
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000u32));
        
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
        
        log::info!("Simulating update blog transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        // Parse compute units consumed
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Update blog simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction
        let mut final_instructions = base_instructions;
        
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

    /// Build an unsigned transaction to burn tokens for a blog
    pub async fn build_burn_tokens_for_blog_transaction(
        &self,
        user_pubkey: &Pubkey,
        blog_id: u64,
        amount: u64,
        message: &str,
    ) -> Result<Transaction, RpcError> {
        // Validate amount
        if amount < BlogConfig::MIN_BLOG_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!(
                "Burn amount must be at least {} MEMO tokens", 
                BlogConfig::MIN_BLOG_BURN_AMOUNT / 1_000_000
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
        
        log::info!("Building burn tokens for blog transaction: {} tokens for blog {}", 
                  amount / 1_000_000, blog_id);
        
        let blog_program_id = BlogConfig::get_program_id()?;
        let memo_token_mint = BlogConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_burn_program_id = BlogConfig::get_memo_burn_program_id()?;
        
        let (blog_pda, _) = BlogConfig::get_blog_pda(blog_id)?;
        let (user_global_burn_stats_pda, _) = BlogConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        let burn_data = BlogBurnData::new(blog_id, user_pubkey.to_string(), message.to_string());
        burn_data.validate(blog_id, &user_pubkey.to_string())?;
        
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
        let mut instruction_data = BlogConfig::get_burn_for_blog_discriminator().to_vec();
        instruction_data.extend_from_slice(&blog_id.to_le_bytes());
        instruction_data.extend_from_slice(&amount.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            blog_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(blog_pda, false),
                AccountMeta::new(memo_token_mint, false),
                AccountMeta::new(user_token_account, false),
                AccountMeta::new(user_global_burn_stats_pda, false),
                AccountMeta::new_readonly(token_2022_program_id, false),
                AccountMeta::new_readonly(memo_burn_program_id, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
            ],
        ));
        
        let blockhash = self.get_latest_blockhash().await?;
        
        // Simulate
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000u32));
        
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
            }
        }
        let sim_message = Message::new(&sim_instructions, Some(user_pubkey));
        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = blockhash;
        
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);
        
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });
        
        log::info!("Simulating burn tokens for blog transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Burn tokens for blog simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction
        let mut final_instructions = base_instructions;
        
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

    /// Build an unsigned transaction to mint tokens for a blog
    pub async fn build_mint_tokens_for_blog_transaction(
        &self,
        user_pubkey: &Pubkey,
        blog_id: u64,
        message: &str,
    ) -> Result<Transaction, RpcError> {
        // Validate message length
        if message.len() > 696 {
            return Err(RpcError::InvalidParameter(
                "Mint message too long (max 696 characters)".to_string()
            ));
        }
        
        log::info!("Building mint tokens for blog transaction for blog {}", blog_id);
        
        let blog_program_id = BlogConfig::get_program_id()?;
        let memo_token_mint = BlogConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_mint_program_id = BlogConfig::get_memo_mint_program_id()?;
        
        let (blog_pda, _) = BlogConfig::get_blog_pda(blog_id)?;
        let (mint_authority_pda, _) = BlogConfig::get_mint_authority_pda()?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        let mint_data = BlogMintData::new(blog_id, user_pubkey.to_string(), message.to_string());
        mint_data.validate(blog_id, &user_pubkey.to_string())?;
        
        // For mint operations, burn_amount should be 0
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount: 0,
            payload: mint_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize mint data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        validate_memo_length_bytes(memo_data_base64.as_bytes())?;
        
        // Build base instructions (without compute budget)
        let mut base_instructions = vec![];
        
        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(memo_data_base64.as_bytes(), &[user_pubkey]));
        
        // Mint instruction
        let mut instruction_data = BlogConfig::get_mint_for_blog_discriminator().to_vec();
        instruction_data.extend_from_slice(&blog_id.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            blog_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(blog_pda, false),
                AccountMeta::new(memo_token_mint, false),
                AccountMeta::new_readonly(mint_authority_pda, false),
                AccountMeta::new(user_token_account, false),
                AccountMeta::new_readonly(token_2022_program_id, false),
                AccountMeta::new_readonly(memo_mint_program_id, false),
                AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
            ],
        ));
        
        let blockhash = self.get_latest_blockhash().await?;
        
        // Simulate
        let mut sim_instructions = base_instructions.clone();
        sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000u32));
        
        if let Some(settings) = crate::core::settings::load_current_network_settings() {
            if let Some(price) = settings.get_cu_price_micro_lamports() {
                sim_instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
            }
        }
        let sim_message = Message::new(&sim_instructions, Some(user_pubkey));
        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = blockhash;
        
        let sim_serialized_tx = base64::encode(bincode::serialize(&sim_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize simulation transaction: {}", e)))?);
        
        let sim_options = serde_json::json!({
            "encoding": "base64",
            "commitment": "confirmed",
            "replaceRecentBlockhash": true,
            "sigVerify": false
        });
        
        log::info!("Simulating mint tokens for blog transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Mint tokens for blog simulation consumed {} compute units", units_consumed);
            units_consumed
        } else {
            return Err(RpcError::Other("Failed to get compute units from simulation".to_string()));
        };
        
        // Build final transaction
        let mut final_instructions = base_instructions;
        
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

    /// Get global blog statistics from the memo-blog contract
    /// 
    /// # Returns
    /// Global statistics including total number of blogs created
    pub async fn get_blog_global_statistics(&self) -> Result<BlogGlobalStatistics, RpcError> {
        let (global_counter_pda, _) = BlogConfig::get_global_blog_counter_pda()?;
        
        log::info!("Fetching global blog statistics from PDA: {}", global_counter_pda);
        
        let account_info = self.get_account_info(&global_counter_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(
                "Global blog counter not found. Please initialize the memo-blog system first.".to_string()
            ));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // Verify the account is owned by memo-blog program
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account owner".to_string()))?;
        
        let expected_program_id = BlogConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-blog program. Expected: {}, Got: {}", 
                expected_program_id, owner
            )));
        }
        
        // Parse total blogs count (skip 8-byte discriminator, read next 8 bytes)
        if data.len() < 16 {
            return Err(RpcError::Other("Invalid account data size".to_string()));
        }
        
        let total_blogs_bytes = &data[8..16];
        let total_blogs = u64::from_le_bytes(
            total_blogs_bytes.try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse total blogs: {:?}", e)))?
        );
        
        log::info!("Global blog statistics: {} total blogs", total_blogs);
        
        Ok(BlogGlobalStatistics { total_blogs })
    }
    
    /// Get information for a specific blog
    /// 
    /// # Parameters
    /// * `blog_id` - The ID of the blog to fetch
    /// 
    /// # Returns
    /// Blog information if it exists
    pub async fn get_blog_info(&self, blog_id: u64) -> Result<BlogInfo, RpcError> {
        let (blog_pda, _) = BlogConfig::get_blog_pda(blog_id)?;
        
        log::info!("Fetching blog {} info from PDA: {}", blog_id, blog_pda);
        
        let account_info = self.get_account_info(&blog_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(format!("Blog {} not found", blog_id)));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // Verify the account is owned by memo-blog program
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account owner".to_string()))?;
        
        let expected_program_id = BlogConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-blog program. Expected: {}, Got: {}", 
                expected_program_id, owner
            )));
        }
        
        // Parse blog data
        self.parse_blog_data(&data)
    }
    
    /// Parse Blog account data according to the contract's data structure
    fn parse_blog_data(&self, data: &[u8]) -> Result<BlogInfo, RpcError> {
        if data.len() < 8 {
            return Err(RpcError::Other("Data too short for discriminator".to_string()));
        }
        
        let mut offset = 8; // Skip discriminator
        
        // Read blog_id (u64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for blog_id".to_string()));
        }
        let blog_id = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse blog_id: {:?}", e)))?
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
        
        // Read minted_amount (u64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for minted_amount".to_string()));
        }
        let minted_amount = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse minted_amount: {:?}", e)))?
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
        
        Ok(BlogInfo {
            blog_id,
            creator,
            created_at,
            last_updated,
            name,
            description,
            image,
            memo_count,
            burned_amount,
            minted_amount,
            last_memo_time,
            bump,
        })
    }
    
    /// Get comprehensive statistics for all blogs
    /// 
    /// # Returns
    /// Complete statistics including all blog information
    pub async fn get_all_blog_statistics(&self) -> Result<BlogStatistics, RpcError> {
        log::info!("Fetching comprehensive blog statistics...");
        
        // First get global statistics
        let global_stats = self.get_blog_global_statistics().await?;
        let total_blogs = global_stats.total_blogs;
        
        if total_blogs == 0 {
            log::info!("No blogs found");
            return Ok(BlogStatistics {
                total_blogs: 0,
                valid_blogs: 0,
                total_memos: 0,
                total_burned_tokens: 0,
                total_mint_operations: 0,
                blogs: vec![],
            });
        }
        
        let mut valid_blogs = 0;
        let mut total_memos = 0;
        let mut total_burned_tokens: u64 = 0;
        let mut total_mint_operations: u64 = 0;
        let mut blogs = Vec::new();
        
        // Iterate through all blogs
        for blog_id in 0..total_blogs {
            match self.get_blog_info(blog_id).await {
                Ok(blog_info) => {
                    valid_blogs += 1;
                    total_memos += blog_info.memo_count;
                    total_burned_tokens += blog_info.burned_amount;
                    total_mint_operations += blog_info.minted_amount;
                    blogs.push(blog_info);
                    
                    log::info!("Successfully fetched blog {}", blog_id);
                },
                Err(e) => {
                    log::warn!("Failed to fetch blog {}: {}", blog_id, e);
                }
            }
        }
        
        log::info!("Blog statistics summary: {}/{} valid blogs, {} total memos, {} total burned tokens, {} mint operations", 
                  valid_blogs, total_blogs, total_memos, total_burned_tokens, total_mint_operations);
        
        Ok(BlogStatistics {
            total_blogs,
            valid_blogs,
            total_memos,
            total_burned_tokens,
            total_mint_operations,
            blogs,
        })
    }

    /// Check if a specific blog exists
    /// 
    /// # Parameters
    /// * `blog_id` - The ID of the blog to check
    /// 
    /// # Returns
    /// True if the blog exists, false otherwise
    pub async fn blog_exists(&self, blog_id: u64) -> Result<bool, RpcError> {
        match self.get_blog_info(blog_id).await {
            Ok(_) => Ok(true),
            Err(RpcError::Other(msg)) if msg.contains("not found") => Ok(false),
            Err(e) => Err(e),
        }
    }
    
    /// Get the total number of blogs that have been created
    /// 
    /// # Returns
    /// The total number of blogs from the global counter
    pub async fn get_total_blogs(&self) -> Result<u64, RpcError> {
        let stats = self.get_blog_global_statistics().await?;
        Ok(stats.total_blogs)
    }
    
    /// Get blogs within a specific range
    /// 
    /// # Parameters
    /// * `start_id` - Starting blog ID (inclusive)
    /// * `end_id` - Ending blog ID (exclusive)
    /// 
    /// # Returns
    /// Vector of blog information for existing blogs in the range
    pub async fn get_blogs_range(&self, start_id: u64, end_id: u64) -> Result<Vec<BlogInfo>, RpcError> {
        if start_id >= end_id {
            return Err(RpcError::InvalidParameter(format!("Invalid range: start_id {} >= end_id {}", start_id, end_id)));
        }
        
        let mut blogs = Vec::new();
        
        for blog_id in start_id..end_id {
            match self.get_blog_info(blog_id).await {
                Ok(blog_info) => {
                    blogs.push(blog_info);
                },
                Err(e) => {
                    log::debug!("Failed to fetch blog {}: {}", blog_id, e);
                    // Continue to next blog instead of failing
                }
            }
        }
        
        log::info!("Fetched {} blogs from range {}-{}", blogs.len(), start_id, end_id);
        Ok(blogs)
    }

    /// Get memo messages for a blog with pagination support
    /// 
    /// # Parameters
    /// * `blog_id` - The ID of the blog to get messages for
    /// * `limit` - Maximum number of messages to retrieve (1-1000)
    /// * `before` - Optional signature to get messages before this transaction
    /// 
    /// # Returns
    /// Blog memo messages response with pagination info
    pub async fn get_blog_memo_messages(
        &self,
        blog_id: u64,
        limit: usize,
        before: Option<String>,
    ) -> Result<BlogMemoMessagesResponse, RpcError> {
        // Validate parameters
        if limit == 0 || limit > 1000 {
            return Err(RpcError::InvalidParameter("Limit must be between 1 and 1000".to_string()));
        }
        
        // Get blog PDA
        let (blog_pda, _) = BlogConfig::get_blog_pda(blog_id)?;
        
        // Check if blog exists
        if !self.blog_exists(blog_id).await? {
            return Err(RpcError::Other(format!("Blog {} not found", blog_id)));
        }
        
        // Get signatures for the blog account
        let mut params = serde_json::json!([
            blog_pda.to_string(),
            {
                "limit": limit,
                "commitment": "confirmed"
            }
        ]);
        
        // Add 'before' parameter if specified
        if let Some(before_sig) = before {
            params[1]["before"] = serde_json::Value::String(before_sig);
        }
        
        log::info!("Fetching signatures for blog {}: {}", blog_id, blog_pda);
        
        // Get signatures for address
        let signatures_response: serde_json::Value = self.send_request("getSignaturesForAddress", params).await?;
        let signatures = signatures_response.as_array()
            .ok_or_else(|| RpcError::Other("Invalid signatures response format".to_string()))?;
        
        log::info!("Found {} signatures for blog {}", signatures.len(), blog_id);
        
        let mut messages = Vec::new();
        
        // Process each signature
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
            
            // Extract memo field directly from signature info
            if let Some(memo_str) = sig_info["memo"].as_str() {
                // The memo field format is "[length] base64_data"
                let memo_data = if let Some(space_pos) = memo_str.find(' ') {
                    &memo_str[space_pos + 1..]
                } else {
                    memo_str
                };
                
                // Convert string to bytes for parsing
                let memo_bytes = memo_data.as_bytes();
                
                // Parse memo data for blog burns and mints
                if let Some((user, message, memo_type, amount)) = parse_blog_memo_data(memo_bytes) {
                    // Include both burn and mint messages
                    if !message.trim().is_empty() {
                        messages.push(BlogMemoMessage {
                            signature: signature.clone(),
                            user,
                            message,
                            timestamp: block_time,
                            slot,
                            amount,
                            memo_type,
                        });
                    }
                }
            }
        }
        
        // Sort messages by timestamp from newest to oldest (descending order)
        messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        let has_more = signatures.len() == limit;
        let total_found = messages.len();
        
        log::info!("Found {} memo messages for blog {}", total_found, blog_id);
        
        Ok(BlogMemoMessagesResponse {
            blog_id,
            messages,
            total_found,
            has_more,
        })
    }

    /// Get recent transactions for the blog contract
    /// 
    /// Fetches the 20 most recent transactions to the blog contract address.
    /// 
    /// # Returns
    /// Recent transactions response with up to 20 transactions
    pub async fn get_recent_blog_contract_transactions(
        &self,
    ) -> Result<BlogContractTransactionsResponse, RpcError> {
        // Get blog program address (the contract address)
        let program_id = BlogConfig::get_program_id()?;
        
        // Get signatures for the contract address (limit to 20 most recent)
        let params = serde_json::json!([
            program_id.to_string(),
            {
                "limit": 20,
                "commitment": "confirmed"
            }
        ]);
        
        log::info!("Fetching recent transactions for blog contract: {}", program_id);
        
        // Get signatures for address
        let signatures_response: serde_json::Value = self.send_request("getSignaturesForAddress", params).await?;
        let signatures = signatures_response.as_array()
            .ok_or_else(|| RpcError::Other("Invalid signatures response format".to_string()))?;
        
        log::info!("Found {} recent signatures for blog contract", signatures.len());
        
        let mut transactions = Vec::new();
        
        // Process each signature
        for sig_info in signatures {
            let signature = sig_info["signature"]
                .as_str()
                .unwrap_or("")
                .to_string();
            
            if signature.is_empty() {
                continue;
            }
            
            // Extract timestamp and slot
            let block_time = sig_info["blockTime"].as_i64().unwrap_or(0);
            let slot = sig_info["slot"].as_u64().unwrap_or(0);
            
            // Extract memo field from signature info
            if let Some(memo_str) = sig_info["memo"].as_str() {
                // The memo field format is "[length] base64_data"
                let memo_data = if let Some(space_pos) = memo_str.find(' ') {
                    &memo_str[space_pos + 1..]
                } else {
                    memo_str
                };
                
                let memo_bytes = memo_data.as_bytes();
                
                // Parse memo data for all blog operations
                if let Some((user, operation_type, details, burn_amount)) = parse_blog_operation_memo(memo_bytes) {
                    transactions.push(BlogContractTransaction {
                        signature: signature.clone(),
                        user,
                        timestamp: block_time,
                        slot,
                        burn_amount,
                        operation_type,
                        details,
                    });
                }
            }
        }
        
        // Sort by timestamp from newest to oldest (descending order)
        transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        let total_found = transactions.len();
        
        log::info!("Found {} recent transactions for blog contract", total_found);
        
        Ok(BlogContractTransactionsResponse {
            transactions,
            total_found,
        })
    }
}

