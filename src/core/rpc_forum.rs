#![allow(dead_code)]

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

/// Post creation data version
pub const POST_CREATION_DATA_VERSION: u8 = 1;

/// Post burn data version
pub const POST_BURN_DATA_VERSION: u8 = 1;

/// Post mint data version
pub const POST_MINT_DATA_VERSION: u8 = 1;

/// Memo-Forum contract configuration and constants
pub struct ForumConfig;

impl ForumConfig {
    /// PDA Seeds for forum contract
    pub const GLOBAL_COUNTER_SEED: &'static [u8] = b"global_counter";
    pub const POST_SEED: &'static [u8] = b"post";
    
    /// Minimum burn amount required to create a post (1 token = 1,000,000 lamports)
    pub const MIN_POST_BURN_AMOUNT: u64 = 1_000_000;
    
    /// Maximum post title length
    pub const MAX_POST_TITLE_LENGTH: usize = 128;
    
    /// Maximum post content length
    pub const MAX_POST_CONTENT_LENGTH: usize = 512;
    
    /// Maximum post image length
    pub const MAX_POST_IMAGE_LENGTH: usize = 256;
    
    /// Maximum reply message length
    pub const MAX_REPLY_MESSAGE_LENGTH: usize = 512;
    
    /// Helper functions
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.forum_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-forum program ID: {}", e)))
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
    
    /// Calculate post PDA for a specific post ID
    pub fn get_post_pda(post_id: u64) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::POST_SEED, &post_id.to_le_bytes()],
            &program_id
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
    
    /// Get create_post instruction discriminator
    pub fn get_create_post_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:create_post");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Get burn_for_post instruction discriminator
    pub fn get_burn_for_post_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:burn_for_post");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Get mint_for_post instruction discriminator
    pub fn get_mint_for_post_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:mint_for_post");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Calculate user global burn stats PDA (from memo-burn program)
    pub fn get_user_global_burn_stats_pda(user_pubkey: &Pubkey) -> Result<(Pubkey, u8), RpcError> {
        let memo_burn_program_id = Self::get_memo_burn_program_id()?;
        Ok(Pubkey::find_program_address(
            &[b"user_global_burn_stats", user_pubkey.as_ref()],
            &memo_burn_program_id
        ))
    }
    
    /// Estimate memo size for create_post operation (for UI validation)
    /// Returns the estimated Base64-encoded memo size in bytes
    pub fn estimate_create_post_memo_size(
        creator: &str,
        post_id: u64,
        title: &str,
        content: &str,
        image: &str,
        burn_amount: u64,
    ) -> usize {
        // Create post creation data
        let post_data = PostCreationData::new(
            creator.to_string(),
            post_id,
            title.to_string(),
            content.to_string(),
            image.to_string(),
        );
        
        // Serialize payload
        let payload = match borsh::to_vec(&post_data) {
            Ok(data) => data,
            Err(_) => return 0,
        };
        
        // Create BurnMemo
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload,
        };
        
        // Serialize BurnMemo
        let memo_bytes = match borsh::to_vec(&burn_memo) {
            Ok(data) => data,
            Err(_) => return 0,
        };
        
        // Return Base64-encoded length
        base64::encode(&memo_bytes).len()
    }
    
    /// Estimate memo size for burn_for_post operation (for UI validation)
    pub fn estimate_burn_for_post_memo_size(
        user: &str,
        post_id: u64,
        message: &str,
        burn_amount: u64,
    ) -> usize {
        let burn_data = PostBurnData::new(
            user.to_string(),
            post_id,
            message.to_string(),
        );
        
        let payload = match borsh::to_vec(&burn_data) {
            Ok(data) => data,
            Err(_) => return 0,
        };
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload,
        };
        
        let memo_bytes = match borsh::to_vec(&burn_memo) {
            Ok(data) => data,
            Err(_) => return 0,
        };
        
        base64::encode(&memo_bytes).len()
    }
    
    /// Estimate memo size for mint_for_post operation (for UI validation)
    pub fn estimate_mint_for_post_memo_size(
        user: &str,
        post_id: u64,
        message: &str,
    ) -> usize {
        let mint_data = PostMintData::new(
            user.to_string(),
            post_id,
            message.to_string(),
        );
        
        let payload = match borsh::to_vec(&mint_data) {
            Ok(data) => data,
            Err(_) => return 0,
        };
        
        // For mint operations, burn_amount is 0
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount: 0,
            payload,
        };
        
        let memo_bytes = match borsh::to_vec(&burn_memo) {
            Ok(data) => data,
            Err(_) => return 0,
        };
        
        base64::encode(&memo_bytes).len()
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

