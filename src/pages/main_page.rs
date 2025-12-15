use leptos::*;
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_domain::get_primary_domain;
use crate::core::session::Session;
use crate::core::NetworkType;
use crate::pages::profile_page::ProfilePage;
use crate::pages::settings_page::SettingsPage;
use crate::pages::mint_page::MintPage;
use crate::pages::chat_page::ChatPage;
use crate::pages::project_page::ProjectPage;
use crate::pages::faucet_page::FaucetPage;
use crate::pages::log_view::{LogView, add_log_entry};
use crate::pages::pixel_view::LazyPixelView;

use web_sys::window;
use std::time::Duration;
use gloo_timers::future::TimeoutFuture;

// menu item enum
#[derive(Clone, PartialEq)]
enum MenuItem {
    Mint,
    Project,
    Chat,
    Faucet,
    Profile,
    Settings,
}

// Helper function to check if a menu item is available for the current network
fn is_menu_available(menu_item: &MenuItem, network: Option<NetworkType>) -> bool {
    match network {
        Some(NetworkType::Testnet) => {
            // Testnet: All features available
            true
        }
        Some(NetworkType::ProdStaging) | Some(NetworkType::Mainnet) => {
            // Production and Staging: Mint, Chat, Profile, and Settings available
            matches!(menu_item, MenuItem::Mint | MenuItem::Chat | MenuItem::Profile | MenuItem::Settings)
        }
        None => {
            // If network not set (shouldn't happen), default to restricted mode
            matches!(menu_item, MenuItem::Mint | MenuItem::Settings)
        }
    }
}

