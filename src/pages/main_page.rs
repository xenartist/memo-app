use leptos::*;
use crate::core::rpc_base::RpcConnection;
use crate::core::session::Session;
use crate::pages::home_page::HomePage;
use crate::pages::profile_page::ProfilePage;
use crate::pages::settings_page::SettingsPage;
use crate::pages::mint_page::MintPage;
use crate::pages::mint_page_legacy::MintPage as MintPageLegacy;
use crate::pages::burn_page::BurnPage;
use crate::pages::chat_page::ChatPage;
use crate::pages::project_page::ProjectPage;
use crate::pages::faucet_page::FaucetPage;
use crate::pages::log_view::{LogView, add_log_entry};

use wasm_bindgen::prelude::*;
use web_sys::{window, Navigator, Clipboard};
use std::time::Duration;
use serde_json;
use gloo_timers::future::TimeoutFuture;

// menu item enum
#[derive(Clone, PartialEq)]
enum MenuItem {
    Home,
    Mint,
    Project,
    Chat,
    Faucet,
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
    
    // Initialize Burn Stats dialog states
    let (show_init_dialog, set_show_init_dialog) = create_signal(false);
    let (init_loading, set_init_loading) = create_signal(false);
    let (init_message, set_init_message) = create_signal(String::new());
    
    // Welcome info dialog states (for new users)
    let (show_welcome_info, set_show_welcome_info) = create_signal(false);
    let (has_shown_welcome, set_has_shown_welcome) = create_signal(false);
    
