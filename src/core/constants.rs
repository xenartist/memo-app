/// Shared constants used across all RPC modules
/// 
/// This module centralizes all common constants to avoid duplication
/// across different RPC implementation files.

// ============================================================================
// Memo Length Constraints
// ============================================================================

/// Minimum memo length (from contract constraint: 69 bytes)
pub const MIN_MEMO_LENGTH: usize = 69;

/// Maximum memo length (from contract constraint: 800 bytes)
pub const MAX_MEMO_LENGTH: usize = 800;

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