/// Post creation data structure (stored in BurnMemo.payload)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PostCreationData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "forum" for memo-forum contract)
    pub category: String,
    
    /// Operation type (must be "create_post" for post creation)
    pub operation: String,
    
    /// Creator pubkey as string (must match the transaction signer)
    pub creator: String,
    
    /// Post ID (provided by client, used as part of PDA seed)
    pub post_id: u64,
    
    /// Post title (required, 1-128 characters)
    pub title: String,
    
    /// Post content (required, 1-512 characters)
    pub content: String,
    
    /// Post image (optional, max 256 characters)
    pub image: String,
}

impl PostCreationData {
    /// Create new post creation data
    pub fn new(
        creator: String,
        post_id: u64,
        title: String,
        content: String,
        image: String,
    ) -> Self {
        Self {
            version: POST_CREATION_DATA_VERSION,
            category: "forum".to_string(),
            operation: "create_post".to_string(),
            creator,
            post_id,
            title,
            content,
            image,
        }
    }
    
    /// Validate the post creation data
    pub fn validate(&self, expected_creator: &str, expected_post_id: u64) -> Result<(), RpcError> {
        // Validate version
        if self.version != POST_CREATION_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported post creation data version: {} (expected: {})", 
                self.version, POST_CREATION_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "forum" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'forum')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "create_post" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'create_post')", self.operation
            )));
        }
        
        // Validate creator
        if self.creator != expected_creator {
            return Err(RpcError::InvalidParameter(format!(
                "Creator mismatch: data contains {}, expected {}", 
                self.creator, expected_creator
            )));
        }
        
        // Validate post ID
        if self.post_id != expected_post_id {
            return Err(RpcError::InvalidParameter(format!(
                "Post ID mismatch: data contains {}, expected {}", 
                self.post_id, expected_post_id
            )));
        }
        
        // Validate title (required, 1-128 characters)
        if self.title.is_empty() || self.title.len() > ForumConfig::MAX_POST_TITLE_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid post title: '{}' (must be 1-{} characters)", 
                self.title, ForumConfig::MAX_POST_TITLE_LENGTH
            )));
        }
        
        // Validate content (required, 1-512 characters)
        if self.content.is_empty() || self.content.len() > ForumConfig::MAX_POST_CONTENT_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid post content: {} characters (must be 1-{})", 
                self.content.len(), ForumConfig::MAX_POST_CONTENT_LENGTH
            )));
        }
        
        // Validate image (optional, max 256 characters)
        if self.image.len() > ForumConfig::MAX_POST_IMAGE_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid post image: {} characters (max: {})", 
                self.image.len(), ForumConfig::MAX_POST_IMAGE_LENGTH
            )));
        }
        
        Ok(())
    }
}

/// Post burn data structure (stored in BurnMemo.payload for burn_for_post)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PostBurnData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "forum" for memo-forum contract)
    pub category: String,
    
    /// Operation type (must be "burn_for_post" for burning tokens)
    pub operation: String,
    
    /// User pubkey as string (must match the transaction signer)
    pub user: String,
    
    /// Post ID being replied to
    pub post_id: u64,
    
    /// Reply message (optional, max 512 characters)
    pub message: String,
}

impl PostBurnData {
    /// Create new post burn data
    pub fn new(
        user: String,
        post_id: u64,
        message: String,
    ) -> Self {
        Self {
            version: POST_BURN_DATA_VERSION,
            category: "forum".to_string(),
            operation: "burn_for_post".to_string(),
            user,
            post_id,
            message,
        }
    }
    
