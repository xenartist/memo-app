use crate::config;
use log::{info, error};
use serde::{Deserialize, Serialize};

// RPC service for interacting with X1 blockchain
pub struct RpcService {
    endpoint: String,
}

// Response structure for getBalance RPC call
#[derive(Deserialize, Debug)]
struct BalanceResponse {
    jsonrpc: String,
    id: u64,
    result: BalanceResult,
}

#[derive(Deserialize, Debug)]
struct BalanceResult {
    context: Context,
    value: u64,
}

#[derive(Deserialize, Debug)]
struct Context {
    slot: u64,
}

// Request structure for RPC calls
#[derive(Serialize)]
struct RpcRequest<'a, T> {
    jsonrpc: &'a str,
    id: u64,
    method: &'a str,
    params: T,
}

impl RpcService {
    // Create a new RPC service with the default endpoint
    pub fn new() -> Self {
        Self {
            endpoint: config::X1_TESTNET_RPC_URL.to_string(),
        }
    }
    
    // Create a new RPC service with a custom endpoint
    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }
    
    // Get the balance of a wallet address
    #[cfg(target_arch = "wasm32")]
    pub async fn get_balance(&self, address: &str) -> Result<f64, String> {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestMode, Response};
        use js_sys::{JSON, Object, Reflect};
        
        info!("Fetching balance for address: {}", address);
        
        let mut opts = RequestInit::new();
        opts.method("POST");
        opts.mode(RequestMode::Cors);
        
        // Create the request body
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "getBalance",
            params: [address],
        };
        
        let request_json = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;
            
        opts.body(Some(&JsValue::from_str(&request_json)));
        
        let request = Request::new_with_str_and_init(&self.endpoint, &opts)
            .map_err(|e| format!("Failed to create request: {:?}", e))?;
            
        request.headers().set("Content-Type", "application/json")
            .map_err(|e| format!("Failed to set headers: {:?}", e))?;
            
        let window = web_sys::window().ok_or("No window found")?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("Failed to fetch: {:?}", e))?;
            
        let resp: Response = resp_value.dyn_into()
            .map_err(|_| "Response is not a Response object".to_string())?;
            
        if !resp.ok() {
            return Err(format!("HTTP error: {}", resp.status()));
        }
        
        let json = JsFuture::from(resp.json().map_err(|e| format!("Failed to parse JSON: {:?}", e))?)
            .await
            .map_err(|e| format!("Failed to await JSON: {:?}", e))?;
            
        // Extract the balance value from the response
        let result = Reflect::get(&json, &JsValue::from_str("result"))
            .map_err(|_| "No result field in response".to_string())?;
            
        let value = Reflect::get(&result, &JsValue::from_str("value"))
            .map_err(|_| "No value field in result".to_string())?;
            
        let balance_lamports = value.as_f64()
            .ok_or("Value is not a number".to_string())?;
            
        // Convert from lamports to SOL (1 SOL = 10^9 lamports)
        let balance_sol = balance_lamports / 1_000_000_000.0;
        
        info!("Balance fetched successfully: {} XNT", balance_sol);
        Ok(balance_sol)
    }
    
    // Non-wasm implementation (for desktop/mobile)
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn get_balance(&self, _address: &str) -> Result<f64, String> {
        error!("RPC calls not implemented for desktop/mobile yet");
        Err("Not implemented for this platform".to_string())
    }
} 