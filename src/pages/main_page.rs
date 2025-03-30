use leptos::*;
use crate::core::rpc::RpcConnection;
use crate::core::session::Session;
use wasm_bindgen::prelude::*;
use web_sys::{window, Navigator, Clipboard};
use std::time::Duration;
use serde_json;

#[component]
pub fn MainPage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (version_status, set_version_status) = create_signal(String::from("Testing RPC connection..."));
    let (blockhash_status, set_blockhash_status) = create_signal(String::from("Getting latest blockhash..."));
    
    let (show_copied, set_show_copied) = create_signal(false);
    
    let (balance, set_balance) = create_signal(0f64);
    
    // get wallet address from session
    let wallet_address = move || {
        match session.get().get_public_key() {
            Ok(addr) => addr,
            Err(_) => "Not initialized".to_string()
        }
    };
    
    // test rpc connection
    spawn_local(async move {
        let rpc = RpcConnection::new();
        let addr = wallet_address();
        
        // get balance
        match rpc.get_balance(&addr).await {
            Ok(balance_result) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&balance_result) {
                    if let Some(lamports) = json.get("value").and_then(|v| v.as_u64()) {
                        let sol = lamports as f64 / 1_000_000_000.0;
                        set_balance.set(sol);
                    }
                }
            }
            Err(e) => {
            }
        }
        
        // test getVersion
        match rpc.get_version().await {
            Ok(version) => {
                set_version_status.set(format!("✅ RPC Version: {}", version));
            }
            Err(e) => {
                set_version_status.set(format!("❌ RPC Version Error: {}", e));
            }
        }

        // test getLatestBlockhash
        match rpc.get_latest_blockhash().await {
            Ok(blockhash) => {
                set_blockhash_status.set(format!("✅ Latest Blockhash: {}", blockhash));
            }
            Err(e) => {
                set_blockhash_status.set(format!("❌ Blockhash Error: {}", e));
            }
        }
    });

    // copy address to clipboard
    let copy_address = move |_| {
        let addr = wallet_address();
        if let Some(window) = window() {
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            let _ = clipboard.write_text(&addr);
            
            // show tooltip
            set_show_copied.set(true);
            
            // hide tooltip after 1.5 seconds
            set_timeout(move || {
                set_show_copied.set(false);
            }, Duration::from_millis(1500));
        }
    };

    view! {
        <div class="main-page">
            <div class="top-bar">
                <div class="wallet-address">
                    <span class="balance">{move || format!("{:.4} SOL", balance.get())}</span>
                    <span class="address-label">"Wallet: "</span>
                    <span 
                        class="address-value" 
                        title={move || wallet_address()}
                        on:mousedown=|e| e.prevent_default()
                    >
                        {move || {
                            let addr = wallet_address();
                            format!("{}...{}", &addr[..4], &addr[addr.len()-4..])
                        }}
                    </span>
                    <div class="copy-container">
                        <button
                            class="copy-button"
                            on:click=copy_address
                            on:mousedown=|e| e.prevent_default()
                            title="Copy address to clipboard"
                        >
                            <i class="fas fa-copy"></i>
                        </button>
                        <div 
                            class="copy-tooltip"
                            class:show=move || show_copied.get()
                        >
                            "Copied!"
                        </div>
                    </div>
                </div>
            </div>

            // rpc status
            <div class="rpc-status">
                <h3>"X1 RPC Status"</h3>
                <p>{version_status}</p>
                <p>{blockhash_status}</p>
            </div>
        </div>
    }
} 