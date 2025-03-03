use serde_json::json;

// RPC client for Solana API calls
pub struct RpcClient {
    endpoint: String,
}

impl RpcClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }

    // Get default testnet client
    pub fn default_testnet() -> Self {
        Self::new("https://rpc.testnet.x1.xyz")
    }

    // Query balance for a wallet address
    pub fn get_balance(&self, address: &str) -> Result<f64, String> {
        let client = reqwest::blocking::Client::new();
        
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [address]
        });

        let response = client
            .post(&self.endpoint)
            .json(&request_body)
            .send()
            .map_err(|e| format!("Failed to send request: {}", e))?;

        let response_json: serde_json::Value = response
            .json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = response_json.get("error") {
            return Err(format!("RPC error: {}", error));
        }

        let lamports = response_json
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "Invalid response format".to_string())?;

        // Convert lamports to SOL (1 SOL = 1_000_000_000 lamports)
        Ok(lamports as f64 / 1_000_000_000.0)
    }
} 