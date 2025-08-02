use super::rpc_base::{RpcConnection, RpcError};
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;

/// Memo-Chat contract configuration and constants
pub struct ChatConfig;

impl ChatConfig {
    /// Memo-chat program ID
    pub const MEMO_CHAT_PROGRAM_ID: &'static str = "54ky4LNnRsbYioDSBKNrc5hG8HoDyZ6yhf8TuncxTBRF";
    
    /// PDA Seeds for chat contract
    pub const GLOBAL_COUNTER_SEED: &'static [u8] = b"global_counter";
    pub const CHAT_GROUP_SEED: &'static [u8] = b"chat_group";
    
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
}