    /// Validate the post burn data
    pub fn validate(&self, expected_user: &str, expected_post_id: u64) -> Result<(), RpcError> {
        // Validate version
        if self.version != POST_BURN_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported post burn data version: {} (expected: {})", 
                self.version, POST_BURN_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "forum" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'forum')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "burn_for_post" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'burn_for_post')", self.operation
            )));
        }
        
        // Validate user
        if self.user != expected_user {
            return Err(RpcError::InvalidParameter(format!(
                "User mismatch: data contains {}, expected {}", 
                self.user, expected_user
            )));
        }
        
        // Validate post ID
        if self.post_id != expected_post_id {
            return Err(RpcError::InvalidParameter(format!(
                "Post ID mismatch: data contains {}, expected {}", 
                self.post_id, expected_post_id
            )));
        }
        
        // Validate message (optional, max 512 characters)
        if self.message.len() > ForumConfig::MAX_REPLY_MESSAGE_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Reply message too long: {} characters (max: {})", 
                self.message.len(), ForumConfig::MAX_REPLY_MESSAGE_LENGTH
            )));
        }
        
        Ok(())
    }
}

/// Post mint data structure (stored in BurnMemo.payload for mint_for_post)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PostMintData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "forum" for memo-forum contract)
    pub category: String,
    
    /// Operation type (must be "mint_for_post" for minting tokens)
    pub operation: String,
    
    /// User pubkey as string (must match the transaction signer)
    pub user: String,
    
    /// Post ID being replied to
    pub post_id: u64,
    
    /// Reply message (optional, max 512 characters)
    pub message: String,
}

impl PostMintData {
    /// Create new post mint data
    pub fn new(
        user: String,
        post_id: u64,
        message: String,
    ) -> Self {
        Self {
            version: POST_MINT_DATA_VERSION,
            category: "forum".to_string(),
            operation: "mint_for_post".to_string(),
            user,
            post_id,
            message,
        }
    }
    
    /// Validate the post mint data
    pub fn validate(&self, expected_user: &str, expected_post_id: u64) -> Result<(), RpcError> {
        // Validate version
        if self.version != POST_MINT_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported post mint data version: {} (expected: {})", 
                self.version, POST_MINT_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "forum" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'forum')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "mint_for_post" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'mint_for_post')", self.operation
            )));
        }
        
        // Validate user
        if self.user != expected_user {
            return Err(RpcError::InvalidParameter(format!(
                "User mismatch: data contains {}, expected {}", 
                self.user, expected_user
            )));
        }
        
        // Validate post ID
        if self.post_id != expected_post_id {
            return Err(RpcError::InvalidParameter(format!(
                "Post ID mismatch: data contains {}, expected {}", 
                self.post_id, expected_post_id
            )));
        }
        
        // Validate message (optional, max 512 characters)
        if self.message.len() > ForumConfig::MAX_REPLY_MESSAGE_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Reply message too long: {} characters (max: {})", 
                self.message.len(), ForumConfig::MAX_REPLY_MESSAGE_LENGTH
            )));
        }
        
        Ok(())
    }
}

/// Represents global forum statistics from the memo-forum contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumGlobalStatistics {
    pub total_posts: u64,
}

/// Represents a post's information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostInfo {
    pub post_id: u64,
    pub creator: String,  // Base58 encoded pubkey
    pub created_at: i64,
    pub last_updated: i64,
    pub title: String,
    pub content: String,
    pub image: String,
    pub reply_count: u64,
    pub burned_amount: u64,
    pub last_reply_time: i64,
    pub bump: u8,
}

/// Summary statistics for all posts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumStatistics {
    pub total_posts: u64,
    pub valid_posts: u64,
    pub total_replies: u64,
    pub total_burned_tokens: u64,
    pub posts: Vec<PostInfo>,
}

/// Represents a single reply to a post (burn or mint)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostReply {
    pub signature: String,      // Transaction signature
    pub user: String,           // User's public key
    pub message: String,        // The reply message
    pub timestamp: i64,         // Block time
    pub slot: u64,             // Slot number
    pub burn_amount: u64,      // Amount burned (0 for mint operations)
    pub is_mint: bool,         // True if this is a mint operation
}

