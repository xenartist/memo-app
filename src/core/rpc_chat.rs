use super::rpc_base::{RpcConnection, RpcError};
use super::network_config::get_program_ids;
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
use spl_associated_token_account;

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
    // Note: Program IDs and token mint are now retrieved dynamically from network configuration
    
    /// PDA Seeds for chat contract
    pub const GLOBAL_COUNTER_SEED: &'static [u8] = b"global_counter";
    pub const CHAT_GROUP_SEED: &'static [u8] = b"chat_group";
    pub const BURN_LEADERBOARD_SEED: &'static [u8] = b"burn_leaderboard";
    
    /// Minimum burn amount required to create a chat group (42,069 tokens = 42,069,000,000 lamports)
    pub const MIN_BURN_AMOUNT: u64 = 42_069_000_000;
    
    /// Memo validation limits (from contract: 69-800 bytes)
    pub const MIN_MEMO_LENGTH: usize = 69;
    pub const MAX_MEMO_LENGTH: usize = 800;
    
    /// Maximum payload length = memo maximum length - borsh fixed overhead
    pub const MAX_PAYLOAD_LENGTH: usize = Self::MAX_MEMO_LENGTH - BORSH_FIXED_OVERHEAD; // 800 - 13 = 787
    
    /// Compute budget configuration
    pub const COMPUTE_UNIT_BUFFER: f64 = 1.2; // 20% buffer for chat operations
    
    /// Helper functions
    pub fn get_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.chat_program_id)
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
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.mint_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-mint program ID: {}", e)))
    }
    
    /// Helper to get token mint
    pub fn get_memo_token_mint() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.token_mint)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo token mint: {}", e)))
    }
    
    /// Helper to get Token 2022 program ID
    pub fn get_token_2022_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.token_2022_program_id)
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

    /// Get create_chat_group instruction discriminator
    pub fn get_create_chat_group_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:create_chat_group");
        let result = hasher.finalize();
        let mut discriminator = [0u8; 8];
        discriminator.copy_from_slice(&result[..8]);
        discriminator
    }

    /// Helper to get memo-burn program ID
    pub fn get_memo_burn_program_id() -> Result<Pubkey, RpcError> {
        let program_ids = get_program_ids();
        Pubkey::from_str(program_ids.burn_program_id)
            .map_err(|e| RpcError::InvalidAddress(format!("Invalid memo-burn program ID: {}", e)))
    }

    /// Calculate burn leaderboard PDA
    pub fn get_burn_leaderboard_pda() -> Result<(Pubkey, u8), RpcError> {
        let program_id = Self::get_program_id()?;
        Ok(Pubkey::find_program_address(
            &[Self::BURN_LEADERBOARD_SEED],
            &program_id
        ))
    }

    /// Get burn_tokens_for_group instruction discriminator
    pub fn get_burn_tokens_for_group_discriminator() -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(b"global:burn_tokens_for_group");
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

/// Chat group burn data structure (stored in BurnMemo.payload for burn_tokens_for_group)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ChatGroupBurnData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "chat" for memo-chat contract)
    pub category: String,
    
    /// Operation type (must be "burn_for_group" for burning tokens)
    pub operation: String,
    
    /// Group ID (must match the target group)
    pub group_id: u64,
    
    /// Burner pubkey as string (must match the transaction signer)
    pub burner: String,
    
    /// Burn message (optional, max 512 characters)
    pub message: String,
}

impl ChatGroupBurnData {
    /// Create new chat group burn data
    pub fn new(
        group_id: u64,
        burner: String,
        message: String,
    ) -> Self {
        Self {
            version: CHAT_GROUP_CREATION_DATA_VERSION,
            category: "chat".to_string(),
            operation: "burn_for_group".to_string(),
            group_id,
            burner,
            message,
        }
    }
    
