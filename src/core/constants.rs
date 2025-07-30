//! Global constants for the MEMO application
//! 
//! This module contains all the shared constants used across the application,
//! ensuring consistency and easy maintenance.

/// MEMO Token mint address
pub const TOKEN_MINT: &str = "HLCoc7wNDavNMfWWw2Bwd7U7A24cesuhBSNkxZgvZm1";

/// Program IDs and other blockchain-related constants
pub mod blockchain {
    /// Main program ID for token operations
    pub const TOKEN_PROGRAM_ID: &str = "TD8dwXKKg7M3QpWa9mQQpcvzaRasDU1MjmQWqZ9UZiw";
    
    /// Mint contract program ID
    pub const MINT_PROGRAM_ID: &str = "A31a17bhgQyRQygeZa1SybytjbCdjMpu6oPr9M3iQWzy";
    
    /// Token 2022 Program ID
    pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
}

/// Memo validation constants
pub mod memo {
    /// Minimum memo length in bytes
    pub const MIN_LENGTH: usize = 69;
    
    /// Maximum memo length in bytes (for mint operations)
    pub const MAX_LENGTH_MINT: usize = 800;
    
    /// Maximum memo length in bytes (for token operations)
    pub const MAX_LENGTH_TOKEN: usize = 700;
}

/// PDA Seeds used across the application
pub mod seeds {
    pub const USER_PROFILE: &[u8] = b"user_profile";
    pub const MINT_AUTHORITY: &[u8] = b"mint_authority";
    pub const LATEST_BURN_SHARD: &[u8] = b"latest_burn_shard";
    pub const GLOBAL_TOP_BURN_INDEX: &[u8] = b"global_top_burn_index";
    pub const TOP_BURN_SHARD: &[u8] = b"top_burn_shard";
    pub const BURN_HISTORY: &[u8] = b"burn_history";
} 