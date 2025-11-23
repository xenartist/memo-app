/// Shared constants used across all RPC modules
/// 
/// This module centralizes all common constants to avoid duplication
/// across different RPC implementation files.

// ============================================================================
// Borsh Serialization Constants
// ============================================================================

/// Size of a u8 in Borsh serialization (version field)
pub const BORSH_U8_SIZE: usize = 1;

/// Size of a u64 in Borsh serialization (burn_amount field)
pub const BORSH_U64_SIZE: usize = 8;

/// Size of Vec length prefix in Borsh serialization (u32)
pub const BORSH_VEC_LENGTH_SIZE: usize = 4;

/// Total fixed overhead for Borsh serialization (version + burn_amount + vec_length)
/// This is the base size that doesn't include the actual payload data
pub const BORSH_FIXED_OVERHEAD: usize = BORSH_U8_SIZE + BORSH_U64_SIZE + BORSH_VEC_LENGTH_SIZE;

// ============================================================================
// Memo Length Constraints
// ============================================================================

/// Minimum memo length (from contract constraint: 69 bytes)
pub const MIN_MEMO_LENGTH: usize = 69;

/// Maximum memo length (from contract constraint: 800 bytes)
pub const MAX_MEMO_LENGTH: usize = 800;

/// Default maximum payload length (800 - 13 = 787 bytes)
pub const MAX_PAYLOAD_LENGTH: usize = MAX_MEMO_LENGTH - BORSH_FIXED_OVERHEAD;

// ============================================================================
// Compute Unit Configuration
// ============================================================================

/// Compute unit buffer percentage (1.0 = 0% buffer, uses exact simulation value)
/// This determines how much extra compute units to add beyond the simulated amount
pub const COMPUTE_UNIT_BUFFER: f64 = 1.0;

/// Minimum compute units to allocate for any transaction
pub const MIN_COMPUTE_UNITS: u64 = 200_000;

// ============================================================================
// Version Constants
// ============================================================================

/// Common burn memo version used across all burn operations
pub const BURN_MEMO_VERSION: u8 = 1;