    // Force refresh signal to trigger UI updates
    let (force_refresh, set_force_refresh) = create_signal(0u32);
    
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
                    format!("Profile: {}", profile.username)
                },
                None => "No Profile".to_string()
            }
        })
    };
    
    // simplify button display logic, like balance directly from session
    let needs_burn_stats_init = move || {
        session.with(|s| !s.has_burn_stats_initialized())
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
    
    // check and get user profile on startup
    create_effect(move |_| {
        let session_clone = session;
        spawn_local(async move {
            let has_profile = session_clone.with_untracked(|s| s.get_user_profile().is_some());
            
            if !has_profile {
                log::info!("No cached profile found, fetching from blockchain...");
                
                // create temporary session for fetching data
                let mut temp_session = session_clone.get_untracked();
                match temp_session.fetch_and_cache_user_profile().await {
                    Ok(Some(_)) => {
                        log::info!("User profile loaded successfully on startup");
                        // use update instead of set, avoid overwriting other updates
                        session_clone.update(|s| {
                            if let Some(profile) = temp_session.get_user_profile() {
                                s.set_user_profile(Some(profile));
                            }
                        });
                    },
                    Ok(None) => {
                        log::info!("No user profile exists on blockchain");
                        // mark as checked (maybe not needed)
                    },
                    Err(e) => {
                        log::warn!("Failed to fetch user profile on startup: {}", e);
                    }
                }
            }
        });
    });
    
    // simplify burn stats check logic  
    create_effect(move |_| {
        let session_clone = session;
        spawn_local(async move {
            let has_burn_stats = session_clone.with_untracked(|s| s.has_burn_stats_initialized());
            
            if !has_burn_stats {
                log::info!("Burn stats not initialized, fetching from blockchain...");
                
                // create temporary session for fetching data
                let mut temp_session = session_clone.get_untracked();
                match temp_session.fetch_and_cache_user_burn_stats().await {
                    Ok(Some(_)) => {
                        log::info!("User burn stats loaded successfully on startup");
                        // use update instead of set, avoid overwriting other updates
                        session_clone.update(|s| {
                            if let Some(stats) = temp_session.get_user_burn_stats() {
                                *s = temp_session; // or more precise update
                            }
                        });
                    },
                    Ok(None) => {
                        log::info!("No user burn stats exist on blockchain");
                        // Show welcome info dialog after a short delay
                        if !has_shown_welcome.get_untracked() {
                            set_timeout(move || {
                                set_show_welcome_info.set(true);
                                set_has_shown_welcome.set(true);
                            }, Duration::from_millis(1500));
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to fetch user burn stats on startup: {}", e);
                    }
                }
            } else {
                log::info!("Burn stats already initialized in session, skipping fetch");
            }
        });
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
                add_log_entry("INFO", &format!("XNT balance: {}", session_update.get_sol_balance()));
                add_log_entry("INFO", &format!("MEMO balance: {}", session_update.get_token_balance()));
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

    // initialize burn stats handler
    let initialize_burn_stats = move |_| {
        // 1. immediately update UI state (sync)
        set_init_loading.set(true);
        set_init_message.set("Initializing Burn Stats...".to_string());
        
        let session_clone = session;
        
        // 2. async delay execution (give UI time to update)
        spawn_local(async move {
            // add short delay, let UI have time to update state, avoid lag
            TimeoutFuture::new(100).await;
            
            let mut session_update = session_clone.get_untracked();
            
            match session_update.initialize_user_burn_stats().await {
                Ok(tx_hash) => {
                    log::info!("Burn stats initialized successfully: {}", tx_hash);
                    add_log_entry("INFO", &format!("Burn stats initialized: {}", tx_hash));
                    
                    // Fetch and cache the newly initialized burn stats
                    match session_update.fetch_and_cache_user_burn_stats().await {
                        Ok(Some(_)) => {
                            log::info!("Successfully fetched and cached burn stats after initialization");
                            // Update session with the new data including burn stats
                            session_clone.set(session_update);
                            
                            // Force UI refresh to hide the button immediately
                            set_force_refresh.update(|n| *n += 1);
                            
                            set_init_message.set("Initialization successful!".to_string());
                            
                            // Close dialog after 2 seconds
                            set_timeout(move || {
                                set_show_init_dialog.set(false);
                                set_init_loading.set(false);
                                set_init_message.set(String::new());
                            }, Duration::from_millis(2000));
                        },
                        Ok(None) => {
                            log::warn!("Burn stats initialized but not found when fetching");
                            // Still update session but show a warning
                            session_clone.set(session_update);
                            set_init_message.set("Initialization completed, but stats not immediately available.".to_string());
                            set_init_loading.set(false);
                        },
                        Err(e) => {
                            log::error!("Failed to fetch burn stats after initialization: {}", e);
                            // Still update session as initialization was successful
                            session_clone.set(session_update);
                            set_init_message.set("Initialization successful!".to_string());
                            
                            // Close dialog after 2 seconds
                            set_timeout(move || {
                                set_show_init_dialog.set(false);
                                set_init_loading.set(false);
                                set_init_message.set(String::new());
                            }, Duration::from_millis(2000));
                        }
                    }
                },
                Err(e) => {
                    log::error!("Failed to initialize burn stats: {}", e);
                    add_log_entry("ERROR", &format!("Failed to initialize burn stats: {}", e));
                    set_init_message.set(format!("Initialization failed: {}", e));
                    set_init_loading.set(false);
                }
            }
        });
    };

    // current selected menu item - changed default from Home to Mint
    let (current_menu, set_current_menu) = create_signal(MenuItem::Mint);

    view! {
        <div class="main-page">
            <div class="top-bar">
                // Left side - Initialize Burn Stats button (only show if needed)
                <div class="left-controls">
                    <Show when=move || needs_burn_stats_init()>
                        <button
                            class="init-burn-stats-btn"
                            on:click=move |_| set_show_init_dialog.set(true)
                            title="Initialize your burn statistics"
                        >
                            <i class="fas fa-fire"></i>
                            <span>"Initialize Burn Stats"</span>
                        </button>
                    </Show>
                </div>
                
                // Temporarily commented out profile info for release
                /*
                <div class="user-info">
                    <span class="profile-status">{profile_status}</span>
                </div>
                */
                
                // Right side - wallet info
                <div class="wallet-address">
                    <span class="token-balance">{move || format!("{:.2} MEMO", token_balance())}</span>
                    <span class="balance">{move || format!("{:.4} XNT", sol_balance())}</span>
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
                    // Temporarily commented out Home menu item for release
                    /*
                    <div 
                        class="menu-item" 
                        class:active=move || current_menu.get() == MenuItem::Home
                        on:click=move |_| set_current_menu.set(MenuItem::Home)
                    >
                        <i class="fas fa-home"></i>
                        <span>"Home"</span>
                    </div>
                    */
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Mint
                        on:click=move |_| set_current_menu.set(MenuItem::Mint)
                    >
                        <i class="fas fa-hammer"></i>
                        <span>"Mint"</span>
                    </div>
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Project
                        on:click=move |_| set_current_menu.set(MenuItem::Project)
                    >
                        <i class="fas fa-project-diagram"></i>
                        <span>"Project"</span>
                    </div>
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Chat
                        on:click=move |_| set_current_menu.set(MenuItem::Chat)
                    >
                        <i class="fas fa-comments"></i>
                        <span>"Chat"</span>
                    </div>
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Faucet
                        on:click=move |_| set_current_menu.set(MenuItem::Faucet)
                    >
                        <i class="fas fa-faucet"></i>
                        <span>"Faucet (testnet)"</span>
                    </div>
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Profile
                        on:click=move |_| set_current_menu.set(MenuItem::Profile)
                    >
                        <i class="fas fa-user"></i>
                        <span>"Profile"</span>
                    </div>
                    // Temporarily commented out for release - only showing Mint, Chat and Profile
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
                    <div style=move || if current_menu.get() == MenuItem::Project { "display: block;" } else { "display: none;" }>
                        <ProjectPage session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Chat { "display: block;" } else { "display: none;" }>
                        <ChatPage session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Faucet { "display: block;" } else { "display: none;" }>
                        <FaucetPage session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Profile { "display: block;" } else { "display: none;" }>
                        <ProfilePage session=session/>
                    </div>
                    // Temporarily commented out for release - only showing Home, Mint, Chat and Profile content
                    /*
                    <div style=move || if current_menu.get() == MenuItem::MintLegacy { "display: block;" } else { "display: none;" }>
                        <MintPageLegacy session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Burn { "display: block;" } else { "display: none;" }>
                        <BurnPage session=session/>
                    </div>
                    <div style=move || if current_menu.get() == MenuItem::Settings { "display: block;" } else { "display: none;" }>
                        <SettingsPage/>
                    </div>
                    */
                </div>
            </div>

            // Global log viewer - always visible at the bottom
            <LogView/>
            
            // Welcome Info Dialog (shown after login/registration if burn stats not initialized)
            <Show when=move || show_welcome_info.get()>
                <div class="modal-overlay" on:click=move |_| set_show_welcome_info.set(false)>
                    <div class="modal-content welcome-info-dialog" on:click=|e| e.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Welcome to MEMO App!"</h3>
                        </div>
                        
                        <div class="modal-body">
                            <div class="welcome-info-content">
                                <div class="info-icon">
                                    <i class="fas fa-fire"></i>
                                </div>
                                <div class="info-text">
                                    <h4>"Initialize Burn Statistics"</h4>
                                    <p>"To use burn features in this app, you need to initialize your Burn Statistics first."</p>
                                    <p>
                                        "You can find the red "
                                        <strong>"Initialize Burn Stats"</strong>
                                        " button in the top-left corner of your screen."
                                    </p>
                                    <p class="info-note">"💡 This is a one-time setup that creates your personal burn statistics record on the blockchain."</p>
                                </div>
                            </div>
                        </div>
                        
                        <div class="modal-footer">
                            <button 
                                class="btn-primary got-it-btn"
                                on:click=move |_| set_show_welcome_info.set(false)
                            >
                                <i class="fas fa-check"></i>
                                "Got it!"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
            
            // Initialize Burn Stats Dialog
            <Show when=move || show_init_dialog.get()>
                <div class="modal-overlay" on:click=move |_| {
                    if !init_loading.get() {
                        set_show_init_dialog.set(false);
                        set_init_message.set(String::new());
                    }
                }>
                    <div class="modal-content init-burn-stats-dialog" on:click=|e| e.stop_propagation()>
                        <div class="modal-header">
                            <h3>"Initialize Burn Statistics"</h3>
                            <Show when=move || !init_loading.get()>
                                <button 
                                    class="modal-close"
                                    on:click=move |_| {
                                        set_show_init_dialog.set(false);
                                        set_init_message.set(String::new());
                                    }
                                >
                                    "×"
                                </button>
                            </Show>
                        </div>
                        
                        <div class="modal-body">
                            <Show 
                                when=move || init_message.get().is_empty() && !init_loading.get()
                                fallback=move || view! {
                                    <div class="init-status">
                                        <Show when=move || init_loading.get()>
                                            <div class="loading-spinner"></div>
                                        </Show>
                                        <p class="init-message">{init_message}</p>
                                    </div>
                                }
                            >
                                <div class="init-explanation">
                                    <p>"Please initialize your Burn Statistics to use burn features."</p>
                                    <p>"This operation only needs to be performed once and will create your personal burn statistics record on the blockchain."</p>
                                    <p class="warning-text">"⚠️ This operation requires a small amount of XNT for transaction fees."</p>
                                </div>
                            </Show>
                        </div>
                        
                        <Show when=move || !init_loading.get() && init_message.get().is_empty()>
                            <div class="modal-footer">
                                <button 
                                    class="btn-secondary"
                                    on:click=move |_| {
                                        set_show_init_dialog.set(false);
                                        set_init_message.set(String::new());
                                    }
                                >
                                    "Cancel"
                                </button>
                                <button 
                                    class="btn-primary init-confirm-btn"
                                    on:click=initialize_burn_stats
                                >
                                    <i class="fas fa-fire"></i>
                                    "Initialize"
                                </button>
                            </div>
                        </Show>
                    </div>
                </div>
            </Show>
        </div>
    }
} 
