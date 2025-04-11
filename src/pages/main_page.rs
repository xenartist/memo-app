use leptos::*;
use crate::core::rpc::RpcConnection;
use crate::core::session::Session;
use crate::pages::home_page::HomePage;
use crate::pages::profile_page::ProfilePage;
use crate::pages::settings_page::SettingsPage;

use wasm_bindgen::prelude::*;
use web_sys::{window, Navigator, Clipboard};
use std::time::Duration;
use serde_json;

// menu item enum
#[derive(Clone, PartialEq)]
enum MenuItem {
    Home,
    Profile,
    Settings,
}

#[component]
pub fn MainPage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (version_status, set_version_status) = create_signal(String::from("Testing RPC connection..."));
    let (blockhash_status, set_blockhash_status) = create_signal(String::from("Getting latest blockhash..."));
    
    let (show_copied, set_show_copied) = create_signal(false);
    
    let (balance, set_balance) = create_signal(0f64);
    
    let (token_balance, set_token_balance) = create_signal(0f64);
    
    // token address
    const TOKEN_MINT: &str = "CrfhYtP7XtqFyHTWMyXp25CCzhjhzojngrPCZJ7RarUz";
    
    // get wallet address from session
    let wallet_address = move || {
        match session.get().get_public_key() {
            Ok(addr) => addr,
            Err(_) => "Not initialized".to_string()
        }
    };
    
    // get username from session
    let username = move || {
        match session.get().get_user_profile() {
            Some(profile) => profile.username,
            None => "N/A".to_string()
        }
    };
    
    // test rpc connection
    spawn_local(async move {
        let rpc = RpcConnection::new();
        let addr = wallet_address();
        
        // get token balance
        match rpc.get_token_balance(&addr, TOKEN_MINT).await {
            Ok(token_result) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&token_result) {
                    // parse token account info
                    if let Some(accounts) = json.get("value").and_then(|v| v.as_array()) {
                        if let Some(first_account) = accounts.first() {
                            if let Some(amount) = first_account
                                .get("account")
                                .and_then(|a| a.get("data"))
                                .and_then(|d| d.get("parsed"))
                                .and_then(|p| p.get("info"))
                                .and_then(|i| i.get("tokenAmount"))
                                .and_then(|t| t.get("uiAmount"))
                                .and_then(|a| a.as_f64())
                            {
                                set_token_balance.set(amount);
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }
        
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

    // current selected menu item
    let (current_menu, set_current_menu) = create_signal(MenuItem::Home);

    view! {
        <div class="main-page">
            <div class="top-bar">
                <div class="user-info">
                    <span class="username">{username}</span>
                </div>
                <div class="wallet-address">
                    <span class="token-balance">{move || format!("{:.2} MEMO", token_balance.get())}</span>
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

            <div class="main-content">
                <div class="sidebar">
                    <div 
                        class="menu-item" 
                        class:active=move || current_menu.get() == MenuItem::Home
                        on:click=move |_| set_current_menu.set(MenuItem::Home)
                    >
                        <i class="fas fa-home"></i>
                        <span>"Home"</span>
                    </div>
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Profile
                        on:click=move |_| set_current_menu.set(MenuItem::Profile)
                    >
                        <i class="fas fa-user"></i>
                        <span>"Profile"</span>
                    </div>
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Settings
                        on:click=move |_| set_current_menu.set(MenuItem::Settings)
                    >
                        <i class="fas fa-cog"></i>
                        <span>"Settings"</span>
                    </div>
                </div>

                <div class="content">
                    {move || match current_menu.get() {
                        MenuItem::Home => view! {
                            <HomePage/>
                        },
                        MenuItem::Profile => view! {
                            <ProfilePage session=session/>
                        },
                        MenuItem::Settings => view! {
                            <SettingsPage/>
                        }
                    }}
                </div>
            </div>
        </div>
    }
} 