/// Response containing replies for a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostRepliesResponse {
    pub post_id: u64,
    pub replies: Vec<PostReply>,
    pub total_found: usize,
    pub has_more: bool,        // Indicates if there are more replies available
}

/// Operation type for forum contract transactions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForumOperationType {
    CreatePost,
    BurnForPost,
    MintForPost,
}

/// Detailed information for different operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForumOperationDetails {
    /// Create post operation with full post details
    Create {
        post_id: u64,
        title: String,
        content: String,
        image: String,
    },
    /// Burn for post operation with message
    Burn {
        post_id: u64,
        message: String,
    },
    /// Mint for post operation with message
    Mint {
        post_id: u64,
        message: String,
    },
}

/// Transaction info for forum contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumContractTransaction {
    pub signature: String,          // Transaction signature
    pub user: String,               // User's public key
    pub timestamp: i64,             // Block time
    pub slot: u64,                  // Slot number
    pub burn_amount: u64,           // Amount burned (0 for mint operations)
    pub operation_type: ForumOperationType,  // Type of operation
    pub details: ForumOperationDetails,      // Operation-specific details
}

/// Response containing recent transactions for forum contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForumContractTransactionsResponse {
    pub transactions: Vec<ForumContractTransaction>,
    pub total_found: usize,
}

/// Parse Base64+Borsh-formatted memo data to extract post reply (burn or mint)
fn parse_borsh_post_reply_message(memo_data: &[u8]) -> Option<(String, String, u64, bool)> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to BurnMemo
    match BurnMemo::try_from_slice(&borsh_bytes) {
        Ok(burn_memo) => {
            // Try to deserialize as PostBurnData
            if let Ok(burn_data) = PostBurnData::try_from_slice(&burn_memo.payload) {
                if burn_data.category == "forum" && burn_data.operation == "burn_for_post" {
                    return Some((burn_data.user, burn_data.message, burn_memo.burn_amount, false));
                }
            }
            
            // Try to deserialize as PostMintData
            if let Ok(mint_data) = PostMintData::try_from_slice(&burn_memo.payload) {
                if mint_data.category == "forum" && mint_data.operation == "mint_for_post" {
                    return Some((mint_data.user, mint_data.message, 0, true));
                }
            }
            
            None
        },
        Err(_) => None
    }
}

/// Parse memo data for all forum operations (create, burn, mint)
/// Returns (user, operation_type, details, burn_amount)
fn parse_forum_operation_memo(memo_data: &[u8]) -> Option<(String, ForumOperationType, ForumOperationDetails, u64)> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to BurnMemo
    let burn_memo = BurnMemo::try_from_slice(&borsh_bytes).ok()?;
    let burn_amount = burn_memo.burn_amount;
    
    // Try to parse as PostCreationData
    if let Ok(creation_data) = PostCreationData::try_from_slice(&burn_memo.payload) {
        if creation_data.category == "forum" && creation_data.operation == "create_post" {
            return Some((
                creation_data.creator.clone(),
                ForumOperationType::CreatePost,
                ForumOperationDetails::Create {
                    post_id: creation_data.post_id,
                    title: creation_data.title,
                    content: creation_data.content,
                    image: creation_data.image,
                },
                burn_amount,
            ));
        }
    }
    
    // Try to parse as PostBurnData
    if let Ok(burn_data) = PostBurnData::try_from_slice(&burn_memo.payload) {
        if burn_data.category == "forum" && burn_data.operation == "burn_for_post" {
            return Some((
                burn_data.user.clone(),
                ForumOperationType::BurnForPost,
                ForumOperationDetails::Burn {
                    post_id: burn_data.post_id,
                    message: burn_data.message,
                },
                burn_amount,
            ));
        }
    }
    
    // Try to parse as PostMintData
    if let Ok(mint_data) = PostMintData::try_from_slice(&burn_memo.payload) {
        if mint_data.category == "forum" && mint_data.operation == "mint_for_post" {
            return Some((
                mint_data.user.clone(),
                ForumOperationType::MintForPost,
                ForumOperationDetails::Mint {
                    post_id: mint_data.post_id,
                    message: mint_data.message,
                },
                0, // burn_amount is 0 for mint operations
            ));
        }
    }
    
    None
}

