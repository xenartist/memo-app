use leptos::*;
use crate::core::rpc::RpcConnection;
use crate::core::session::Session;
use wasm_bindgen::prelude::*;
use web_sys::{window, Navigator, Clipboard};

#[component]
pub fn MainPage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (version_status, set_version_status) = create_signal(String::from("Testing RPC connection..."));
    let (blockhash_status, set_blockhash_status) = create_signal(String::from("Getting latest blockhash..."));
    
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

    // 添加复制功能
    let copy_address = move |_| {
        let addr = wallet_address();
        if let Some(window) = window() {
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            let _ = clipboard.write_text(&addr);
        }
    };

    view! {
        <div class="main-page">
            <div class="top-bar">
                <div class="wallet-address">
                    <span class="address-label">"Wallet: "</span>
                    <span class="address-value" title={move || wallet_address()}>
                        {move || {
                            let addr = wallet_address();
                            format!("{}...{}", &addr[..4], &addr[addr.len()-4..])
                        }}
                    </span>
                    <button
                        class="copy-button"
                        on:click=copy_address
                        title="Copy address to clipboard"
                    >
                        "📋"
                    </button>
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