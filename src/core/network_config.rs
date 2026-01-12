use once_cell::sync::Lazy;
use std::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Network environment enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkType {
    /// Testnet environment with testnet program IDs
    Testnet,
    /// Production staging - testnet RPC with mainnet program IDs (for final testing)
    ProdStaging,
    /// Mainnet environment with mainnet program IDs
    Mainnet,
}

/// Network configuration including RPC endpoints and program IDs
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub network_type: NetworkType,
    pub rpc_endpoints: &'static [&'static str],
    pub program_ids: ProgramIds,
}

/// Program IDs and token addresses configuration
#[derive(Debug, Clone)]
pub struct ProgramIds {
    pub mint_program_id: &'static str,
    pub burn_program_id: &'static str,
    pub chat_program_id: &'static str,
    pub profile_program_id: &'static str,
    pub project_program_id: &'static str,
    pub blog_program_id: &'static str,
    pub forum_program_id: &'static str,
    pub token_mint: &'static str,
    pub token_2022_program_id: &'static str,
}

impl NetworkConfig {
    /// Testnet configuration
    const TESTNET: NetworkConfig = NetworkConfig {
        network_type: NetworkType::Testnet,
        rpc_endpoints: &[
            "https://rpc.testnet.x1.xyz",
        ],
        program_ids: ProgramIds {
            mint_program_id: "A31a17bhgQyRQygeZa1SybytjbCdjMpu6oPr9M3iQWzy",
            burn_program_id: "FEjJ9KKJETocmaStfsFteFrktPchDLAVNTMeTvndoxaP",
            chat_program_id: "54ky4LNnRsbYioDSBKNrc5hG8HoDyZ6yhf8TuncxTBRF",
            profile_program_id: "BwQTxuShrwJR15U6Utdfmfr4kZ18VT6FA1fcp58sT8US",
            project_program_id: "ENVapgjzzMjbRhLJ279yNsSgaQtDYYVgWq98j54yYnyx",
            blog_program_id: "HPvqPUneCLwb8YYoYTrWmy6o7viRKsnLTgxwkg7CCpfB",
            forum_program_id: "9kwS5nSidmoHq84TyNzqFrtD29odp4sdRxm97tCbdpbS",
            token_mint: "HLCoc7wNDavNMfWWw2Bwd7U7A24cesuhBSNkxZgvZm1",
            token_2022_program_id: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        },
    };

    /// Production staging configuration
    /// Uses testnet RPC but with mainnet program IDs for final verification
    const PROD_STAGING: NetworkConfig = NetworkConfig {
        network_type: NetworkType::ProdStaging,
        rpc_endpoints: &[
            "https://rpc.testnet.x1.xyz",
        ],
        program_ids: ProgramIds {
            mint_program_id: "8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX",
            burn_program_id: "2sb3gz5Cmr2g1ia5si2rmCZqPACxgaZXEmiS5k6Htcvh",
            chat_program_id: "Hni4qE8GGW5uwBWzUEkpPBDRwXvKCWhM96teieAReRyd",
            profile_program_id: "2BY8vPpQRFFwAqK3HqU5qL3qsGMH3VnX9Gv9bud3vzH8",
            project_program_id: "6Vavot6ybhWBG3rjNXnLfNRPVTz7Garf6E4EZk3byp3a",
            blog_program_id: "3EKdp88FgyPC41bxRDzFAtCDUMV2g9SVt5UiytE8wdzM",
            forum_program_id: "6gzhG5BveTkJfTi466toX4qmN3BtU9qp1Grnk61GvmXD",
            token_mint: "memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick",
            token_2022_program_id: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        },
    };

    /// Mainnet configuration
    const MAINNET: NetworkConfig = NetworkConfig {
        network_type: NetworkType::Mainnet,
        rpc_endpoints: &[
            "https://rpc.mainnet.x1.xyz",
        ],
        program_ids: ProgramIds {
            mint_program_id: "8iq6zqaEVcfaym2u8t939PAN5jmfPVc6Z333RuxKTTZX",
            burn_program_id: "2sb3gz5Cmr2g1ia5si2rmCZqPACxgaZXEmiS5k6Htcvh",
            chat_program_id: "Hni4qE8GGW5uwBWzUEkpPBDRwXvKCWhM96teieAReRyd",
            profile_program_id: "2BY8vPpQRFFwAqK3HqU5qL3qsGMH3VnX9Gv9bud3vzH8",
            project_program_id: "6Vavot6ybhWBG3rjNXnLfNRPVTz7Garf6E4EZk3byp3a",
            blog_program_id: "3EKdp88FgyPC41bxRDzFAtCDUMV2g9SVt5UiytE8wdzM",
            forum_program_id: "6gzhG5BveTkJfTi466toX4qmN3BtU9qp1Grnk61GvmXD",
            token_mint: "memoX1sJsBY6od7CfQ58XooRALwnocAZen4L7mW1ick",
            token_2022_program_id: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
        },
    };

