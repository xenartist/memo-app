use dioxus::prelude::*;
use crate::rpc::RpcService;
use crate::config;

// Function to fetch balance from RPC
pub fn fetch_balance(address: String, mut balance: Signal<Option<f64>>, mut is_loading: Signal<bool>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen_futures::spawn_local;
        
        is_loading.set(true);
        log::info!("Fetching balance for address: {}", address);
        
        let rpc_service = RpcService::new();
        
        spawn_local(async move {
            match rpc_service.get_balance(&address).await {
                Ok(bal) => {
                    log::info!("Balance fetched: {} {}", bal, config::TOKEN_SYMBOL);
                    balance.set(Some(bal));
                },
                Err(e) => {
                    log::error!("Failed to fetch balance: {}", e);
                    // Keep the old balance, don't set to None
                }
            }
            
            is_loading.set(false);
        });
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        log::warn!("Balance fetching not implemented for desktop/mobile");
    }
} 