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
use sha2::{Sha256, Digest};
use base64;
use bincode;
use wasm_bindgen::prelude::*;

/// Borsh serialization version constants
pub const BURN_MEMO_VERSION: u8 = 1;
pub const CHAT_GROUP_CREATION_DATA_VERSION: u8 = 1;

/// Borsh length constants
const BORSH_U8_SIZE: usize = 1;         // version (u8)
const BORSH_U64_SIZE: usize = 8;        // burn_amount (u64)
const BORSH_VEC_LENGTH_SIZE: usize = 4; // user_data.len() (u32)
const BORSH_FIXED_OVERHEAD: usize = BORSH_U8_SIZE + BORSH_U64_SIZE + BORSH_VEC_LENGTH_SIZE;

/// Memo-Chat contract configuration and constants
pub struct ChatConfig;

impl ChatConfig {
    /// Memo-chat program ID
    pub const MEMO_CHAT_PROGRAM_ID: &'static str = "54ky4LNnRsbYioDSBKNrc5hG8HoDyZ6yhf8TuncxTBRF";
    
    /// PDA Seeds for chat contract
    pub const GLOBAL_COUNTER_SEED: &'static [u8] = b"global_counter";
    pub const CHAT_GROUP_SEED: &'static [u8] = b"chat_group";
    
    /// Memo-mint program ID (referenced by chat contract)
    pub const MEMO_MINT_PROGRAM_ID: &'static str = "A31a17bhgQyRQygeZa1SybytjbCdjMpu6oPr9M3iQWzy";
    
    /// Token 2022 Program ID
    pub const TOKEN_2022_PROGRAM_ID: &'static str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    
    /// Authorized MEMO token mint address
    pub const MEMO_TOKEN_MINT: &'static str = "HLCoc7wNDavNMfWWw2Bwd7U7A24cesuhBSNkxZgvZm1";
    
    /// Memo validation limits (from contract: 69-800 bytes)
    pub const MIN_MEMO_LENGTH: usize = 69;
    pub const MAX_MEMO_LENGTH: usize = 800;
    
    /// Maximum payload length = memo maximum length - borsh fixed overhead
    pub const MAX_PAYLOAD_LENGTH: usize = Self::MAX_MEMO_LENGTH - BORSH_FIXED_OVERHEAD; // 800 - 13 = 787
    
    /// Compute budget configuration
    pub const COMPUTE_UNIT_BUFFER: f64 = 1.2; // 20% buffer for chat operations
    pub const MIN_COMPUTE_UNITS: u64 = 120_000; // Min cu to 120K
    pub const MAX_COMPUTE_UNITS: u64 = 400_000; // Maximum CU limit
    
    /// Helper functions
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MEMO_CHAT_PROGRAM_ID)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-chat program ID: {}", e)))
    }
    
    /// Calculate global counter PDA
    pub fn get_global_counter_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::GLOBAL_COUNTER_SEED],
            &program_id
        ))
    }
    
    /// Calculate chat group PDA for a specific group ID
    pub fn get_chat_group_pda(group_id: u64) -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::CHAT_GROUP_SEED, &group_id.to_le_bytes()],
            &program_id
        ))
    }
    
    /// Helper to get memo-mint program ID
    pub fn get_memo_mint_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MEMO_MINT_PROGRAM_ID)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-mint program ID: {}", e)))
    }
    
    /// Helper to get token mint
    pub fn get_memo_token_mint() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::MEMO_TOKEN_MINT)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo token mint: {}", e)))
    }
    
    /// Helper to get Token 2022 program ID
    pub fn get_token_2022_program_id() -> Result<Pubkey, RpcError> {
        Pubkey::from_str(Self::TOKEN_2022_PROGRAM_ID)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid Token 2022 program ID: {}", e)))
    }
    
    /// Calculate mint authority PDA (from memo-mint program)
    pub fn get_mint_authority_pda() -> Result<(Pubkey, u8), RpcError> {
        let memo_mint_program_id = Self::get_memo_mint_program_id()?;
        Ok(Pubkey::find_program_address(
            &[b"mint_authority"],
            &memo_mint_program_id
        ))
    }
    
    /// Get send_memo_to_group instruction discriminator
    pub fn get_send_memo_to_group_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:send_memo_to_group");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }
    
    /// Validate memo length for binary data
    pub fn validate_memo_length(memo_data: &[u8]) -> Result<(), RpcError> {
        let memo_len = memo_data.len();
        if memo_len < Self::MIN_MEMO_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Memo too short: {} bytes (minimum: {})", 
                memo_len, Self::MIN_MEMO_LENGTH
            )));
        }
        if memo_len > Self::MAX_MEMO_LENGTH {
            return Err(RpcError::InvalidParameter(format!(
                "Memo too long: {} bytes (maximum: {})", 
                memo_len, Self::MAX_MEMO_LENGTH
            )));
        }
        Ok(())
    }
}