    /// Get network configuration for specific network type
    pub fn for_network(network: NetworkType) -> &'static NetworkConfig {
        match network {
            NetworkType::Testnet => &Self::TESTNET,
            NetworkType::ProdStaging => &Self::PROD_STAGING,
            NetworkType::Mainnet => &Self::MAINNET,
        }
    }
}

impl NetworkType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            NetworkType::Testnet => "testnet",
            NetworkType::ProdStaging => "prod-staging",
            NetworkType::Mainnet => "mainnet",
        }
    }

    /// Check if this is a production environment
    pub fn is_production(&self) -> bool {
        matches!(self, NetworkType::ProdStaging | NetworkType::Mainnet)
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            NetworkType::Testnet => "Testnet",
            NetworkType::ProdStaging => "Production Staging",
            NetworkType::Mainnet => "Mainnet",
        }
    }

    /// Get description for UI
    pub fn description(&self) -> &'static str {
        match self {
            NetworkType::Testnet => "Development and testing environment",
            NetworkType::ProdStaging => "Final testing with mainnet contracts on testnet",
            NetworkType::Mainnet => "Production environment - real assets",
        }
    }
}

/// Network state management - can only be set once during login
struct NetworkState {
    current: RwLock<Option<NetworkType>>,
}

impl NetworkState {
    const fn new() -> Self {
        Self {
            current: RwLock::new(None),
        }
    }

    /// Initialize network - can only be called once during login
    /// Returns true if successfully set, false if already set
    fn initialize(&self, network: NetworkType) -> bool {
        let mut current = self.current.write().unwrap();
        if current.is_some() {
            log::warn!("Attempted to change network after initialization. Network is locked.");
            return false;
        }
        *current = Some(network);
        log::info!("===========================================");
        log::info!("Network initialized: {} ({})", 
                   network.display_name(), 
                   if network.is_production() { "PRODUCTION" } else { "DEVELOPMENT" });
        log::info!("RPC: {}", NetworkConfig::for_network(network).rpc_endpoints[0]);
        log::info!("Network is locked until logout");
        log::info!("===========================================");
        true
    }

    /// Clear network - called during logout
    fn clear(&self) {
        let mut current = self.current.write().unwrap();
        if let Some(network) = *current {
            log::info!("Network cleared: {}. Can select network on next login.", network.display_name());
        }
        *current = None;
    }

    /// Get current network
    fn get(&self) -> Option<NetworkType> {
        *self.current.read().unwrap()
    }
}

/// Global network state
static NETWORK_STATE: Lazy<NetworkState> = Lazy::new(NetworkState::new);

// ============ Public API ============

/// Initialize network during login - can only be called once
/// Returns true if successful, false if network already set
pub fn initialize_network(network: NetworkType) -> bool {
    NETWORK_STATE.initialize(network)
}

/// Clear network during logout - allows selecting network again on next login
pub fn clear_network() {
    NETWORK_STATE.clear();
}

/// Get current network type
/// Returns None if not initialized (before login)
pub fn get_network() -> Option<NetworkType> {
    NETWORK_STATE.get()
}

/// Get network configuration for current network
/// Panics if network not initialized - should only be called after login
pub fn get_network_config() -> &'static NetworkConfig {
    let network = get_network()
        .expect("Network not initialized. Must call initialize_network() during login.");
    NetworkConfig::for_network(network)
}

/// Get program IDs for current network
/// Panics if network not initialized - should only be called after login
pub fn get_program_ids() -> &'static ProgramIds {
    &get_network_config().program_ids
}

/// Try to get network config safely (returns None if not initialized)
pub fn try_get_network_config() -> Option<&'static NetworkConfig> {
    get_network().map(NetworkConfig::for_network)
}