    /// Validate burn data
    pub fn validate(&self, expected_group_id: u64, expected_burner: &str) -> Result<(), RpcError> {
        // Validate version
        if self.version != CHAT_GROUP_CREATION_DATA_VERSION {
            return Err(RpcError::InvalidParameter(format!(
                "Unsupported chat group burn data version: {} (expected: {})", 
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
        if self.operation != "burn_for_group" {
            return Err(RpcError::InvalidParameter(format!(
                "Invalid operation: '{}' (expected: 'burn_for_group')", self.operation
            )));
        }
        
        // Validate group ID
        if self.group_id != expected_group_id {
            return Err(RpcError::InvalidParameter(format!(
                "Group ID mismatch: data contains {}, expected {}", 
                self.group_id, expected_group_id
            )));
        }
        
        // Validate burner
        if self.burner != expected_burner {
            return Err(RpcError::InvalidParameter(format!(
                "Burner mismatch: data contains {}, expected {}", 
                self.burner, expected_burner
            )));
        }
        
        // Validate message
        if self.message.len() > 512 {
            return Err(RpcError::InvalidParameter("Burn message too long (max 512 characters)".to_string()));
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

/// Parse Base64+Borsh-formatted memo data to extract burn message
fn parse_borsh_burn_message(memo_data: &[u8]) -> Option<(String, String, u64)> {
    // Convert bytes to UTF-8 string (should be Base64)
    let memo_str = std::str::from_utf8(memo_data).ok()?;
    
    // Decode Base64 to get original Borsh binary data
    let borsh_bytes = base64::decode(memo_str).ok()?;
    
    // Deserialize Borsh binary data to BurnMemo first
    match BurnMemo::try_from_slice(&borsh_bytes) {
        Ok(burn_memo) => {
            // Try to deserialize the payload as ChatGroupBurnData
            match ChatGroupBurnData::try_from_slice(&burn_memo.payload) {
                Ok(burn_data) => {
                    // Validate category and operation
                    if burn_data.category == "chat" && burn_data.operation == "burn_for_group" {
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

/// Parse memo data and determine message type
fn parse_memo_data(memo_data: &[u8]) -> Option<(String, String, String, Option<u64>)> {
    // Try parsing as chat message first
    if let Some((sender, message)) = parse_borsh_chat_message(memo_data) {
        return Some((sender, message, "chat".to_string(), None));
    }
    
    // Try parsing as burn message
    if let Some((burner, message, burn_amount)) = parse_borsh_burn_message(memo_data) {
        return Some((burner, message, "burn".to_string(), Some(burn_amount)));
    }
    
    None
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
    pub sender: String,         // Sender's public key (for chat) or burner (for burn)
    pub message: String,        // The memo content or burn message
    pub timestamp: i64,         // Block time
    pub slot: u64,             // Slot number
    pub memo_amount: u64,      // Amount of MEMO tokens burned for this message
    pub message_type: String,  // "chat" or "burn"
    pub burn_amount: Option<u64>, // For burn messages, the amount burned (in lamports)
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
                message_type: "chat".to_string(),
                burn_amount: None,
            },
            status: MessageStatus::Sending,
            is_local: true,
        }
    }
    
    /// Create a new local burn message for immediate UI display
    pub fn new_local_burn(sender: String, message: String, burn_amount: u64, group_id: u64) -> Self {
        Self {
            message: ChatMessage {
                signature: format!("local_burn_{}", js_sys::Date::now() as u64), // temporary local signature
                sender,
                message,
                timestamp: (js_sys::Date::now() / 1000.0) as i64, // current timestamp
                slot: 0,
                memo_amount: 0,
                message_type: "burn".to_string(),
                burn_amount: Some(burn_amount * 1_000_000), // Convert to lamports for display
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
    /// Build an unsigned transaction to send a chat message
    pub async fn build_send_chat_message_transaction(
        &self,
        user_pubkey: &Pubkey,
        group_id: u64,
        message: &str,
        receiver: Option<String>,
        reply_to_sig: Option<String>,
    ) -> Result<Transaction, RpcError> {
        // Validate message
        if message.is_empty() {
            return Err(RpcError::InvalidParameter("Message cannot be empty".to_string()));
        }
        if message.len() > 512 {
            return Err(RpcError::InvalidParameter("Message too long (max 512 characters)".to_string()));
        }
        
        log::info!("Building send chat message transaction to group {}: {} characters", group_id, message.len());
        
        let chat_program_id = ChatConfig::get_program_id()?;
        let memo_mint_program_id = ChatConfig::get_memo_mint_program_id()?;
        let memo_token_mint = ChatConfig::get_memo_token_mint()?;
        let token_2022_program_id = ChatConfig::get_token_2022_program_id()?;
        
        let (chat_group_pda, _) = ChatConfig::get_chat_group_pda(group_id)?;
        let (mint_authority_pda, _) = ChatConfig::get_mint_authority_pda()?;
        
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        // Check if user's token account exists
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
        
        chat_message_data.validate(group_id, &user_pubkey.to_string())?;
        
        let memo_data_bytes = chat_message_data.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize chat message data: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        ChatConfig::validate_memo_length(memo_data_base64.as_bytes())?;
        
        // Build instructions
        let mut instructions = vec![];
        
        // Add compute budget instruction (use a reasonable default)
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(400_000));
        
        // Add memo instruction
        instructions.push(spl_memo::build_memo(
            memo_data_base64.as_bytes(),
            &[user_pubkey],
        ));
        
        // If token account doesn't exist, create it
        if token_account_info["value"].is_null() {
            log::info!("User token account does not exist, will create it");
            instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                    user_pubkey,
                    user_pubkey,
                    &memo_token_mint,
                    &token_2022_program_id
                )
            );
        }
        
        // Create send_memo_to_group instruction
        let mut instruction_data = ChatConfig::get_send_memo_to_group_discriminator().to_vec();
        instruction_data.extend_from_slice(&group_id.to_le_bytes());
        
        let accounts = vec![
            AccountMeta::new(*user_pubkey, true),
            AccountMeta::new(chat_group_pda, false),
            AccountMeta::new(memo_token_mint, false),
            AccountMeta::new_readonly(mint_authority_pda, false),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new_readonly(token_2022_program_id, false),
            AccountMeta::new_readonly(memo_mint_program_id, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
        ];
        
        instructions.push(Instruction::new_with_bytes(
            chat_program_id,
            &instruction_data,
            accounts,
        ));
        
        let blockhash = self.get_latest_blockhash().await?;
        
        let message = Message::new(&instructions, Some(user_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        
        Ok(transaction)
    }

    /// Build an unsigned transaction to create a chat group
    pub async fn build_create_chat_group_transaction(
        &self,
        user_pubkey: &Pubkey,
        name: &str,
        description: &str,
        image: &str,
        tags: Vec<String>,
        min_memo_interval: Option<i64>,
        burn_amount: u64,
    ) -> Result<(Transaction, u64), RpcError> {
        // Basic parameter validation
        if name.is_empty() || name.len() > 64 {
            return Err(RpcError::InvalidParameter(format!("Group name must be 1-64 characters, got {}", name.len())));
        }
        if description.len() > 128 {
            return Err(RpcError::InvalidParameter(format!("Group description must be at most 128 characters, got {}", description.len())));
        }
        if burn_amount < ChatConfig::MIN_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!("Burn amount must be at least {} MEMO tokens", ChatConfig::MIN_BURN_AMOUNT / 1_000_000)));
        }
        if burn_amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens".to_string()));
        }
        
        log::info!("Building create chat group transaction '{}': {} tokens", name, burn_amount / 1_000_000);
        
        // Get next group_id
        let global_stats = self.get_chat_global_statistics().await?;
        let expected_group_id = global_stats.total_groups;
        
        let chat_program_id = ChatConfig::get_program_id()?;
        let memo_token_mint = ChatConfig::get_memo_token_mint()?;
        let token_2022_program_id = ChatConfig::get_token_2022_program_id()?;
        let memo_burn_program_id = ChatConfig::get_memo_burn_program_id()?;
        
        let (global_counter_pda, _) = ChatConfig::get_global_counter_pda()?;
        let (chat_group_pda, _) = ChatConfig::get_chat_group_pda(expected_group_id)?;
        let (burn_leaderboard_pda, _) = ChatConfig::get_burn_leaderboard_pda()?;
        let (user_global_burn_stats_pda, _) = ChatConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey, &memo_token_mint, &token_2022_program_id,
        );
        
        // Prepare group creation data
        let group_creation_data = ChatGroupCreationData::new(
            expected_group_id,
            name.to_string(),
            description.to_string(),
            image.to_string(),
            tags,
            min_memo_interval,
        );
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount,
            payload: group_creation_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize group data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        ChatConfig::validate_memo_length(memo_data_base64.as_bytes())?;
        
        let mut instructions = vec![];
        
        // Add compute budget instruction
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(400_000));
        
        // Add memo instruction
        instructions.push(Instruction {
            program_id: spl_memo::id(),
            accounts: vec![],
            data: memo_data_base64.into_bytes(),
        });
        
        // Create group instruction
        let mut instruction_data = ChatConfig::get_create_chat_group_discriminator().to_vec();
        instruction_data.extend_from_slice(&expected_group_id.to_le_bytes());
        instruction_data.extend_from_slice(&burn_amount.to_le_bytes());
        
        instructions.push(Instruction::new_with_bytes(
            chat_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(global_counter_pda, false),
                AccountMeta::new(chat_group_pda, false),
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
        
        let message = Message::new(&instructions, Some(user_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        
        Ok((transaction, expected_group_id))
    }

    /// Build an unsigned transaction to burn tokens for a group
    pub async fn build_burn_tokens_for_group_transaction(
        &self,
        user_pubkey: &Pubkey,
        group_id: u64,
        amount: u64,
        message: &str,
    ) -> Result<Transaction, RpcError> {
        // Basic parameter validation
        if amount < 1_000_000 {
            return Err(RpcError::InvalidParameter("Burn amount must be at least 1 MEMO token".to_string()));
        }
        if amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens".to_string()));
        }
        if message.len() > 512 {
            return Err(RpcError::InvalidParameter("Burn message too long (max 512 characters)".to_string()));
        }
        
        log::info!("Building burn tokens for group transaction: {} tokens for group {}", amount / 1_000_000, group_id);
        
        let chat_program_id = ChatConfig::get_program_id()?;
        let memo_token_mint = ChatConfig::get_memo_token_mint()?;
        let token_2022_program_id = ChatConfig::get_token_2022_program_id()?;
        let memo_burn_program_id = ChatConfig::get_memo_burn_program_id()?;
        
        let (chat_group_pda, _) = ChatConfig::get_chat_group_pda(group_id)?;
        let (burn_leaderboard_pda, _) = ChatConfig::get_burn_leaderboard_pda()?;
        let (user_global_burn_stats_pda, _) = ChatConfig::get_user_global_burn_stats_pda(user_pubkey)?;
        
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        // Prepare burn data
        let burn_data = ChatGroupBurnData::new(
            group_id,
            user_pubkey.to_string(),
            message.to_string(),
        );
        
        burn_data.validate(group_id, &user_pubkey.to_string())?;
        
        let burn_memo = BurnMemo {
            version: 1,
            burn_amount: amount,
            payload: burn_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize burn data: {}", e)))?,
        };
        
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        ChatConfig::validate_memo_length(memo_data_base64.as_bytes())?;
        
        let mut instructions = vec![];
        
        // Add compute budget instruction
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(400_000));
        
        // Add memo instruction
        instructions.push(Instruction {
            program_id: spl_memo::id(),
            accounts: vec![],
            data: memo_data_base64.into_bytes(),
        });
        
        // Create burn instruction
        let mut instruction_data = ChatConfig::get_burn_tokens_for_group_discriminator().to_vec();
        instruction_data.extend_from_slice(&group_id.to_le_bytes());
        instruction_data.extend_from_slice(&amount.to_le_bytes());
        
        instructions.push(Instruction::new_with_bytes(
            chat_program_id,
            &instruction_data,
            vec![
                AccountMeta::new(*user_pubkey, true),
                AccountMeta::new(chat_group_pda, false),
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
        
        let message = Message::new(&instructions, Some(user_pubkey));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.message.recent_blockhash = blockhash;
        
        Ok(transaction)
    }

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
        
        let expected_program_id = ChatConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-chat program. Expected: {}, Got: {}", 
                expected_program_id, owner
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
        
        let expected_program_id = ChatConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-chat program. Expected: {}, Got: {}", 
                expected_program_id, owner
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
                                        
                                        // Parse memo data (both chat and burn messages)
                                        if let Some((sender, message, msg_type, burn_amount)) = parse_memo_data(memo_bytes) {
                                            // Skip empty messages
                                            if !message.trim().is_empty() {
                                                messages.push(ChatMessage {
                                                    signature: signature.clone(),
                                                    sender,
                                                    message,
                                                    timestamp: block_time,
                                                    slot,
                                                    memo_amount: 0,
                                                    message_type: msg_type,
                                                    burn_amount,
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
    /// Legacy method - use build_send_chat_message_transaction + sign in Session + send_signed_transaction
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
            AccountMeta::new_readonly(mint_authority_pda, false),           // mint_authority
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
            // apply 10% buffer
            let final_units = (units_consumed as f64 * ChatConfig::COMPUTE_UNIT_BUFFER) as u64;
            
            log::info!("CU calculation: simulated={}, final={} (+10%)", 
                      units_consumed, final_units);
            
            final_units
        } else {
            log::error!("Failed to get compute units from simulation");
            return Err(RpcError::Other("Simulation failed to provide compute units".to_string()));
        };

        log::info!("Using {} compute units for chat message", computed_units);
        
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
            {"encoding": "base64", "skipPreflight": false, "preflightCommitment": "processed"}
        ]);
        
        check_timeout()?;
        log::info!("Sending chat message transaction...");
        
        // Send transaction with minimal logging
        match self.send_request("sendTransaction", send_params).await {
            Ok(signature) => {
                log::info!("Chat message sent successfully");
                Ok(signature)
            },
            Err(e) => {
                log::error!("Failed to send chat message: {}", e);
                Err(e)
            }
        }
    }

    /// Legacy method - use build_send_chat_message_transaction + sign in Session + send_signed_transaction
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

    /// Legacy method - use build_create_chat_group_transaction + sign in Session + send_signed_transaction
    /// 
    /// # Parameters
    /// * `name` - Group name (1-64 characters)
    /// * `description` - Group description (max 128 characters) 
    /// * `image` - Group image info (max 256 characters)
    /// * `tags` - Tags (max 4 tags, each max 32 characters)
    /// * `min_memo_interval` - Minimum memo interval in seconds (optional, defaults to 60)
    /// * `burn_amount` - Amount of MEMO tokens to burn (in lamports, must be >= 1_000_000)
    /// * `keypair_bytes` - The user's keypair bytes for signing
    /// 
    /// # Returns
    /// Result containing transaction signature and created group_id
    pub async fn create_chat_group(
        &self,
        name: &str,
        description: &str,
        image: &str,
        tags: Vec<String>,
        min_memo_interval: Option<i64>,
        burn_amount: u64,
        keypair_bytes: &[u8],
    ) -> Result<(String, u64), RpcError> {
        // basic parameter validation
        if name.is_empty() || name.len() > 64 {
            return Err(RpcError::InvalidParameter(format!("Group name must be 1-64 characters, got {}", name.len())));
        }
        if description.len() > 128 {
            return Err(RpcError::InvalidParameter(format!("Group description must be at most 128 characters, got {}", description.len())));
        }
        if burn_amount < ChatConfig::MIN_BURN_AMOUNT {
            return Err(RpcError::InvalidParameter(format!("Burn amount must be at least {} MEMO tokens (42,069), got {} lamports", ChatConfig::MIN_BURN_AMOUNT / 1_000_000, burn_amount)));
        }
        if burn_amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens (multiple of 1,000,000 lamports)".to_string()));
        }
        
        log::info!("Creating chat group '{}': {} tokens", name, burn_amount / 1_000_000);
        
        // create keypair
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let user_pubkey = keypair.pubkey();
        
        // get next group_id
        let global_stats = self.get_chat_global_statistics().await?;
        let expected_group_id = global_stats.total_groups;
        
        // get config
        let chat_program_id = ChatConfig::get_program_id()?;
        let memo_token_mint = ChatConfig::get_memo_token_mint()?;
        let token_2022_program_id = ChatConfig::get_token_2022_program_id()?;
        let memo_burn_program_id = ChatConfig::get_memo_burn_program_id()?;
        
        // calculate PDAs
        let (global_counter_pda, _) = ChatConfig::get_global_counter_pda()?;
        let (chat_group_pda, _) = ChatConfig::get_chat_group_pda(expected_group_id)?;
        let (burn_leaderboard_pda, _) = ChatConfig::get_burn_leaderboard_pda()?;
        let (user_global_burn_stats_pda, _) = ChatConfig::get_user_global_burn_stats_pda(&user_pubkey)?;
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &user_pubkey, &memo_token_mint, &token_2022_program_id,
        );
        
        // prepare group creation data
        let group_creation_data = ChatGroupCreationData::new(
            expected_group_id, name.to_string(), description.to_string(), 
            image.to_string(), tags.clone(), min_memo_interval,
        );
        
        // create BurnMemo
        let burn_memo = BurnMemo {
            version: BURN_MEMO_VERSION,
            burn_amount,
            payload: group_creation_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize group data: {}", e)))?,
        };
        
        // serialize and encode to Base64
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        // validate memo length
        ChatConfig::validate_memo_length(memo_data_base64.as_bytes())?;
        
        // check if token account exists
        let token_account_info = self.get_account_info(&user_token_account.to_string(), Some("base64")).await?;
        let token_account_info: serde_json::Value = serde_json::from_str(&token_account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse token account info: {}", e)))?;
        
        // base instructions
        let mut base_instructions = vec![];
        
        // 1. memo instruction
        base_instructions.push(spl_memo::build_memo(memo_data_base64.as_bytes(), &[&user_pubkey]));
        
        // 2. if needed, create token account
        if token_account_info["value"].is_null() {
            base_instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                    &user_pubkey, &user_pubkey, &memo_token_mint, &token_2022_program_id
                )
            );
        }
        
        // 3. create chat group instruction
        let mut instruction_data = ChatConfig::get_create_chat_group_discriminator().to_vec();
        instruction_data.extend_from_slice(&expected_group_id.to_le_bytes());
        instruction_data.extend_from_slice(&burn_amount.to_le_bytes());
        
        let accounts = vec![
            AccountMeta::new(user_pubkey, true),                      // creator
            AccountMeta::new(global_counter_pda, false),             // global_counter
            AccountMeta::new(chat_group_pda, false),                 // chat_group
            AccountMeta::new(burn_leaderboard_pda, false),           // burn_leaderboard
            AccountMeta::new(memo_token_mint, false),                // mint
            AccountMeta::new(user_token_account, false),             // creator_token_account
            AccountMeta::new(user_global_burn_stats_pda, false),     // user_global_burn_stats
            AccountMeta::new_readonly(token_2022_program_id, false), // token_program
            AccountMeta::new_readonly(memo_burn_program_id, false),  // memo_burn_program
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false), // system_program
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions
        ];
        
        base_instructions.push(Instruction::new_with_bytes(chat_program_id, &instruction_data, accounts));
        
        // get blockhash and simulate transaction
        let blockhash_response: serde_json::Value = self.send_request("getLatestBlockhash", serde_json::json!([])).await?;
        let blockhash_str = blockhash_response["value"]["blockhash"].as_str()
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
        
        log::info!("Simulating chat group creation transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;

        // Parse simulation result to extract compute units consumed
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Chat group creation simulation consumed {} compute units", units_consumed);
            // apply 10% buffer
            let final_units = (units_consumed as f64 * ChatConfig::COMPUTE_UNIT_BUFFER) as u64;
            
            log::info!("CU calculation for group creation: simulated={}, final={} (+10%)", 
                      units_consumed, final_units);
            
            final_units
        } else {
            log::error!("Failed to get compute units from simulation");
            return Err(RpcError::Other("Simulation failed to provide compute units".to_string()));
        };

        log::info!("Using {} compute units for chat group creation", computed_units);
        
        // Now build the final transaction with the calculated compute units
        let mut final_instructions = vec![];

        // Add compute budget instruction first
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));

        // Add all base instructions
        final_instructions.extend(base_instructions);
        
        // create and sign final transaction
        let final_message = Message::new(&final_instructions, Some(&user_pubkey));
        let mut final_transaction = Transaction::new_unsigned(final_message);
        final_transaction.message.recent_blockhash = blockhash;
        final_transaction.sign(&[&keypair], blockhash);
        
        // send transaction
        let serialized_tx = base64::encode(bincode::serialize(&final_transaction)
            .map_err(|e| RpcError::Other(format!("Failed to serialize transaction: {}", e)))?);
        
        let send_params = serde_json::json!([
            serialized_tx,
            {"encoding": "base64", "skipPreflight": false, "preflightCommitment": "processed"}
        ]);
        
        let signature = self.send_request("sendTransaction", send_params).await?;
        
        log::info!("Chat group '{}' created successfully with ID {}", name, expected_group_id);
        Ok((signature, expected_group_id))
    }

    /// Burn tokens for a chat group
    /// 
    /// Legacy method - use build_burn_tokens_for_group_transaction + sign in Session + send_signed_transaction
    /// 
    /// # Parameters
    /// * `group_id` - The ID of the chat group to burn tokens for
    /// * `amount` - Amount of MEMO tokens to burn (in lamports, must be >= 1_000_000)
    /// * `message` - Optional burn message (max 512 characters)
    /// * `keypair_bytes` - The user's keypair bytes for signing
    /// 
    /// # Returns
    /// Result containing transaction signature
    pub async fn burn_tokens_for_group(
        &self,
        group_id: u64,
        amount: u64,
        message: &str,
        keypair_bytes: &[u8],
    ) -> Result<String, RpcError> {
        // Basic parameter validation
        if amount < 1_000_000 {
            return Err(RpcError::InvalidParameter("Burn amount must be at least 1 MEMO token (1,000,000 lamports)".to_string()));
        }
        if amount % 1_000_000 != 0 {
            return Err(RpcError::InvalidParameter("Burn amount must be a whole number of tokens (multiple of 1,000,000 lamports)".to_string()));
        }
        if message.len() > 512 {
            return Err(RpcError::InvalidParameter("Burn message too long (max 512 characters)".to_string()));
        }
        
        log::info!("Burning {} tokens for group {}: {}", amount / 1_000_000, group_id, message);
        
        // Create keypair from bytes and get pubkey
        let keypair = Keypair::from_bytes(keypair_bytes)
            .map_err(|e| RpcError::Other(format!("Failed to create keypair: {}", e)))?;
        let user_pubkey = keypair.pubkey();
        
        log::info!("Burner pubkey: {}", user_pubkey);
        
        // Get contract configuration
        let chat_program_id = ChatConfig::get_program_id()?;
        let memo_token_mint = ChatConfig::get_memo_token_mint()?;
        let token_2022_program_id = ChatConfig::get_token_2022_program_id()?;
        let memo_burn_program_id = ChatConfig::get_memo_burn_program_id()?;
        
        // Calculate PDAs
        let (chat_group_pda, _) = ChatConfig::get_chat_group_pda(group_id)?;
        let (burn_leaderboard_pda, _) = ChatConfig::get_burn_leaderboard_pda()?;
        let (user_global_burn_stats_pda, _) = ChatConfig::get_user_global_burn_stats_pda(&user_pubkey)?;
        
        // Calculate user's token account (ATA)
        let user_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &user_pubkey,
            &memo_token_mint,
            &token_2022_program_id,
        );
        
        log::info!("Chat group PDA: {}", chat_group_pda);
        log::info!("Burn leaderboard PDA: {}", burn_leaderboard_pda);
        log::info!("User token account: {}", user_token_account);
        
        // Check if user's token account exists
        let token_account_info = self.get_account_info(&user_token_account.to_string(), Some("base64")).await?;
        let token_account_info: serde_json::Value = serde_json::from_str(&token_account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse token account info: {}", e)))?;
        
        if token_account_info["value"].is_null() {
            return Err(RpcError::Other("User token account does not exist. Please mint some tokens first.".to_string()));
        }
        
        // Prepare chat group burn data
        let burn_data = ChatGroupBurnData::new(
            group_id,
            user_pubkey.to_string(),
            message.to_string(),
        );
        
        // Validate burn data
        burn_data.validate(group_id, &user_pubkey.to_string())?;
        
        // Create BurnMemo
        let burn_memo = BurnMemo {
            version: BURN_MEMO_VERSION,
            burn_amount: amount,
            payload: burn_data.try_to_vec()
                .map_err(|e| RpcError::Other(format!("Failed to serialize burn data: {}", e)))?,
        };
        
        // Serialize and encode to Base64
        let memo_data_bytes = burn_memo.try_to_vec()
            .map_err(|e| RpcError::Other(format!("Failed to serialize burn memo: {}", e)))?;
        let memo_data_base64 = base64::encode(&memo_data_bytes);
        
        log::info!("Burn memo data: {} bytes Borsh → {} bytes Base64", 
                  memo_data_bytes.len(), memo_data_base64.len());

        // Validate memo length
        ChatConfig::validate_memo_length(memo_data_base64.as_bytes())?;
        
        // Build base instructions (for simulation)
        let mut base_instructions = vec![];

        // Add memo instruction
        base_instructions.push(spl_memo::build_memo(
            memo_data_base64.as_bytes(),
            &[&user_pubkey],
        ));
        
        // Create burn_tokens_for_group instruction
        let mut instruction_data = ChatConfig::get_burn_tokens_for_group_discriminator().to_vec();
        instruction_data.extend_from_slice(&group_id.to_le_bytes());
        instruction_data.extend_from_slice(&amount.to_le_bytes());
        
        let accounts = vec![
            AccountMeta::new(user_pubkey, true),                    // burner (signer)
            AccountMeta::new(chat_group_pda, false),               // chat_group
            AccountMeta::new(burn_leaderboard_pda, false),         // burn_leaderboard
            AccountMeta::new(memo_token_mint, false),              // mint
            AccountMeta::new(user_token_account, false),           // burner_token_account
            AccountMeta::new(user_global_burn_stats_pda, false),   // user_global_burn_stats
            AccountMeta::new_readonly(token_2022_program_id, false), // token_program
            AccountMeta::new_readonly(memo_burn_program_id, false), // memo_burn_program
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false), // instructions sysvar
        ];
        
        let burn_instruction = Instruction::new_with_bytes(
            chat_program_id,
            &instruction_data,
            accounts,
        );
        
        base_instructions.push(burn_instruction);
        
        // Get latest blockhash and simulate transaction
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
        
        log::info!("Simulating burn tokens for group transaction...");
        let sim_result = self.simulate_transaction(&sim_serialized_tx, Some(sim_options)).await?;
        let sim_result: serde_json::Value = serde_json::from_str(&sim_result)
            .map_err(|e| RpcError::Other(format!("Failed to parse simulation result: {}", e)))?;

        // Parse simulation result to extract compute units consumed
        let computed_units = if let Some(units_consumed) = sim_result["value"]["unitsConsumed"].as_u64() {
            log::info!("Burn tokens for group simulation consumed {} compute units", units_consumed);
            // apply 20% buffer
            let final_units = (units_consumed as f64 * ChatConfig::COMPUTE_UNIT_BUFFER) as u64;
            
            log::info!("CU calculation for burn tokens: simulated={}, final={} (+20%)", 
                      units_consumed, final_units);
            
            final_units
        } else {
            log::error!("Failed to get compute units from simulation");
            return Err(RpcError::Other("Simulation failed to provide compute units".to_string()));
        };

        // use calculated CU, like sending message
        log::info!("Using {} compute units for burn tokens for group", computed_units);
        
        // Now build the final transaction with the calculated compute units
        let mut final_instructions = vec![];

        // Add compute budget instruction first
        final_instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(computed_units as u32));

        // Add all base instructions
        final_instructions.extend(base_instructions);
        
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
        
        log::info!("Sending burn tokens for group transaction...");
        
        // Send transaction
        match self.send_request("sendTransaction", send_params).await {
            Ok(signature) => {
                log::info!("Burn tokens for group successful");
                Ok(signature)
            },
            Err(e) => {
                log::error!("Failed to burn tokens for group: {}", e);
                Err(e)
            }
        }
    }

    /// get burn leaderboard
    /// 
    /// # Returns
    /// burn leaderboard data, including the top 100 chat groups
    pub async fn get_burn_leaderboard(&self) -> Result<BurnLeaderboardResponse, RpcError> {
        let (burn_leaderboard_pda, _) = ChatConfig::get_burn_leaderboard_pda()?;
        
        log::info!("Fetching burn leaderboard from PDA: {}", burn_leaderboard_pda);
        
        let account_info = self.get_account_info(&burn_leaderboard_pda.to_string(), Some("base64")).await?;
        let account_info: serde_json::Value = serde_json::from_str(&account_info)
            .map_err(|e| RpcError::Other(format!("Failed to parse leaderboard account info: {}", e)))?;
        
        if account_info["value"].is_null() {
            return Err(RpcError::Other(
                "Burn leaderboard not found. Please initialize the memo-chat system first.".to_string()
            ));
        }
        
        let account_data = account_info["value"]["data"][0]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get leaderboard account data".to_string()))?;
        
        let data = base64::decode(account_data)
            .map_err(|e| RpcError::Other(format!("Failed to decode leaderboard account data: {}", e)))?;
        
        // verify account owner
        let owner = account_info["value"]["owner"]
            .as_str()
            .ok_or_else(|| RpcError::Other("Failed to get leaderboard account owner".to_string()))?;
        
        let expected_program_id = ChatConfig::get_program_id()?.to_string();
        if owner != expected_program_id {
            return Err(RpcError::Other(format!(
                "Account not owned by memo-chat program. Expected: {}, Got: {}", 
                expected_program_id, owner
            )));
        }
        
        // parse leaderboard data
        self.parse_burn_leaderboard_data(&data)
    }
    
    /// parse burn leaderboard account data
    fn parse_burn_leaderboard_data(&self, data: &[u8]) -> Result<BurnLeaderboardResponse, RpcError> {
        if data.len() < 8 {
            return Err(RpcError::Other("Data too short for discriminator".to_string()));
        }
        
        let mut offset = 8; // skip discriminator
        
        // read current_size (u8)
        if data.len() < offset + 1 {
            return Err(RpcError::Other("Data too short for current_size".to_string()));
        }
        let current_size = data[offset];
        offset += 1;
        
        // read entries Vec length (u32)
        if data.len() < offset + 4 {
            return Err(RpcError::Other("Data too short for entries length".to_string()));
        }
        let entries_len = u32::from_le_bytes(
            data[offset..offset + 4].try_into()
                .map_err(|e| RpcError::Other(format!("Failed to parse entries length: {:?}", e)))?
        ) as usize;
        offset += 4;
        
        // read each leaderboard entry
        let mut entries = Vec::new();
        let mut total_burned_tokens = 0u64;
        
        for i in 0..entries_len {
            if data.len() < offset + 16 { // each entry 16 bytes (8 + 8)
                return Err(RpcError::Other(format!("Data too short for entry {}", i)));
            }
            
            // read group_id (u64)
            let group_id = u64::from_le_bytes(
                data[offset..offset + 8].try_into()
                    .map_err(|e| RpcError::Other(format!("Failed to parse group_id for entry {}: {:?}", i, e)))?
            );
            offset += 8;
            
            // read burned_amount (u64)
            let burned_amount = u64::from_le_bytes(
                data[offset..offset + 8].try_into()
                    .map_err(|e| RpcError::Other(format!("Failed to parse burned_amount for entry {}: {:?}", i, e)))?
            );
            offset += 8;
            
            total_burned_tokens = total_burned_tokens.saturating_add(burned_amount);
            
            entries.push(LeaderboardEntry {
                group_id,
                burned_amount,
                rank: (i + 1) as u8, // rank starts from 1
            });
        }
        
        log::info!("Parsed burn leaderboard: {} entries, total burned: {:.2} MEMO", 
                  entries.len(), total_burned_tokens as f64 / 1_000_000.0);
        
        Ok(BurnLeaderboardResponse {
            current_size,
            entries,
            total_burned_tokens,
        })
    }
    
    /// get the rank of a specific group in the burn leaderboard
    /// 
    /// # Parameters
    /// * `group_id` - group ID
    /// 
    /// # Returns
    /// rank (1-100), return None if the group is not in the leaderboard
    pub async fn get_group_burn_rank(&self, group_id: u64) -> Result<Option<u8>, RpcError> {
        let leaderboard = self.get_burn_leaderboard().await?;
        
        for entry in &leaderboard.entries {
            if entry.group_id == group_id {
                return Ok(Some(entry.rank));
            }
        }
        
        Ok(None)
    }
}

/// Chat group creation data structure (stored in BurnMemo.payload for create_chat_group)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ChatGroupCreationData {
    /// Version of this structure (for future compatibility)
    pub version: u8,
    
    /// Category of the request (must be "chat" for memo-chat contract)
    pub category: String,
    
    /// Operation type (must be "create_group" for group creation)
    pub operation: String,
    
    /// Group ID (must match expected_group_id)
    pub group_id: u64,
    
    /// Group name (required, 1-64 characters)
    pub name: String,
    
    /// Group description (optional, max 128 characters)  
    pub description: String,
    
    /// Group image info (optional, max 256 characters)
    pub image: String,
    
    /// Tags (optional, max 4 tags, each max 32 characters)
    pub tags: Vec<String>,
    
    /// Minimum memo interval in seconds (optional, defaults to 60)
    pub min_memo_interval: Option<i64>,
}

impl ChatGroupCreationData {
    /// Create new chat group creation data
    pub fn new(
        group_id: u64,
        name: String,
        description: String,
        image: String,
        tags: Vec<String>,
        min_memo_interval: Option<i64>,
    ) -> Self {
        Self {
            version: CHAT_GROUP_CREATION_DATA_VERSION,
            category: "chat".to_string(),
            operation: "create_group".to_string(),
            group_id,
            name,
            description,
            image,
            tags,
            min_memo_interval,
        }
    }
    
    /// Calculate the final memo size (Borsh + Base64) for this chat group creation data
    /// 
    /// # Parameters
    /// * `burn_amount` - The burn amount that will be used in BurnMemo
    /// 
    /// # Returns
    /// The final size in bytes after Borsh serialization and Base64 encoding
    pub fn calculate_final_memo_size(&self, burn_amount: u64) -> Result<usize, String> {
        // Serialize ChatGroupCreationData to Borsh
        let payload_bytes = self.try_to_vec()
            .map_err(|e| format!("Failed to serialize ChatGroupCreationData: {}", e))?;
        
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
        Ok(final_size >= ChatConfig::MIN_MEMO_LENGTH && final_size <= ChatConfig::MAX_MEMO_LENGTH)
    }
}

/// leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaderboardEntry {
    pub group_id: u64,
    pub burned_amount: u64,
    pub rank: u8, // rank (1-100)
}

/// burn leaderboard response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BurnLeaderboardResponse {
    pub current_size: u8,
    pub entries: Vec<LeaderboardEntry>,
    pub total_burned_tokens: u64, // total burned amount of all leaderboard entries
}

// Chat page view mode
#[derive(Clone, PartialEq)]
enum ChatView {
    GroupsList,
    ChatRoom(u64), // group_id
}

// Chat groups display mode
#[derive(Clone, PartialEq, Debug)]
enum GroupsDisplayMode {
    BurnLeaderboard,
    Latest,
    Oldest,
}

impl ToString for GroupsDisplayMode {
    fn to_string(&self) -> String {
        match self {
            GroupsDisplayMode::BurnLeaderboard => "Burn Leaderboard".to_string(),
            GroupsDisplayMode::Latest => "Latest".to_string(),
            GroupsDisplayMode::Oldest => "Oldest".to_string(),
        }
    }
}