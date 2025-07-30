use leptos::*;
use crate::core::rpc_base::RpcConnection;
use crate::core::session::Session;
use crate::core::constants::TOKEN_MINT;
use crate::pages::home_page::HomePage;
use crate::pages::profile_page::ProfilePage;
use crate::pages::settings_page::SettingsPage;
use crate::pages::mint_page::MintPage;
use crate::pages::mint_page_legacy::MintPage as MintPageLegacy;
use crate::pages::burn_page::BurnPage;
use crate::pages::log_view::{LogView, add_log_entry};

use wasm_bindgen::prelude::*;
use web_sys::{window, Navigator, Clipboard};
use std::time::Duration;
use serde_json;

// menu item enum
#[derive(Clone, PartialEq)]
enum MenuItem {
    Home,
    Mint,
    MintLegacy,
    Burn,
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
    
    // Now using global constant - no need to define locally
    
    // get wallet address from session
    let wallet_address = move || {
        session.with(|s| {
            match s.get_public_key() {
                Ok(addr) => addr,
                Err(_) => "Not initialized".to_string()
            }
        })
    };
    
    // get sol balance from session
    let sol_balance = move || {
        session.with(|s| s.get_sol_balance())
    };
    
    let token_balance = move || {
        session.with(|s| s.get_token_balance())
    };
    
    // get username from session
    let profile_status = move || {
        session.with(|s| {
            match s.get_user_profile() {
                Some(profile) => {
                    // Display profile creation status and basic stats
                    format!("Profile Active (Minted: {}, Burned: {})", 
                        profile.total_minted, profile.total_burned)
                },
                None => "No Profile".to_string()
            }
        })
    };
    
    // listen to balance update needed in session
    create_effect(move |_| {
        let needs_update = session.with(|s| s.is_balance_update_needed());
        if needs_update {
            log::info!("Balance update needed, fetching latest balances...");
            let session_clone = session;
            spawn_local(async move {
                let mut session_update = session_clone.get_untracked();
                match session_update.fetch_and_update_balances().await {
                    Ok(()) => {
                        log::info!("Successfully updated balances");
                        // update balance info in session
                        session_clone.update(|s| {
                            s.set_balances(session_update.get_sol_balance(), session_update.get_token_balance());
                        });
                    },
                    Err(e) => {
                        log::error!("Failed to update balances: {}", e);
                    }
                }
            });
        }
    });
    
    // test rpc connection
    spawn_local(async move {
        let addr = session.get_untracked().get_public_key().unwrap_or_else(|_| "Not initialized".to_string());
        
        add_log_entry("INFO", "Starting RPC connection tests");
        let rpc = RpcConnection::new();
        
        // initial fetch balance and set to session
        let mut session_update = session.get_untracked();
        match session_update.fetch_and_update_balances().await {
            Ok(()) => {
                log::info!("Initial balance fetch successful");
                session.update(|s| {
                    s.set_balances(session_update.get_sol_balance(), session_update.get_token_balance());
                });
                add_log_entry("INFO", &format!("SOL balance: {}", session_update.get_sol_balance()));
                add_log_entry("INFO", &format!("Token balance: {}", session_update.get_token_balance()));
            },
            Err(e) => {
                log::error!("Failed to fetch initial balances: {}", e);
                add_log_entry("ERROR", &format!("Failed to get balances: {}", e));
            }
        }
        
        // test getVersion
        match rpc.get_version().await {
            Ok(version) => {
                set_version_status.set(format!("✅ RPC Version: {}", version));
                add_log_entry("INFO", &format!("RPC version retrieved: {}", version));
            }
            Err(e) => {
                set_version_status.set(format!("❌ RPC Version Error: {}", e));
                add_log_entry("ERROR", &format!("Failed to get RPC version: {}", e));
            }
        }

        // test getLatestBlockhash
        match rpc.get_latest_blockhash().await {
            Ok(blockhash) => {
                set_blockhash_status.set(format!("✅ Latest Blockhash: {}", blockhash));
                add_log_entry("INFO", &format!("Latest blockhash retrieved: {}", blockhash));
            }
            Err(e) => {
                set_blockhash_status.set(format!("❌ Blockhash Error: {}", e));
                add_log_entry("ERROR", &format!("Failed to get latest blockhash: {}", e));
            }
        }
    });

    // copy address to clipboard
    let copy_address = move |_| {
        let addr = session.with_untracked(|s| {
            s.get_public_key().unwrap_or_else(|_| "Not initialized".to_string())
        });
        
        if let Some(window) = window() {
            let navigator = window.navigator();
            let clipboard = navigator.clipboard();
            let _ = clipboard.write_text(&addr);
            
            add_log_entry("INFO", "Address copied to clipboard");
            
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
                // Temporarily commented out profile info for release
                /*
                <div class="user-info">
                    <span class="profile-status">{profile_status}</span>
                </div>
                */
                <div class="wallet-address">
                    <span class="token-balance">{move || format!("{:.2} MEMO", token_balance())}</span>
                    <span class="balance">{move || format!("{:.4} SOL", sol_balance())}</span>
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
                        class:active=move || current_menu.get() == MenuItem::Mint
                        on:click=move |_| set_current_menu.set(MenuItem::Mint)
                    >
                        <i class="fas fa-hammer"></i>
                        <span>"Mint"</span>
                    </div>
                    // Temporarily commented out for release - only showing Home and Mint
                    /*
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::MintLegacy
                        on:click=move |_| set_current_menu.set(MenuItem::MintLegacy)
                    >
                        <i class="fas fa-history"></i>
                        <span>"Mint (legacy)"</span>
                    </div>
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Burn
                        on:click=move |_| set_current_menu.set(MenuItem::Burn)
                    >
                        <i class="fas fa-fire"></i>
                        <span>"Burn"</span>
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
                    */
                </div>

                <div class="content">
                    <div style=move || if current_menu.get() == MenuItem::Home { "display: block;" } else { "display: none;" }>
                        <HomePage session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Mint { "display: block;" } else { "display: none;" }>
                        <MintPage session=session/>
                    </div>
                    // Temporarily commented out for release - only showing Home and Mint content
                    /*
                    <div style=move || if current_menu.get() == MenuItem::MintLegacy { "display: block;" } else { "display: none;" }>
                        <MintPageLegacy session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Burn { "display: block;" } else { "display: none;" }>
                        <BurnPage session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Profile { "display: block;" } else { "display: none;" }>
                        <ProfilePage session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Settings { "display: block;" } else { "display: none;" }>
                        <SettingsPage/>
                    </div>
                    */
                </div>
            </div>

            // Global log viewer - always visible at the bottom
            <LogView/>
        </div>
    }
} 