#[component]
pub fn MainPage(
    session: RwSignal<Session>,
    on_logout: impl Fn() + 'static,
    on_lock_screen: impl Fn() + 'static
) -> impl IntoView {
    // Store callbacks to avoid ownership issues in <Show> components
    let on_logout = store_value(on_logout);
    let on_lock_screen = store_value(on_lock_screen);
    
    let (_version_status, set_version_status) = create_signal(String::from("Testing RPC connection..."));
    let (_blockhash_status, set_blockhash_status) = create_signal(String::from("Getting latest blockhash..."));
    
    let (show_copied, set_show_copied) = create_signal(false);
    
    // Theme state - true for dark mode, false for light mode
    let (is_dark_mode, set_is_dark_mode) = create_signal(false);
    
    // Initialize theme from localStorage on component mount
    create_effect(move |_| {
        if let Some(window) = window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(theme)) = storage.get_item("theme") {
                    let is_dark = theme == "dark";
                    set_is_dark_mode.set(is_dark);
                    // Apply theme to document
                    if let Some(document) = window.document() {
                        if let Some(html) = document.document_element() {
                            let _ = html.set_attribute("data-theme", if is_dark { "dark" } else { "light" });
                        }
                    }
                }
            }
        }
    });
    
    // Theme toggle handler
    let toggle_theme = move |_| {
        let new_is_dark = !is_dark_mode.get();
        set_is_dark_mode.set(new_is_dark);
        
        if let Some(window) = window() {
            // Save to localStorage
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("theme", if new_is_dark { "dark" } else { "light" });
            }
            // Apply theme to document
            if let Some(document) = window.document() {
                if let Some(html) = document.document_element() {
                    let _ = html.set_attribute("data-theme", if new_is_dark { "dark" } else { "light" });
                }
            }
        }
        
        add_log_entry("INFO", &format!("Theme changed to {}", if new_is_dark { "Dark Mode" } else { "Light Mode" }));
    };
    
    // Primary domain from X1NS
    let (primary_domain, set_primary_domain) = create_signal(Option::<String>::None);
    
    // Initialize Burn Stats dialog states
    let (show_init_dialog, set_show_init_dialog) = create_signal(false);
    let (init_loading, set_init_loading) = create_signal(false);
    let (init_message, set_init_message) = create_signal(String::new());
    
    // Welcome info dialog states (for new users)
    let (show_welcome_info, set_show_welcome_info) = create_signal(false);
    let (has_shown_welcome, set_has_shown_welcome) = create_signal(false);
    
    // Force refresh signal to trigger UI updates
    let (_force_refresh, set_force_refresh) = create_signal(0u32);
    
    // Transfer dialog states
    let (show_transfer_dialog, set_show_transfer_dialog) = create_signal(false);
    let (transfer_type, set_transfer_type) = create_signal("MEMO".to_string());
    let (transfer_address, set_transfer_address) = create_signal(String::new());
    let (transfer_amount, set_transfer_amount) = create_signal(String::new());
    let (transfer_loading, set_transfer_loading) = create_signal(false);
    let (transfer_message, set_transfer_message) = create_signal(String::new());
    let (transfer_success, set_transfer_success) = create_signal(false);
    let (transfer_tx_hash, set_transfer_tx_hash) = create_signal(String::new());
    let (show_confirm_dialog, set_show_confirm_dialog) = create_signal(false);
    let (confirm_transfer_data, set_confirm_transfer_data) = create_signal(Option::<(String, String, String)>::None);
    
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
    let _profile_status = move || {
        session.with(|s| {
            match s.get_user_profile() {
                Some(profile) => {
                    profile.username.clone()
                },
                None => "No Profile".to_string()
            }
        })
    };
    
    // simplify button display logic, like balance directly from session
    let needs_burn_stats_init = move || {
        session.with(|s| !s.has_burn_stats_initialized())
    };
    
    // Get current network from session
    let current_network = move || {
        session.with(|s| s.get_network())
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
                                s.set_user_burn_stats(Some(stats));
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
    
    // Fetch primary domain from X1NS
    {
        let session_clone = session;
        spawn_local(async move {
            let addr = session_clone.get_untracked().get_public_key().unwrap_or_else(|_| String::new());
            if !addr.is_empty() && addr != "Not initialized" {
                log::info!("Fetching primary domain for address: {}", addr);
                match get_primary_domain(&addr).await {
                    Ok(Some(domain)) => {
                        log::info!("Primary domain found: {}", domain);
                        set_primary_domain.set(Some(domain));
                    },
                    Ok(None) => {
                        log::debug!("No primary domain set for this address");
                    },
                    Err(e) => {
                        log::warn!("Failed to fetch primary domain: {}", e);
                    }
                }
            }
        });
    }
    
    // test rpc connection
    spawn_local(async move {
        let _addr = session.get_untracked().get_public_key().unwrap_or_else(|_| "Not initialized".to_string());
        
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
                let blockhash_str = blockhash.to_string();
                set_blockhash_status.set(format!("✅ Latest Blockhash: {}", blockhash_str));
                add_log_entry("INFO", &format!("Latest blockhash retrieved: {}", blockhash_str));
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

    // logout handler
    let handle_logout_click = move |_| {
        add_log_entry("INFO", "User logged out");
        on_logout.with_value(|f| f());
    };

    // lock screen handler
    let handle_lock_click = move |_| {
        add_log_entry("INFO", "Screen locked");
        on_lock_screen.with_value(|f| f());
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
                    
                    // Wait for on-chain synchronization (20 seconds)
                    log::info!("Waiting 20 seconds for on-chain synchronization...");
                    add_log_entry("INFO", "Waiting for on-chain synchronization...");
                    TimeoutFuture::new(20000).await;
                    
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
                // Left side - Control buttons
                <div class="left-controls">
                    // Theme toggle button
                    <button
                        class="theme-toggle-btn"
                        on:click=toggle_theme
                        title=move || if is_dark_mode.get() { "Switch to Light Mode" } else { "Switch to Dark Mode" }
                    >
                        {move || if is_dark_mode.get() {
                            view! { <><i class="fas fa-sun"></i><span>"Light"</span></> }
                        } else {
                            view! { <><i class="fas fa-moon"></i><span>"Dark"</span></> }
                        }}
                    </button>
                    
                    // Logout button
                    <button
                        class="logout-btn"
                        on:click=handle_logout_click
                        title="Logout and return to login screen"
                    >
                        <i class="fas fa-sign-out-alt"></i>
                        <span>"Logout"</span>
                    </button>
                    
                    // Lock Screen button - only show for internal wallet
                    <Show when=move || session.with(|s| s.is_internal_wallet())>
                        <button
                            class="lock-screen-btn"
                            on:click=handle_lock_click
                            title="Lock screen"
                        >
                            <i class="fas fa-lock"></i>
                            <span>"Lock"</span>
                        </button>
                    </Show>
                    
                    // Initialize Burn Stats button (only show if needed)
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
                
                // Right side - profile avatar and wallet info
                <div class="wallet-address">
                    // Profile avatar
                    {move || {
                        session.with(|s| {
                            match s.get_user_profile() {
                                Some(profile) => {
                                    // Show avatar only
                                    if !profile.image.is_empty() {
                                        view! {
                                            <div class="profile-avatar-small" title={profile.username.clone()}>
                                                <LazyPixelView
                                                    art={profile.image.clone()}
                                                    size=32
                                                />
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div class="profile-avatar-small avatar-default" title={profile.username.clone()}>
                                                <i class="fas fa-user"></i>
                                            </div>
                                        }.into_view()
                                    }
                                },
                                None => {
                                    view! {
                                        <div class="profile-avatar-small avatar-default" title="No Profile">
                                            <i class="fas fa-user"></i>
                                        </div>
                                    }.into_view()
                                }
                            }
                        })
                    }}
                    
                    // Wallet info button
                    <button
                        class="wallet-info-button"
                        on:click=move |_| set_show_transfer_dialog.set(true)
                        title="Click to transfer tokens"
                    >
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
                                let short_addr = format!("{}...{}", &addr[..4], &addr[addr.len()-4..]);
                                if let Some(domain) = primary_domain.get() {
                                    format!("{} ({})", domain, short_addr)
                                } else {
                                    short_addr
                                }
                            }}
                        </span>
                    </button>
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
                    // Mint - always visible
                    <div 
                        class="menu-item"
                        class:active=move || current_menu.get() == MenuItem::Mint
                        on:click=move |_| set_current_menu.set(MenuItem::Mint)
                    >
                        <i class="fas fa-hammer"></i>
                        <span>"Mint"</span>
                    </div>
                    
                    // Project - only in testnet
                    <Show when=move || is_menu_available(&MenuItem::Project, current_network())>
                        <div 
                            class="menu-item"
                            class:active=move || current_menu.get() == MenuItem::Project
                            on:click=move |_| set_current_menu.set(MenuItem::Project)
                        >
                            <i class="fas fa-project-diagram"></i>
                            <span>"Project"</span>
                        </div>
                    </Show>
                    
                    // Chat - available in testnet, staging, and mainnet
                    <Show when=move || is_menu_available(&MenuItem::Chat, current_network())>
                        <div 
                            class="menu-item"
                            class:active=move || current_menu.get() == MenuItem::Chat
                            on:click=move |_| set_current_menu.set(MenuItem::Chat)
                        >
                            <i class="fas fa-comments"></i>
                            <span>"Chat"</span>
                        </div>
                    </Show>
                    
                    // Faucet - only in testnet
                    <Show when=move || is_menu_available(&MenuItem::Faucet, current_network())>
                        <div 
                            class="menu-item"
                            class:active=move || current_menu.get() == MenuItem::Faucet
                            on:click=move |_| set_current_menu.set(MenuItem::Faucet)
                        >
                            <i class="fas fa-faucet"></i>
                            <span>"Faucet (testnet)"</span>
                        </div>
                    </Show>
                    
                    // Profile - only in testnet
                    <Show when=move || is_menu_available(&MenuItem::Profile, current_network())>
                        <div 
                            class="menu-item"
                            class:active=move || current_menu.get() == MenuItem::Profile
                            on:click=move |_| set_current_menu.set(MenuItem::Profile)
                        >
                            <i class="fas fa-user"></i>
                            <span>"Profile"</span>
                        </div>
                    </Show>

                    // Settings - available on all networks
                    <Show when=move || is_menu_available(&MenuItem::Settings, current_network())>
                        <div
                            class="menu-item"
                            class:active=move || current_menu.get() == MenuItem::Settings
                            on:click=move |_| set_current_menu.set(MenuItem::Settings)
                        >
                            <i class="fas fa-cog"></i>
                            <span>"Settings"</span>
                        </div>
                    </Show>
                    
                    // Network status indicator at bottom of sidebar
                    <div class="sidebar-network-status">
                        {move || {
                            match current_network() {
                                Some(NetworkType::Testnet) => view! {
                                    <div class="network-indicator network-testnet">
                                        <i class="fas fa-circle"></i>
                                        <span>"Testnet"</span>
                                    </div>
                                }.into_view(),
                                Some(NetworkType::ProdStaging) => view! {
                                    <div class="network-indicator network-staging">
                                        <i class="fas fa-circle"></i>
                                        <span>"Prod Staging"</span>
                                    </div>
                                }.into_view(),
                                Some(NetworkType::Mainnet) => view! {
                                    <div class="network-indicator network-mainnet">
                                        <i class="fas fa-circle"></i>
                                        <span>"Mainnet"</span>
                                    </div>
                                }.into_view(),
                                None => view! {
                                    <div class="network-indicator network-unknown">
                                        <i class="fas fa-circle"></i>
                                        <span>"Unknown"</span>
                                    </div>
                                }.into_view(),
                            }
                        }}
                    </div>
                </div>

                <div class="content">
                    // Mint - always visible
                    <div style=move || if current_menu.get() == MenuItem::Mint { "display: block;" } else { "display: none;" }>
                        <MintPage session=session/>
                    </div>
                    
                    // Project - only in testnet
                    <Show when=move || is_menu_available(&MenuItem::Project, current_network())>
                        <div style=move || if current_menu.get() == MenuItem::Project { "display: block;" } else { "display: none;" }>
                            <ProjectPage session=session/>
                        </div>
                    </Show>
                    
                    // Chat - available in testnet, staging, and mainnet
                    <Show when=move || is_menu_available(&MenuItem::Chat, current_network())>
                        <div style=move || if current_menu.get() == MenuItem::Chat { "display: block;" } else { "display: none;" }>
                            <ChatPage session=session/>
                        </div>
                    </Show>
                    
                    // Faucet - only in testnet
                    <Show when=move || is_menu_available(&MenuItem::Faucet, current_network())>
                        <div style=move || if current_menu.get() == MenuItem::Faucet { "display: block;" } else { "display: none;" }>
                            <FaucetPage session=session/>
                        </div>
                    </Show>
                    
                    // Profile - only in testnet
                    <Show when=move || is_menu_available(&MenuItem::Profile, current_network())>
                        <div style=move || if current_menu.get() == MenuItem::Profile { "display: block;" } else { "display: none;" }>
                            <ProfilePage session=session/>
                        </div>
                    </Show>

                    // Settings - available on all networks
                    <Show when=move || is_menu_available(&MenuItem::Settings, current_network())>
                        <div style=move || if current_menu.get() == MenuItem::Settings { "display: block;" } else { "display: none;" }>
                            <SettingsPage/>
                        </div>
                    </Show>
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
            
            // Transfer Dialog
            <Show when=move || show_transfer_dialog.get()>
                <div class="modal-overlay" on:click=move |_| {
                    if !transfer_loading.get() && !transfer_success.get() {
                        set_show_transfer_dialog.set(false);
                        set_transfer_address.set(String::new());
                        set_transfer_amount.set(String::new());
                        set_transfer_message.set(String::new());
                        set_transfer_success.set(false);
                        set_transfer_tx_hash.set(String::new());
                    }
                }>
                    <div class="modal-content transfer-dialog" on:click=|e| e.stop_propagation()>
                        <div class="modal-header">
                            <h3>
                                <i class="fas fa-paper-plane"></i>
                                "Transfer Tokens"
                            </h3>
                            <Show when=move || !transfer_loading.get()>
                                <button 
                                    class="modal-close"
                                    on:click=move |_| {
                                        set_show_transfer_dialog.set(false);
                                        set_transfer_address.set(String::new());
                                        set_transfer_amount.set(String::new());
                                        set_transfer_message.set(String::new());
                                        set_transfer_success.set(false);
                                        set_transfer_tx_hash.set(String::new());
                                    }
                                >
                                    "×"
                                </button>
                            </Show>
                        </div>
                        
                        <div class="modal-body">
                            <Show 
                                when=move || !transfer_loading.get() && !transfer_success.get() && transfer_message.get().is_empty()
                                fallback=move || {
                                    if transfer_success.get() {
                                        view! {
                                            <div class="transfer-success">
                                                <div class="success-icon">
                                                    <i class="fas fa-check"></i>
                                                </div>
                                                <p class="success-message">"Transfer Successful!"</p>
                                                <div class="tx-info">
                                                    <span class="tx-label">"Transaction Signature:"</span>
                                                    <div class="tx-hash">{transfer_tx_hash}</div>
                                                </div>
                                            </div>
                                        }.into_view()
                                    } else if transfer_loading.get() {
                                        view! {
                                            <div class="transfer-status">
                                                <div class="loading-spinner"></div>
                                                <p class="transfer-message">{transfer_message}</p>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div class="transfer-status">
                                                <p class="transfer-message">{transfer_message}</p>
                                            </div>
                                        }.into_view()
                                    }
                                }
                            >
                                <div class="transfer-form">
                                    <div class="form-group">
                                        <label>
                                            <i class="fas fa-coins"></i>
                                            "Token Type:"
                                        </label>
                                        <select 
                                            class="form-control"
                                            on:change=move |ev| {
                                                let value = event_target_value(&ev);
                                                set_transfer_type.set(value);
                                            }
                                        >
                                            <option value="MEMO" selected={move || transfer_type.get() == "MEMO"}>"MEMO"</option>
                                            <option value="XNT" selected={move || transfer_type.get() == "XNT"}>"XNT"</option>
                                        </select>
                                    </div>
                                    
                                    <div class="form-group">
                                        <label>
                                            <i class="fas fa-wallet"></i>
                                            "Recipient Address:"
                                        </label>
                                        <input 
                                            type="text"
                                            class="form-control"
                                            placeholder="Enter recipient address"
                                            prop:value=move || transfer_address.get()
                                            on:input=move |ev| {
                                                set_transfer_address.set(event_target_value(&ev));
                                            }
                                        />
                                    </div>
                                    
                                    <div class="form-group">
                                        <label>
                                            <i class="fas fa-money-bill-wave"></i>
                                            {move || format!("Amount ({}):", transfer_type.get())}
                                        </label>
                                        <input 
                                            type="text"
                                            inputmode="decimal"
                                            class="form-control"
                                            placeholder="Enter amount (e.g., 0.1)"
                                            prop:value=move || transfer_amount.get()
                                            on:input=move |ev| {
                                                let value = event_target_value(&ev);
                                                // Allow numbers, one decimal point, and leading zeros
                                                // Remove any invalid characters except digits and one decimal point
                                                let filtered: String = value.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
                                                
                                                // Ensure only one decimal point
                                                let parts: Vec<&str> = filtered.split('.').collect();
                                                let clean_value = if parts.len() <= 2 {
                                                    filtered
                                                } else {
                                                    // Keep only first decimal point
                                                    format!("{}.{}", parts[0], parts[1..].join(""))
                                                };
                                                
                                                set_transfer_amount.set(clean_value);
                                            }
                                        />
                                        <div class="balance-info">
                                            <i class="fas fa-info-circle"></i>
                                            {move || {
                                                if transfer_type.get() == "MEMO" {
                                                    format!("Available: {:.6} MEMO", token_balance())
                                                } else {
                                                    format!("Available: {:.6} XNT", sol_balance())
                                                }
                                            }}
                                        </div>
                                    </div>
                                </div>
                            </Show>
                        </div>
                        
                        <Show when=move || transfer_success.get()>
                            <div class="modal-footer">
                                <button 
                                    class="btn-primary"
                                    on:click=move |_| {
                                        set_show_transfer_dialog.set(false);
                                        set_transfer_address.set(String::new());
                                        set_transfer_amount.set(String::new());
                                        set_transfer_message.set(String::new());
                                        set_transfer_success.set(false);
                                        set_transfer_tx_hash.set(String::new());
                                    }
                                >
                                    <i class="fas fa-check"></i>
                                    "Close"
                                </button>
                            </div>
                        </Show>
                        
                        <Show when=move || !transfer_loading.get() && !transfer_success.get() && transfer_message.get().is_empty()>
                            <div class="modal-footer">
                                <button 
                                    class="btn-secondary"
                                    on:click=move |_| {
                                        set_show_transfer_dialog.set(false);
                                        set_transfer_address.set(String::new());
                                        set_transfer_amount.set(String::new());
                                        set_transfer_message.set(String::new());
                                    }
                                >
                                    <i class="fas fa-times"></i>
                                    "Cancel"
                                </button>
                                <button 
                                    class="btn-primary transfer-btn"
                                    on:click=move |_| {
                                        let address = transfer_address.get();
                                        let amount_str = transfer_amount.get();
                                        let token_type = transfer_type.get();
                                        
                                        // Validate inputs
                                        if address.is_empty() {
                                            set_transfer_message.set("Please enter a recipient address".to_string());
                                            set_timeout(move || {
                                                set_transfer_message.set(String::new());
                                            }, Duration::from_millis(3000));
                                            return;
                                        }
                                        
                                        if amount_str.is_empty() {
                                            set_transfer_message.set("Please enter an amount".to_string());
                                            set_timeout(move || {
                                                set_transfer_message.set(String::new());
                                            }, Duration::from_millis(3000));
                                            return;
                                        }
                                        
                                        let amount: f64 = match amount_str.parse() {
                                            Ok(a) => a,
                                            Err(_) => {
                                                set_transfer_message.set("Invalid amount".to_string());
                                                set_timeout(move || {
                                                    set_transfer_message.set(String::new());
                                                }, Duration::from_millis(3000));
                                                return;
                                            }
                                        };
                                        
                                        if amount <= 0.0 {
                                            set_transfer_message.set("Amount must be greater than 0".to_string());
                                            set_timeout(move || {
                                                set_transfer_message.set(String::new());
                                            }, Duration::from_millis(3000));
                                            return;
                                        }
                                        
                                        // Check balance
                                        let current_balance = if token_type == "MEMO" {
                                            token_balance()
                                        } else {
                                            sol_balance()
                                        };
                                        
                                        if amount > current_balance {
                                            set_transfer_message.set("Insufficient balance".to_string());
                                            set_timeout(move || {
                                                set_transfer_message.set(String::new());
                                            }, Duration::from_millis(3000));
                                            return;
                                        }
                                        
                                        // Show confirmation dialog
                                        set_confirm_transfer_data.set(Some((token_type.clone(), address.clone(), amount_str.clone())));
                                        set_show_confirm_dialog.set(true);
                                    }
                                >
                                    <i class="fas fa-paper-plane"></i>
                                    "Transfer"
                                </button>
                            </div>
                        </Show>
                    </div>
                </div>
            </Show>
            
            // Transfer Confirmation Dialog
            <Show when=move || show_confirm_dialog.get()>
                <div class="modal-overlay" on:click=move |_| set_show_confirm_dialog.set(false)>
                    <div class="modal-content confirm-dialog" on:click=|e| e.stop_propagation()>
                        <div class="modal-header">
                            <h3>
                                <i class="fas fa-exclamation-triangle"></i>
                                "Confirm Transfer"
                            </h3>
                            <button 
                                class="modal-close"
                                on:click=move |_| set_show_confirm_dialog.set(false)
                            >
                                "×"
                            </button>
                        </div>
                        
                        <div class="modal-body">
                            {move || {
                                if let Some((token_type, address, amount)) = confirm_transfer_data.get() {
                                    view! {
                                        <div class="confirm-details">
                                            <p>
                                                <i class="fas fa-coins"></i>
                                                <strong>"Token Type: "</strong>
                                                <span>{token_type.clone()}</span>
                                            </p>
                                            <p>
                                                <i class="fas fa-wallet"></i>
                                                <strong>"Recipient: "</strong>
                                                <span>{format!("{}...{}", &address[..8], &address[address.len()-8..])}</span>
                                            </p>
                                            <p>
                                                <i class="fas fa-money-bill-wave"></i>
                                                <strong>"Amount: "</strong>
                                                <span>{format!("{} {}", amount, token_type)}</span>
                                            </p>
                                            <p class="warning-text">
                                                <i class="fas fa-exclamation-circle"></i>
                                                "Please confirm this transfer. This action cannot be undone."
                                            </p>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! { <div></div> }.into_view()
                                }
                            }}
                        </div>
                        
                        <div class="modal-footer">
                            <button 
                                class="btn-secondary"
                                on:click=move |_| set_show_confirm_dialog.set(false)
                            >
                                <i class="fas fa-times"></i>
                                "Cancel"
                            </button>
                            <button 
                                class="btn-primary confirm-transfer-btn"
                                on:click=move |_| {
                                    set_show_confirm_dialog.set(false);
                                    
                                    if let Some((token_type, address, amount_str)) = confirm_transfer_data.get() {
                                        set_transfer_loading.set(true);
                                        set_transfer_message.set("Processing transfer...".to_string());
                                        
                                        let session_clone = session;
                                        spawn_local(async move {
                                            let amount: f64 = amount_str.parse().unwrap_or(0.0);
                                            
                                            let result = if token_type == "MEMO" {
                                                // Transfer MEMO tokens (in lamports with 6 decimals)
                                                let amount_lamports = (amount * 1_000_000.0) as u64;
                                                let mut session_update = session_clone.get_untracked();
                                                session_update.transfer_token(&address, amount_lamports).await
                                            } else {
                                                // Transfer XNT (in lamports with 9 decimals)
                                                let amount_lamports = (amount * 1_000_000_000.0) as u64;
                                                let mut session_update = session_clone.get_untracked();
                                                session_update.transfer_native(&address, amount_lamports).await
                                            };
                                            
                                            match result {
                                                Ok(tx_hash) => {
                                                    log::info!("Transfer successful: {}", tx_hash);
                                                    add_log_entry("INFO", &format!("Transfer successful: {}", tx_hash));
                                                    
                                                    // Update session to trigger balance refresh
                                                    session_clone.update(|s| {
                                                        s.mark_balance_update_needed();
                                                    });
                                                    
                                                    // Show success state with transaction hash (don't auto-close)
                                                    set_transfer_success.set(true);
                                                    set_transfer_tx_hash.set(tx_hash);
                                                    set_transfer_loading.set(false);
                                                    set_transfer_message.set(String::new());
                                                },
                                                Err(e) => {
                                                    log::error!("Transfer failed: {}", e);
                                                    add_log_entry("ERROR", &format!("Transfer failed: {}", e));
                                                    set_transfer_message.set(format!("Transfer failed: {}", e));
                                                    set_transfer_loading.set(false);
                                                }
                                            }
                                        });
                                    }
                                }
                            >
                                <i class="fas fa-check"></i>
                                "Confirm"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
} 