/// BurnMemo structure (compatible with contract)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BurnMemo {
    /// Version of the BurnMemo structure (for future compatibility)
    pub version: u8,
    
    /// Burn amount (must match actual burn amount)
    pub burn_amount: u64,
    
    /// Application payload (variable length, max 787 bytes)
    pub payload: Vec<u8>,
}

/// Chat message data structure (stored in BurnMemo.payload for send_memo_to_group)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ChatMessageData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "chat" for memo-chat contract)
    pub category: String,
    
    /// Operation type (must be "send_message" for sending messages)
    pub operation: String,
    
    /// Group ID (must match the target group)
    pub group_id: u64,
    
    /// Sender pubkey as string (must match the transaction signer)
    pub sender: String,
    
    /// Message content (required, 1-512 characters)
    pub message: String,
    
    /// Optional receiver pubkey as string (for direct messages within group)
    pub receiver: Option<String>,
    
    /// Optional reply to signature (for message threading)
    pub reply_to_sig: Option<String>,
}

impl ChatMessageData {
    /// Create new chat message data
    pub fn new(
        group_id: u64,
        sender: String,
        message: String,
        receiver: Option<String>,
        reply_to_sig: Option<String>,
    ) -> Self {
        Self {
            version: CHAT_GROUP_CREATION_DATA_VERSION,
            category: "chat".to_string(),
            operation: "send_message".to_string(),
            group_id,
            sender,
            message,
            receiver,
            reply_to_sig,
        }
    }
    
    /// Validate message data
    pub fn validate(&self, expected_group_id: u64, expected_sender: &str) -> Result<(), RpcError> {
        // Validate version
        if self.version != CHAT_GROUP_CREATION_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported chat message data version: {} (expected: {})", 
                self.version, CHAT_GROUP_CREATION_DATA_VERSION
            )));
        }
        
        // Validate category
        if self.category != "chat" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid category: '{}' (expected: 'chat')", self.category
            )));
        }
        
        // Validate operation
        if self.operation != "send_message" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'send_message')", self.operation
            )));
        }
        
        // Validate group ID
        if self.group_id != expected_group_id {
            return Err(RpcError::InvalidParameter(format!(
                "Group ID mismatch: data contains {}, expected {}", 
                self.group_id, expected_group_id
            )));
        }
        
        // Validate sender
        if self.sender != expected_sender {
            return Err(RpcError::InvalidParameter(format!(
                "Sender mismatch: data contains {}, expected {}", 
                self.sender, expected_sender
            )));
        }
        
        // Validate message
        if self.message.is_empty() {
            return Err(RpcError::InvalidParameter("Message cannot be empty".to_string()));
        }
        
        if self.message.len() > 512 {
            return Err(RpcError::InvalidParameter("Message too long (max 512 characters)".to_string()));
        }
        
        Ok(())
    }
}

/// Parse Base64+Borsh-formatted memo data to extract chat message
fn parse_borsh_chat_message(memo_data: &[u8]) -> Option<(String, String)> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to ChatMessageData
    match ChatMessageData::try_from_slice(&borsh_bytes) {
        Ok(chat_data) => {
            // Validate category and operation
            if chat_data.category == "chat" && chat_data.operation == "send_message" {
                Some((chat_data.sender, chat_data.message))
            } else {
                None
            }
        },
        Err(_) => None
    }
}

/// Represents global statistics from the memo-chat contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStatistics {
    pub total_groups: u64,
}

/// Represents a chat group's information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGroupInfo {
    pub group_id: u64,
    pub creator: String,  // Base58 encoded pubkey
    pub created_at: i64,
    pub name: String,
    pub description: String,
    pub image: String,
    pub tags: Vec<String>,
    pub memo_count: u64,
    pub burned_amount: u64,
    pub min_memo_interval: i64,
    pub last_memo_time: i64,
    pub bump: u8,
}