impl RpcConnection {
    /// Build an unsigned transaction to create a forum post
    pub async fn build_create_post_transaction(
        &self,
        user_pubkey: &Pubkey,
        title: &str,
        content: &str,
        image: &str,
        burn_amount: u64,
    ) -> Result<(Transaction, u64), RpcError> {
        // Basic parameter validation
        if title.is_empty() || title.len() > ForumConfig::MAX_POST_TITLE_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Post title must be 1-{} characters, got {}", 
                ForumConfig::MAX_POST_TITLE_LENGTH, title.len()
            )));
        }
        if content.is_empty() || content.len() > ForumConfig::MAX_POST_CONTENT_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Post content must be 1-{} characters, got {}", 
                ForumConfig::MAX_POST_CONTENT_LENGTH, content.len()
            )));
        }
        if image.len() > ForumConfig::MAX_POST_IMAGE_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Post image must be at most {} characters, got {}", 
                ForumConfig::MAX_POST_IMAGE_LENGTH, image.len()
            )));
        }
        if burn_amount < ForumConfig::MIN_POST_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!(
                "Burn amount must be at least {} MEMO tokens", 
                ForumConfig::MIN_POST_BURN_AMOUNT / 1_000_000
            )));
        }
        if burn_amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens".to_string()));
        }
        
        log::info!("Building create post transaction '{}': {} tokens", title, burn_amount / 1_000_000);
        
        // Get next post_id
        let global_stats = self.get_forum_global_statistics().await?;
        let expected_post_id = global_stats.total_posts;
        
        let forum_program_id = ForumConfig::get_program_id()?;
        let memo_token_mint = ForumConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_burn_program_id = ForumConfig::get_memo_burn_program_id()?;
        
        let (global_counter_pda, _) = ForumConfig::get_global_counter_pda()?;
        let (post_pda, _) = ForumConfig::get_post_pda(expected_post_id)?;
        let (user_global_burn_stats_pda, _) = ForumConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey, &memo_token_mint, &token_2022_program_id,
        );
        
        let post_creation_data = PostCreationData::new(
            user_pubkey.to_string(),
            expected_post_id,
            title.to_string(),
            content.to_string(),
            image.to_string(),
        );
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload: post_creation_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize post data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        validate_memo_length_bytes(memo_data_base64.as_bytes())?;
        
        // Build base instructions (without compute budget)
        let mut base_instructions = vec![];
        
        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(memo_data_base64.as_bytes(), &[user_pubkey]));
        
        // Create post instruction
        let mut instruction_data = ForumConfig::get_create_post_discriminator().to_vec();
        instruction_data.extend_from_slice(&expected_post_id.to_le_bytes());
        instruction_data.extend_from_slice(&burn_amount.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            forum_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(global_counter_pda, false),
                AccountMeta::new(post_pda, false),
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
        
        log::info!("Simulating create post transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Create post simulation consumed {} compute units", units_consumed);
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
        
        Ok((transaction, expected_post_id))
    }

    /// Build an unsigned transaction to burn tokens for a post (reply with burn)
    pub async fn build_burn_for_post_transaction(
        &self,
        user_pubkey: &Pubkey,
        post_id: u64,
        amount: u64,
        message: &str,
    ) -> Result<Transaction, RpcError> {
        // Validate amount
        if amount < ForumConfig::MIN_POST_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!(
                "Burn amount must be at least {} MEMO tokens", 
                ForumConfig::MIN_POST_BURN_AMOUNT / 1_000_000
            )));
        }
        
        if amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter(
                "Burn amount must be a whole number of tokens".to_string()
            ));
        }
        
        // Validate message length
        if message.len() > ForumConfig::MAX_REPLY_MESSAGE_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Reply message too long: {} characters (max: {})", 
                message.len(), ForumConfig::MAX_REPLY_MESSAGE_LENGTH
            )));
        }
        
        log::info!("Building burn for post transaction: {} tokens for post {}", 
                  amount / 1_000_000, post_id);
        
        let forum_program_id = ForumConfig::get_program_id()?;
        let memo_token_mint = ForumConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_burn_program_id = ForumConfig::get_memo_burn_program_id()?;
        
        let (post_pda, _) = ForumConfig::get_post_pda(post_id)?;
        let (user_global_burn_stats_pda, _) = ForumConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        let burn_data = PostBurnData::new(user_pubkey.to_string(), post_id, message.to_string());
        burn_data.validate(&user_pubkey.to_string(), post_id)?;
        
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
        let mut instruction_data = ForumConfig::get_burn_for_post_discriminator().to_vec();
        instruction_data.extend_from_slice(&post_id.to_le_bytes());
        instruction_data.extend_from_slice(&amount.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            forum_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(post_pda, false),
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
        
        log::info!("Simulating burn for post transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Burn for post simulation consumed {} compute units", units_consumed);
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

    /// Build an unsigned transaction to mint tokens for a post (reply with mint)
    pub async fn build_mint_for_post_transaction(
        &self,
        user_pubkey: &Pubkey,
        post_id: u64,
        message: &str,
    ) -> Result<Transaction, RpcError> {
        // Validate message length
        if message.len() > ForumConfig::MAX_REPLY_MESSAGE_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Reply message too long: {} characters (max: {})", 
                message.len(), ForumConfig::MAX_REPLY_MESSAGE_LENGTH
            )));
        }
        
        log::info!("Building mint for post transaction for post {}", post_id);
        
        let forum_program_id = ForumConfig::get_program_id()?;
        let memo_token_mint = ForumConfig::get_memo_token_mint()?;
        let token_2022_program_id = get_token_2022_program_id()?;
        let memo_mint_program_id = ForumConfig::get_memo_mint_program_id()?;
        
        let (post_pda, _) = ForumConfig::get_post_pda(post_id)?;
        let (mint_authority_pda, _) = ForumConfig::get_mint_authority_pda()?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        let mint_data = PostMintData::new(user_pubkey.to_string(), post_id, message.to_string());
        mint_data.validate(&user_pubkey.to_string(), post_id)?;
        
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
        let mut instruction_data = ForumConfig::get_mint_for_post_discriminator().to_vec();
        instruction_data.extend_from_slice(&post_id.to_le_bytes());
        
        base_instructions.push(Instruction::new_with_bytes(
            forum_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(post_pda, false),
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
        
        log::info!("Simulating mint for post transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;
        
        let simulated_cu = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Mint for post simulation consumed {} compute units", units_consumed);
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

    /// Get global forum statistics from the memo-forum contract
    pub async fn get_forum_global_statistics(&self) -> Result<ForumGlobalStatistics, RpcError> {
        let (global_counter_pda, _) = ForumConfig::get_global_counter_pda()?;
        
        log::info!("Fetching global forum statistics from PDA: {}", global_counter_pda);
        
        let account_info = self.get_account_info(&global_counter_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(
                "Global forum counter not found. Please initialize the memo-forum system first.".to_string()
            ));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // Verify the account is owned by memo-forum program
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account owner".to_string()))?;
        
        let expected_program_id = ForumConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-forum program. Expected: {}, Got: {}", 
                expected_program_id, owner
            )));
        }
        
        // Parse total posts count (skip 8-byte discriminator, read next 8 bytes)
        if data.len() < 16 {
            return Err(RpcError::Other("Invalid account data size".to_string()));
        }
        
        let total_posts_bytes = &data[8..16];
        let total_posts = u64::from_le_bytes(
            total_posts_bytes.try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse total posts: {:?}", e)))?
        );
        
        log::info!("Global forum statistics: {} total posts", total_posts);
        
        Ok(ForumGlobalStatistics { total_posts })
    }
    
    /// Get information for a specific post
    pub async fn get_post_info(&self, post_id: u64) -> Result<PostInfo, RpcError> {
        let (post_pda, _) = ForumConfig::get_post_pda(post_id)?;
        
        log::info!("Fetching post {} info from PDA: {}", post_id, post_pda);
        
        let account_info = self.get_account_info(&post_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(format!("Post {} not found", post_id)));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // Verify the account is owned by memo-forum program
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account owner".to_string()))?;
        
        let expected_program_id = ForumConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-forum program. Expected: {}, Got: {}", 
                expected_program_id, owner
            )));
        }
        
        // Parse post data
        self.parse_post_data(&data)
    }
    
    /// Parse Post account data according to the contract's data structure
    fn parse_post_data(&self, data: &[u8]) -> Result<PostInfo, RpcError> {
        if data.len() < 8 {
            return Err(RpcError::Other("Data too short for discriminator".to_string()));
        }
        
        let mut offset = 8; // Skip discriminator
        
        // Read post_id (u64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for post_id".to_string()));
        }
        let post_id = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse post_id: {:?}", e)))?
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
        
        // Read title (String)
        let (title, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read content (String)
        let (content, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read image (String)
        let (image, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read reply_count (u64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for reply_count".to_string()));
        }
        let reply_count = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse reply_count: {:?}", e)))?
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
        
        // Read last_reply_time (i64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for last_reply_time".to_string()));
        }
        let last_reply_time = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse last_reply_time: {:?}", e)))?
        );
        offset += 8;
        
        // Read bump (u8)
        if data.len() < offset + 1 {
            return Err(RpcError::Other("Data too short for bump".to_string()));
        }
        let bump = data[offset];
        
        Ok(PostInfo {
            post_id,
            creator,
            created_at,
            last_updated,
            title,
            content,
            image,
            reply_count,
            burned_amount,
            last_reply_time,
            bump,
        })
    }
    
    /// Get all posts with statistics
    pub async fn get_all_forum_posts(&self) -> Result<ForumStatistics, RpcError> {
        log::info!("Fetching all forum posts...");
        
        // First get global statistics
        let global_stats = self.get_forum_global_statistics().await?;
        let total_posts = global_stats.total_posts;
        
        if total_posts == 0 {
            log::info!("No posts found");
            return Ok(ForumStatistics {
                total_posts: 0,
                valid_posts: 0,
                total_replies: 0,
                total_burned_tokens: 0,
                posts: vec![],
            });
        }
        
        let mut valid_posts = 0;
        let mut total_replies = 0;
        let mut total_burned_tokens: u64 = 0;
        let mut posts = Vec::new();
        
        // Iterate through all posts
        for post_id in 0..total_posts {
            match self.get_post_info(post_id).await {
                Ok(post_info) => {
                    valid_posts += 1;
                    total_replies += post_info.reply_count;
                    total_burned_tokens += post_info.burned_amount;
                    posts.push(post_info);
                    
                    log::info!("Successfully fetched post {}", post_id);
                },
                Err(e) => {
                    log::warn!("Failed to fetch post {}: {}", post_id, e);
                }
            }
        }
        
        log::info!("Forum statistics summary: {}/{} valid posts, {} total replies, {} total burned tokens", 
                  valid_posts, total_posts, total_replies, total_burned_tokens);
        
        Ok(ForumStatistics {
            total_posts,
            valid_posts,
            total_replies,
            total_burned_tokens,
            posts,
        })
    }

    /// Check if a specific post exists
    pub async fn post_exists(&self, post_id: u64) -> Result<bool, RpcError> {
        match self.get_post_info(post_id).await {
            Ok(_) => Ok(true),
            Err(RpcError::Other(msg)) if msg.contains("not found") => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get replies for a post with pagination support
    pub async fn get_post_replies(
        &self,
        post_id: u64,
        limit: usize,
        before: Option<String>,
    ) -> Result<PostRepliesResponse, RpcError> {
        // Validate parameters
        if limit == 0 || limit > 1000 {
            return Err(RpcError::InvalidParameter("Limit must be between 1 and 1000".to_string()));
        }
        
        // Get post PDA
        let (post_pda, _) = ForumConfig::get_post_pda(post_id)?;
        
        // Check if post exists
        if !self.post_exists(post_id).await? {
            return Err(RpcError::Other(format!("Post {} not found", post_id)));
        }
        
        // Get signatures for the post account
        let mut params = serde_json::json!([
            post_pda.to_string(),
            {
                "limit": limit,
                "commitment": "confirmed"
            }
        ]);
        
        // Add 'before' parameter if specified
        if let Some(before_sig) = before {
            params[1]["before"] = serde_json::Value::String(before_sig);
        }
        
        log::info!("Fetching signatures for post {}: {}", post_id, post_pda);
        
        // Get signatures for address
        let signatures_response: serde_json::Value = self.send_request("getSignaturesForAddress", params).await?;
        let signatures = signatures_response.as_array()
            .ok_or_else(|| RpcError::Other("Invalid signatures response format".to_string()))?;
        
        log::info!("Found {} signatures for post {}", signatures.len(), post_id);
        
        let mut replies = Vec::new();
        
        // Process each signature
        for sig_info in signatures {
            let signature = sig_info["signature"]
                .as_str()
                .unwrap_or("")
                .to_string();
            
            if signature.is_empty() {
                continue;
            }
            
            let block_time = sig_info["blockTime"].as_i64().unwrap_or(0);
            let slot = sig_info["slot"].as_u64().unwrap_or(0);
            
            // Extract memo field directly from signature info
            if let Some(memo_str) = sig_info["memo"].as_str() {
                let memo_data = if let Some(space_pos) = memo_str.find(' ') {
                    &memo_str[space_pos + 1..]
                } else {
                    memo_str
                };
                
                let memo_bytes = memo_data.as_bytes();
                
                // Parse memo data for post replies
                if let Some((user, message, burn_amount, is_mint)) = parse_borsh_post_reply_message(memo_bytes) {
                    replies.push(PostReply {
                        signature: signature.clone(),
                        user,
                        message,
                        timestamp: block_time,
                        slot,
                        burn_amount,
                        is_mint,
                    });
                }
            }
        }
        
        // Sort replies by timestamp from newest to oldest
        replies.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        let has_more = signatures.len() == limit;
        let total_found = replies.len();
        
        log::info!("Found {} replies for post {}", total_found, post_id);
        
        Ok(PostRepliesResponse {
            post_id,
            replies,
            total_found,
            has_more,
        })
    }

    /// Get recent transactions for the forum contract
    pub async fn get_recent_forum_contract_transactions(
        &self,
    ) -> Result<ForumContractTransactionsResponse, RpcError> {
        let program_id = ForumConfig::get_program_id()?;
        
        let params = serde_json::json!([
            program_id.to_string(),
            {
                "limit": 3,
                "commitment": "confirmed"
            }
        ]);
        
        log::info!("Fetching recent transactions for forum contract: {}", program_id);
        
        let signatures_response: serde_json::Value = self.send_request("getSignaturesForAddress", params).await?;
        let signatures = signatures_response.as_array()
            .ok_or_else(|| RpcError::Other("Invalid signatures response format".to_string()))?;
        
        log::info!("Found {} recent signatures for forum contract", signatures.len());
        
        let mut transactions = Vec::new();
        
        for sig_info in signatures {
            let signature = sig_info["signature"]
                .as_str()
                .unwrap_or("")
                .to_string();
            
            if signature.is_empty() {
                continue;
            }
            
            let block_time = sig_info["blockTime"].as_i64().unwrap_or(0);
            let slot = sig_info["slot"].as_u64().unwrap_or(0);
            
            if let Some(memo_str) = sig_info["memo"].as_str() {
                let memo_data = if let Some(space_pos) = memo_str.find(' ') {
                    &memo_str[space_pos + 1..]
                } else {
                    memo_str
                };
                
                let memo_bytes = memo_data.as_bytes();
                
                if let Some((user, operation_type, details, burn_amount)) = parse_forum_operation_memo(memo_bytes) {
                    transactions.push(ForumContractTransaction {
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
        
        // Sort by timestamp from newest to oldest
        transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        let total_found = transactions.len();
        
        log::info!("Found {} recent transactions for forum contract", total_found);
        
        Ok(ForumContractTransactionsResponse {
            transactions,
            total_found,
        })
    }
}
