// Configuration constants for the application

// X1 testnet RPC endpoint
pub const X1_TESTNET_RPC_URL: &str = "https://rpc.testnet.x1.xyz";

// Network name
pub const NETWORK_NAME: &str = "X1 Testnet";

// Token symbol
pub const TOKEN_SYMBOL: &str = "XNT";

// Explorer URL base
pub const EXPLORER_URL: &str = "https://explorer.x1.xyz";

// Function to get explorer URL for an address
pub fn get_explorer_address_url(address: &str) -> String {
    format!("{}/address/{}", EXPLORER_URL, address)
} 