/// Summary statistics for all chat groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStatistics {
    pub total_groups: u64,
    pub valid_groups: u64,
    pub total_memos: u64,
    pub total_burned_tokens: u64,
    pub groups: Vec<ChatGroupInfo>,
}

/// Represents a single chat message/memo in a group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub signature: String,      // Transaction signature
    pub sender: String,         // Sender's public key
    pub message: String,        // The memo content
    pub timestamp: i64,         // Block time
    pub slot: u64,             // Slot number
    pub memo_amount: u64,      // Amount of MEMO tokens burned for this message
}

/// Response containing chat messages for a group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessagesResponse {
    pub group_id: u64,
    pub messages: Vec<ChatMessage>,
    pub total_found: usize,
    pub has_more: bool,        // Indicates if there are more messages available
}

/// Local message status for UI display
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageStatus {
    Sending,
    Sent,
    Failed,
    Timeout,
}

/// Custom error type that includes timeout
#[derive(Debug)]
pub enum ChatError {
    Rpc(RpcError),
    Timeout,
}

impl From<RpcError> for ChatError {
    fn from(error: RpcError) -> Self {
        ChatError::Rpc(error)
    }
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatError::Rpc(e) => write!(f, "RPC Error: {}", e),
            ChatError::Timeout => write!(f, "Request timeout"),
        }
    }
}

impl ChatError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, ChatError::Timeout)
    }
}

/// Local message for immediate UI display
#[derive(Debug, Clone)]
pub struct LocalChatMessage {
    pub message: ChatMessage,
    pub status: MessageStatus,
    pub is_local: bool, // true if this is a local message not yet confirmed on chain
}

impl LocalChatMessage {
    /// Create a new local message for immediate UI display
    pub fn new_local(sender: String, message: String, group_id: u64) -> Self {
        Self {
            message: ChatMessage {
                signature: format!("local_{}", js_sys::Date::now() as u64), // temporary local signature
                sender,
                message,
                timestamp: (js_sys::Date::now() / 1000.0) as i64, // current timestamp
                slot: 0,
                memo_amount: 0,
            },
            status: MessageStatus::Sending,
            is_local: true,
        }
    }
    
    /// Create from chain message
    pub fn from_chain_message(message: ChatMessage) -> Self {
        Self {
            message,
            status: MessageStatus::Sent,
            is_local: false,
        }
    }
    
    /// Update status
    pub fn with_status(mut self, status: MessageStatus) -> Self {
        self.status = status;
        self
    }
}

