use serde::{Serialize, Deserialize};
use reqwest::Client;
use std::fmt;

// error type
#[derive(Debug)]
pub enum RpcError {
    ConnectionFailed(String),
    InvalidAddress(String),
    TransactionFailed(String),
    Other(String),
}

// implement the display for the rpc error
impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            RpcError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            RpcError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            RpcError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

// define the rpc response error structure
#[derive(Deserialize, Debug)]
struct RpcResponseError {
    code: i64,
    message: String,
}

pub struct RpcConnection {
    client: Client,
    endpoint: String,
}

#[derive(Serialize)]
struct RpcRequest<T> {
    jsonrpc: String,
    id: u64,
    method: String,
    params: T,
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    jsonrpc: String,
    id: u64,
    result: T,
    #[serde(default)]
    error: Option<RpcResponseError>,
}

impl RpcConnection {
    // X1 testnet RPC endpoint
    const DEFAULT_RPC_ENDPOINT: &'static str = "https://rpc.testnet.x1.xyz";
    
    pub fn new() -> Self {
        Self::with_endpoint(Self::DEFAULT_RPC_ENDPOINT)
    }

    pub fn with_endpoint(endpoint: &str) -> Self {
        let client = Client::new();
        Self {
            client,
            endpoint: endpoint.to_string(),
        }
    }

    // get the account balance
    pub async fn get_balance(&self, pubkey: &str) -> Result<u64, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getBalance".to_string(),
            params: vec![pubkey],
        };

        let response = self.client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| RpcError::ConnectionFailed(e.to_string()))?;

        let result: RpcResponse<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| RpcError::Other(e.to_string()))?;

        if let Some(error) = result.error {
            return Err(RpcError::Other(error.message));
        }

        result.result
            .as_u64()
            .ok_or_else(|| RpcError::Other("Invalid balance format".to_string()))
    }

    // get the latest block hash
    pub async fn get_latest_blockhash(&self) -> Result<String, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getLatestBlockhash".to_string(),
            params: Vec::<String>::new(),
        };

        let response = self.client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| RpcError::ConnectionFailed(e.to_string()))?;

        #[derive(Deserialize)]
        struct BlockhashResult {
            blockhash: String,
        }

        let result: RpcResponse<BlockhashResult> = response
            .json()
            .await
            .map_err(|e| RpcError::Other(e.to_string()))?;

        if let Some(error) = result.error {
            return Err(RpcError::Other(error.message));
        }

        Ok(result.result.blockhash)
    }

    // send a signed transaction
    pub async fn send_transaction(&self, serialized_tx: &str) -> Result<String, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "sendTransaction".to_string(),
            params: vec![serialized_tx],
        };

        let response = self.client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| RpcError::ConnectionFailed(e.to_string()))?;

        let result: RpcResponse<String> = response
            .json()
            .await
            .map_err(|e| RpcError::Other(e.to_string()))?;

        if let Some(error) = result.error {
            return Err(RpcError::TransactionFailed(error.message));
        }

        Ok(result.result)
    }

    // get the transaction status
    pub async fn get_transaction_status(&self, signature: &str) -> Result<String, RpcError> {
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getSignatureStatuses".to_string(),
            params: vec![vec![signature]],
        };

        let response = self.client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| RpcError::ConnectionFailed(e.to_string()))?;

        let result: RpcResponse<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| RpcError::Other(e.to_string()))?;

        if let Some(error) = result.error {
            return Err(RpcError::Other(error.message));
        }

        Ok(result.result.to_string())
    }
}

// implement the default trait
impl Default for RpcConnection {
    fn default() -> Self {
        Self::new()
    }
} 