impl RpcConnection {
    /// Get global chat statistics from the memo-chat contract
    /// 
    /// # Returns
    /// Global statistics including total number of groups created
    pub async fn get_chat_global_statistics(&self) -> Result<GlobalStatistics, RpcError> {
        let (global_counter_pda, _) = ChatConfig::get_global_counter_pda()?;
        
        log::info!("Fetching global chat statistics from PDA: {}", global_counter_pda);
        
        let account_info = self.get_account_info(&global_counter_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(
                "Global counter not found. Please initialize the memo-chat system first.".to_string()
            ));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // Verify the account is owned by memo-chat program
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account owner".to_string()))?;
        
        if owner != ChatConfig::MEMO_CHAT_PROGRAM_ID {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-chat program. Expected: {}, Got: {}", 
                ChatConfig::MEMO_CHAT_PROGRAM_ID, owner
            )));
        }
        
        // Parse total groups count (skip 8-byte discriminator, read next 8 bytes)
        if data.len() < 16 {
            return Err(RpcError::Other("Invalid account data size".to_string()));
        }
        
        let total_groups_bytes = &data[8..16];
        let total_groups = u64::from_le_bytes(
            total_groups_bytes.try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse total groups: {:?}", e)))?
        );
        
        log::info!("Global chat statistics: {} total groups", total_groups);
        
        Ok(GlobalStatistics { total_groups })
    }
    
    /// Get information for a specific chat group
    /// 
    /// # Parameters
    /// * `group_id` - The ID of the chat group to fetch
    /// 
    /// # Returns
    /// Chat group information if it exists
    pub async fn get_chat_group_info(&self, group_id: u64) -> Result<ChatGroupInfo, RpcError> {
        let (chat_group_pda, _) = ChatConfig::get_chat_group_pda(group_id)?;
        
        log::info!("Fetching chat group {} info from PDA: {}", group_id, chat_group_pda);
        
        let account_info = self.get_account_info(&chat_group_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(format!("Chat group {} not found", group_id)));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode account data: {}", e)))?;
        
        // Verify the account is owned by memo-chat program
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get account owner".to_string()))?;
        
        if owner != ChatConfig::MEMO_CHAT_PROGRAM_ID {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-chat program. Expected: {}, Got: {}", 
                ChatConfig::MEMO_CHAT_PROGRAM_ID, owner
            )));
        }
        
        // Parse chat group data
        self.parse_chat_group_data(&data)
    }
    
    /// Get comprehensive statistics for all chat groups
    /// 
    /// # Returns
    /// Complete statistics including all group information
    pub async fn get_all_chat_statistics(&self) -> Result<ChatStatistics, RpcError> {
        log::info!("Fetching comprehensive chat statistics...");
        
        // First get global statistics
        let global_stats = self.get_chat_global_statistics().await?;
        let total_groups = global_stats.total_groups;
        
        if total_groups == 0 {
            log::info!("No chat groups found");
            return Ok(ChatStatistics {
                total_groups: 0,
                valid_groups: 0,
                total_memos: 0,
                total_burned_tokens: 0,
                groups: vec![],
            });
        }
        
        let mut valid_groups = 0;
        let mut total_memos = 0;
        let mut total_burned_tokens = 0;
        let mut groups = Vec::new();
        
        // Iterate through all groups
        for group_id in 0..total_groups {
            match self.get_chat_group_info(group_id).await {
                Ok(group_info) => {
                    valid_groups += 1;
                    total_memos += group_info.memo_count;
                    total_burned_tokens += group_info.burned_amount;
                    groups.push(group_info);
                    
                    log::info!("Successfully fetched group {}", group_id);
                },
                Err(e) => {
                    log::warn!("Failed to fetch group {}: {}", group_id, e);
                }
            }
        }
        
        log::info!("Chat statistics summary: {}/{} valid groups, {} total memos, {} total burned tokens", 
                  valid_groups, total_groups, total_memos, total_burned_tokens);
        
        Ok(ChatStatistics {
            total_groups,
            valid_groups,
            total_memos,
            total_burned_tokens,
            groups,
        })
    }
    
    /// Parse ChatGroup account data according to the contract's data structure
    fn parse_chat_group_data(&self, data: &[u8]) -> Result<ChatGroupInfo, RpcError> {
        if data.len() < 8 {
            return Err(RpcError::Other("Data too short for discriminator".to_string()));
        }
        
        let mut offset = 8; // Skip discriminator
        
        // Read group_id (u64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for group_id".to_string()));
        }
        let group_id = u64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse group_id: {:?}", e)))?
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
        
        // Read name (String)
        let (name, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read description (String)
        let (description, new_offset) = self.read_string_from_data(data, offset)?;
        offset = new_offset;
        
        // Read image (String)
        let (image, new_offset) = self.read_string_from_data(data, offset)?;
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
        
        // Read min_memo_interval (i64)
        if data.len() < offset + 8 {
            return Err(RpcError::Other("Data too short for min_memo_interval".to_string()));
        }
        let min_memo_interval = i64::from_le_bytes(
            data[offset..offset + 8].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse min_memo_interval: {:?}", e)))?
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
        
        Ok(ChatGroupInfo {
            group_id,
            creator,
            created_at,
            name,
            description,
            image,
            tags,
            memo_count,
            burned_amount,
            min_memo_interval,
            last_memo_time,
            bump,
        })
    }
    
    /// Helper function to read a String from account data
    fn read_string_from_data(&self, data: &[u8], offset: usize) -> Result<(String, usize), RpcError> {
        if data.len() < offset + 4 {
            return Err(RpcError::Other("Data too short for string length".to_string()));
        }
        
        let len = u32::from_le_bytes(
            data[offset..offset + 4].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse string length: {:?}", e)))?
        ) as usize;
        let new_offset = offset + 4;
        
        if data.len() < new_offset + len {
            return Err(RpcError::Other("Data too short for string content".to_string()));
        }
        
        let string_data = &data[new_offset..new_offset + len];
        let string = String::from_utf8(string_data.to_vec())
            .map_err(|e| RpcError::Other(format!("Failed to parse string as UTF-8: {}", e)))?;
        
        Ok((string, new_offset + len))
    }
    
    /// Helper function to read a Vec<String> from account data
    fn read_string_vec_from_data(&self, data: &[u8], offset: usize) -> Result<(Vec<String>, usize), RpcError> {
        if data.len() < offset + 4 {
            return Err(RpcError::Other("Data too short for vec length".to_string()));
        }
        
        let vec_len = u32::from_le_bytes(
            data[offset..offset + 4].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse vec length: {:?}", e)))?
        ) as usize;
        let mut new_offset = offset + 4;
        let mut strings = Vec::new();
        
        for _ in 0..vec_len {
            let (string, next_offset) = self.read_string_from_data(data, new_offset)?;
            strings.push(string);
            new_offset = next_offset;
        }
        
        Ok((strings, new_offset))
    }
    
    /// Check if a specific chat group exists
    /// 
    /// # Parameters
    /// * `group_id` - The ID of the chat group to check
    /// 
    /// # Returns
    /// True if the group exists, false otherwise
    pub async fn chat_group_exists(&self, group_id: u64) -> Result<bool, RpcError> {
        match self.get_chat_group_info(group_id).await {
            Ok(_) => Ok(true),
            Err(RpcError::Other(msg)) if msg.contains("not found") => Ok(false),
            Err(e) => Err(e),
        }
    }
    
    /// Get the total number of chat groups that have been created
    /// 
    /// # Returns
    /// The total number of groups from the global counter
    pub async fn get_total_chat_groups(&self) -> Result<u64, RpcError> {
        let stats = self.get_chat_global_statistics().await?;
        Ok(stats.total_groups)
    }
    
    /// Get chat groups within a specific range
    /// 
    /// # Parameters
    /// * `start_id` - Starting group ID (inclusive)
    /// * `end_id` - Ending group ID (exclusive)
    /// 
    /// # Returns
    /// Vector of chat group information for existing groups in the range
    pub async fn get_chat_groups_range(&self, start_id: u64, end_id: u64) -> Result<Vec<ChatGroupInfo>, RpcError> {
        if start_id >= end_id {
            return Err(RpcError::InvalidParameter("start_id must be less than end_id".to_string()));
        }
        
        let mut groups = Vec::new();
        
        for group_id in start_id..end_id {
            match self.get_chat_group_info(group_id).await {
                Ok(group_info) => groups.push(group_info),
                Err(RpcError::Other(msg)) if msg.contains("not found") => {
                    log::debug!("Group {} not found, skipping", group_id);
                },
                Err(e) => {
                    log::warn!("Failed to fetch group {}: {}", group_id, e);
                }
            }
        }
        
        Ok(groups)
    }
    
    /// Get chat messages for a specific group (using Borsh format parsing)
    /// 
    /// # Parameters
    /// * `group_id` - The ID of the chat group
    /// * `limit` - Maximum number of messages to fetch (default: 50)
    /// * `before` - Optional signature to fetch messages before this one (for pagination)
    /// 
    /// # Returns
    /// Chat messages for the group, ordered from oldest to newest
    pub async fn get_chat_messages(
        &self,
        group_id: u64,
        limit: Option<usize>,
        before: Option<String>,
    ) -> Result<ChatMessagesResponse, RpcError> {
        let limit = limit.unwrap_or(50).min(1000); // Maximum limit 1000
        
        log::info!("Fetching chat messages for group {}, limit: {}", group_id, limit);
        
        // Get chat group PDA
        let (chat_group_pda, _) = ChatConfig::get_chat_group_pda(group_id)?;
        
        // Build request parameters
        let mut params = serde_json::json!([
            chat_group_pda.to_string(),
            {
                "encoding": "base64",
                "commitment": "confirmed",
                "limit": limit
            }
        ]);
        
        // Add 'before' parameter if specified
        if let Some(before_sig) = before {
            params[1]["before"] = serde_json::Value::String(before_sig);
        }
        
        log::info!("Fetching signatures for address: {}", chat_group_pda);
        
        // Get signatures for address
        let signatures_response: serde_json::Value = self.send_request("getSignaturesForAddress", params).await?;
        let signatures = signatures_response.as_array()
            .ok_or_else(|| RpcError::Other("Invalid signatures response format".to_string()))?;
        
        log::info!("Found {} signatures", signatures.len());
        
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
            
            // Get transaction details  
            let tx_params = serde_json::json!([
                signature,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed",
                    "maxSupportedTransactionVersion": 0
                }
            ]);
            
            match self.send_request("getTransaction", tx_params).await {
                Ok(tx_response) => {
                    let tx_data: serde_json::Value = tx_response;
                    
                    // Extract timestamp and slot
                    let block_time = tx_data["blockTime"].as_i64().unwrap_or(0);
                    let slot = tx_data["slot"].as_u64().unwrap_or(0);
                    
                    // Look for memo instruction at index 1
                    if let Some(instructions) = tx_data["transaction"]["message"]["instructions"].as_array() {
                        // Check if index 1 is a memo instruction
                        if let Some(instruction) = instructions.get(1) {
                            // In jsonParsed mode, check if this is a parsed memo instruction
                            if let Some(program) = instruction["program"].as_str() {
                                if program == "spl-memo" {
                                    // Get the parsed memo data directly
                                    if let Some(parsed) = instruction["parsed"].as_str() {
                                        // Convert string to bytes for parsing
                                        let memo_bytes = parsed.as_bytes();
                                        
                                        // Parse Base64+Borsh format message
                                        if let Some((sender, message)) = parse_borsh_chat_message(memo_bytes) {
                                            // Skip empty messages
                                            if !message.trim().is_empty() {
                                                messages.push(ChatMessage {
                                                    signature: signature.clone(),
                                                    sender,
                                                    message,
                                                    timestamp: block_time,
                                                    slot,
                                                    memo_amount: 0,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    log::debug!("Failed to get transaction details {}: {}", signature, e);
                }
            }
        }
        
        // Sort messages by timestamp from oldest to newest (ascending order)
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        let has_more = signatures.len() == limit;
        let total_found = messages.len();
        
        log::info!("Found {} chat messages for group {}", total_found, group_id);
        
        Ok(ChatMessagesResponse {
            group_id,
            messages,
            total_found,
            has_more,
        })
    }

    /// Send a chat message to a group with timeout handling
    /// 
    /// # Parameters
    /// * `group_id` - The ID of the chat group to send message to
    /// * `message` - The message content (1-512 characters)
    /// * `keypair_bytes` - The user's keypair bytes for signing
    /// * `receiver` - Optional receiver public key for direct messages
    /// * `reply_to_sig` - Optional signature to reply to
    /// * `timeout_ms` - Timeout in milliseconds (default: 30000ms = 30s)
    /// 
    /// # Returns
    /// Result containing signature or error (RpcError::Other for timeout)
    pub async fn send_chat_message_with_timeout(
        &self,
        group_id: u64,
        message: &str,
        keypair_bytes: &[u8],
        receiver: Option<String>,
        reply_to_sig: Option<String>,
        timeout_ms: Option<u32>,
    ) -> Result<String, RpcError> {
        let timeout_duration = timeout_ms.unwrap_or(30000);
        let start_time = js_sys::Date::now();
        
        let result = self.send_chat_message_internal(group_id, message, keypair_bytes, receiver, reply_to_sig, start_time, timeout_duration).await;
        
        match result {
            Ok(signature) => Ok(signature),
            Err(e) => {
                // return timeout error only when actual network request timeout
                // if it's business error (like MemoTooFrequent), return the original error directly
                match &e {
                    RpcError::SolanaRpcError(msg) => {
                        // this is Solana contract error, return directly, don't be covered by timeout logic
                        log::error!("Chat message failed with Solana error: {}", msg);
                        Err(e)
                    },
                    RpcError::Other(msg) if msg.contains("Failed to send request") || msg.contains("Failed to parse JSON") => {
                        // this might be the actual network timeout, check the time
                        let elapsed = js_sys::Date::now() - start_time;
                        if elapsed >= timeout_duration as f64 {
                            log::warn!("Chat message send timeout after {}ms", elapsed);
                            Err(RpcError::Other(format!("Request timeout after {}ms", elapsed)))
                        } else {
                            Err(e)
                        }
                    },
                    _ => {
                        // other errors, return directly
                        Err(e)
                    }
                }
            }
        }
    }

    /// Internal implementation with timeout checking
    async fn send_chat_message_internal(
        &self,
        group_id: u64,
        message: &str,
        keypair_bytes: &[u8],
        receiver: Option<String>,
        reply_to_sig: Option<String>,
        start_time: f64,
        timeout_ms: u32,
    ) -> Result<String, RpcError> {
        // check timeout helper function - only check before actual time-consuming operation
        let check_timeout = || {
            let elapsed = js_sys::Date::now() - start_time;
            if elapsed >= timeout_ms as f64 {
                log::warn!("Timeout detected before operation: {}ms elapsed", elapsed);
                return Err(RpcError::Other(format!("Request timeout after {}ms", elapsed)));
            }
            Ok(())
        };

        // Validate message
        if message.is_empty() {
            return Err(RpcError::InvalidParameter("Message cannot be empty".to_string()));
        }
        if message.len() > 512 {
            return Err(RpcError::InvalidParameter("Message too long (max 512 characters)".to_string()));
        }
        
        check_timeout()?;
        
        log::info!("Sending chat message to group {}: {} characters", group_id, message.len());
        
        // Create keypair from bytes and get pubkey
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let user_pubkey = keypair.pubkey();
        
        log::info!("Sender pubkey: {}", user_pubkey);
        
        // Get contract configuration
        let chat_program_id = ChatConfig::get_program_id()?;
        let memo_mint_program_id = ChatConfig::get_memo_mint_program_id()?;
        let memo_token_mint = ChatConfig::get_memo_token_mint()?;
        let token_2022_program_id = ChatConfig::get_token_2022_program_id()?;
        
        // Calculate chat group PDA
        let (chat_group_pda, _) = ChatConfig::get_chat_group_pda(group_id)?;
        
        // Calculate mint authority PDA
        let (mint_authority_pda, _) = ChatConfig::get_mint_authority_pda()?;
        
        // Calculate user's token account (ATA)
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        log::info!("Chat group PDA: {}", chat_group_pda);
        log::info!("User token account: {}", user_token_account);
        log::info!("Mint authority PDA: {}", mint_authority_pda);
        
        // Check if user's token account exists
        check_timeout()?;
        let token_account_info = self.get_account_info(&user_token_account.to_string(), Some("base64")).await?;
        let token_account_info: serde_json::Value = serde_json::from_str(&token_account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse token account info: {}", e)))?;
        
        // Prepare chat message data
        let chat_message_data = ChatMessageData::new(
            group_id,
            user_pubkey.to_string(),
            message.to_string(),
            receiver,
            reply_to_sig,
        );
        
        // Validate message data
        chat_message_data.validate(group_id, &user_pubkey.to_string())?;
        
        // Serialize chat message data to Borsh binary
        let memo_data_bytes = chat_message_data.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize chat message data: {}", e)))?;

        // Encode Borsh binary data to Base64 string for UTF-8 compatibility with spl-memo
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        log::info!("Chat message data: {} bytes Borsh → {} bytes Base64", 
                  memo_data_bytes.len(), memo_data_base64.len());

        // Validate memo length (now Base64 string length)
        ChatConfig::validate_memo_length(memo_data_base64.as_bytes())?;
        
        // Build base instructions (for simulation first)
        let mut base_instructions = vec![];

        // Add memo instruction with Base64-encoded Borsh data
        base_instructions.push(spl_memo::build_memo(
            memo_data_base64.as_bytes(), // UTF-8 Base64 string
            &[&user_pubkey],
        ));
        
        // If token account doesn't exist, create it
        if token_account_info["value"].is_null() {
            log::info!("User token account does not exist, creating it...");
            base_instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                    &user_pubkey,           // Funding account (fee payer)
                    &user_pubkey,           // Wallet address  
                    &memo_token_mint,       // Mint address
                    &token_2022_program_id  // Token 2022 program ID
                )
            );
        }
        
        // Create send_memo_to_group instruction
        let mut instruction_data = ChatConfig::get_send_memo_to_group_discriminator().to_vec();
        instruction_data.extend_from_slice(&group_id.to_le_bytes());
        
        let accounts = vec![
            AccountMeta::new(user_pubkey, true),                    // sender (signer)
            AccountMeta::new(chat_group_pda, false),               // chat_group
            AccountMeta::new(memo_token_mint, false),              // mint
            AccountMeta::new(mint_authority_pda, false),           // mint_authority
            AccountMeta::new(user_token_account, false),           // sender_token_account
            AccountMeta::new_readonly(token_2022_program_id, false), // token_program
            AccountMeta::new_readonly(memo_mint_program_id, false), // memo_mint_program
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions sysvar
        ];
        
        let send_memo_instruction = Instruction::new_with_bytes(
            chat_program_id,
            &instruction_data,
            accounts,
        );
        
        base_instructions.push(send_memo_instruction);
        
        // Get latest blockhash
        check_timeout()?;
        let blockhash_response: serde_json::Value = self.send_request("getLatestBlockhash", serde_json::json!([])).await?;
        let blockhash_str = blockhash_response["value"]["blockhash"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get blockhash".to_string()))?;
        let blockhash = solana_sdk::hash::Hash::from_str(blockhash_str)
            .map_err(|e| RpcError::Other(format!("Failed to parse blockhash: {}", e)))?;
        
        // Create simulation transaction (without compute budget instruction)
        let sim_message = Message::new(&base_instructions, Some(&user_pubkey));
        let mut sim_transaction = Transaction::new_unsigned(sim_message);
        sim_transaction.message.recent_blockhash = blockhash;
        sim_transaction.sign(&[&keypair], blockhash);
        
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
        
        check_timeout()?;
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;

        // Parse simulation result to extract compute units consumed
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Chat message simulation consumed {} compute units", units_consumed);
            // Apply 20% buffer and ensure minimum
            let with_buffer = (units_consumed as f64 * ChatConfig::COMPUTE_UNIT_BUFFER) as u64;
            let final_units = with_buffer.max(ChatConfig::MIN_COMPUTE_UNITS).min(ChatConfig::MAX_COMPUTE_UNITS);
            
            log::info!("CU calculation: simulated={}, with_buffer={}, final={}", 
                      units_consumed, with_buffer, final_units);
            
            final_units
        } else {
            log::warn!("Failed to get compute units from simulation, using minimum guarantee");
            ChatConfig::MIN_COMPUTE_UNITS
        };

        log::info!("Using {} compute units for chat message (simulation: {}, +20% buffer)", 
          computed_units, 
          sim_result["value"]["unitsConsumed"].as_u64().unwrap_or(0));
        
        // Now build the final transaction with the calculated compute units
        let mut final_instructions = vec![];

        // Add compute budget instruction first (index 0)
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));

        // Add memo instruction SECOND (index 1) - CONTRACT REQUIREMENT
        final_instructions.push(spl_memo::build_memo(
            memo_data_base64.as_bytes(), // UTF-8 Base64 string
            &[&user_pubkey],
        ));

        // If token account doesn't exist, create it AFTER memo (index 2+)
        if token_account_info["value"].is_null() {
            log::info!("User token account does not exist, creating it...");
            final_instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                    &user_pubkey,           
                    &user_pubkey,           
                    &memo_token_mint,       
                    &token_2022_program_id  
                )
            );
        }

        // Add send_memo_to_group instruction LAST
        let mut instruction_data = ChatConfig::get_send_memo_to_group_discriminator().to_vec();
        instruction_data.extend_from_slice(&group_id.to_le_bytes());

        let accounts = vec![
            AccountMeta::new(user_pubkey, true),
            AccountMeta::new(chat_group_pda, false),
            AccountMeta::new(memo_token_mint, false),
            AccountMeta::new(mint_authority_pda, false),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new_readonly(token_2022_program_id, false),
            AccountMeta::new_readonly(memo_mint_program_id, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
        ];

        let send_memo_instruction = Instruction::new_with_bytes(
            chat_program_id,
            &instruction_data,
            accounts,
        );

        final_instructions.push(send_memo_instruction);
        
        // Create and sign final transaction
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
        
        check_timeout()?;
        log::info!("Sending chat message transaction...");
        
        // add error capture
        match self.send_request("sendTransaction", send_params).await {
            Ok(signature) => {
                log::info!("Chat message sent successfully! Signature: {}", signature);
                Ok(signature)
            },
            Err(e) => {
                log::error!("send_request returned error in send_chat_message_internal: {:?}", e);
                Err(e)
            }
        }
    }

    /// Original method maintained for backward compatibility
    pub async fn send_chat_message(
        &self,
        group_id: u64,
        message: &str,
        keypair_bytes: &[u8],
        receiver: Option<String>,
        reply_to_sig: Option<String>,
    ) -> Result<String, RpcError> {
        self.send_chat_message_with_timeout(group_id, message, keypair_bytes, receiver, reply_to_sig, None).await
    }
}