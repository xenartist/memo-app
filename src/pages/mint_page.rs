use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_mint::{MintConfig, SupplyTier};
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use web_sys::window;

// Mint mode enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MintMode {
    Manual,
    Auto,
}

// Leaderboard tab enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeaderboardTab {
    Holders,
    Burners,
}

// Generate random JSON memo between 69-800 bytes
fn generate_random_memo() -> String {
    // Create simplified base JSON structure (no sensitive info)
    // This generates exactly 70 bytes, meeting the 69-800 byte requirement
    let memo_json = serde_json::json!({
        "action": "mint",
        "platform": "memo-app",
        "version": "1.0.0",
        "chain": "X1"
    });
    
    let memo = serde_json::to_string(&memo_json).unwrap();
    
    // Verify length (should be 70 bytes)
    debug_assert!(memo.len() >= 69 && memo.len() <= 800, 
        "Memo length {} is outside valid range 69-800", memo.len());
    
    memo
}

// Format supply display with proper units and thousand separators
fn format_supply_display(supply_tokens: f64) -> String {
    // add thousand separators helper function
    fn add_thousand_separators(num: f64, decimal_places: usize) -> String {
        let formatted = format!("{:.prec$}", num, prec = decimal_places);
        let parts: Vec<&str> = formatted.split('.').collect();
        let integer_part = parts[0];
        let decimal_part = if parts.len() > 1 { parts[1] } else { "" };
        
        // add thousand separators to integer part
        let mut result = String::new();
        let chars: Vec<char> = integer_part.chars().collect();
        for (i, ch) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                result.push(',');
            }
            result.push(*ch);
        }
        
        // if there is decimal part and not all zeros, add decimal part
        if !decimal_part.is_empty() && !decimal_part.chars().all(|c| c == '0') {
            result.push('.');
            result.push_str(decimal_part.trim_end_matches('0'));
        }
        
        result
    }
    
    if supply_tokens < 1_000_000.0 {
        // less than 1M, show full value
        format!("{} tokens", add_thousand_separators(supply_tokens, 0))
    } else if supply_tokens < 1_000_000_000.0 {
        // less than 1B, more than 1M, show value and unit M
        let millions = supply_tokens / 1_000_000.0;
        format!("{}M tokens", add_thousand_separators(millions, 2))
    } else if supply_tokens < 1_000_000_000_000.0 {
        // less than 1T, more than 1B, show value and unit B
        let billions = supply_tokens / 1_000_000_000.0;
        format!("{}B tokens", add_thousand_separators(billions, 2))
    } else {
        // more than 1T, show value and unit T
        let trillions = supply_tokens / 1_000_000_000_000.0;
        format!("{}T tokens", add_thousand_separators(trillions, 2))
    }
}

#[component]
pub fn SupplyProgressBar() -> impl IntoView {
    let (supply_info, set_supply_info) = create_signal::<Option<(u64, SupplyTier)>>(None);
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal::<Option<String>>(None);
    let (timer_active, set_timer_active) = create_signal(false);

    // fetch supply data
    let fetch_supply_data = move |is_initial_load: bool| {
        spawn_local(async move {
            if is_initial_load {
                set_loading.set(true);
            }
            set_error.set(None);
            
            let rpc = RpcConnection::new();
            match rpc.get_current_supply_tier_info().await {
                Ok((supply, tier)) => {
                    set_supply_info.set(Some((supply, tier)));
                    set_loading.set(false);
                    
                    // after first successful fetch, start timer
                    if !timer_active.get() {
                        set_timer_active.set(true);
                        log::info!("Starting supply data auto-refresh timer (10 seconds interval for testing)");
                        
                        // start background timer
                        spawn_local(async move {
                            loop {
                                // wait 3600 seconds (1 hour)
                                TimeoutFuture::new(3600_000).await;
                                
                                // check if timer should still run
                                if !timer_active.get() {
                                    break;
                                }
                                
                                log::info!("Auto-refreshing supply data...");
                                
                                // background update data (no loading state)
                                let rpc = RpcConnection::new();
                                match rpc.get_current_supply_tier_info().await {
                                    Ok((supply, tier)) => {
                                        set_supply_info.set(Some((supply, tier)));
                                        log::info!("Supply data auto-refreshed successfully");
                                    },
                                    Err(e) => {
                                        log::warn!("Failed to auto-refresh supply data: {}", e);
                                        // silent failure, do not update error state, keep current data
                                    }
                                }
                            }
                        });
                    }
                },
                Err(e) => {
                    log::error!("Failed to fetch token supply: {}", e);
                    set_error.set(Some(format!("Failed to load supply data: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // fetch data on first load
    create_effect(move |_| {
        fetch_supply_data(true); // first load, show loading state
    });

    // stop timer on component unmount
    on_cleanup(move || {
        set_timer_active.set(false);
        log::info!("Stopped supply data auto-refresh timer");
    });

    // Manual refresh handler
    let handle_refresh = move |_| {
        log::info!("Manual refresh triggered");
        fetch_supply_data(false); // false = don't show loading spinner
    };

    view! {
        <div class="supply-progress-container">
            <div class="supply-progress-header">
                <div class="supply-progress-title">
                    <h3>
                        <i class="fas fa-chart-line"></i>
                        "Token Supply Progress"
                    </h3>
                    <p>"Current mining tier based on total supply"</p>
                </div>
                <button 
                    class="supply-refresh-btn"
                    on:click=handle_refresh
                    disabled=move || loading.get()
                    title="Refresh supply data"
                >
                    <i class="fas fa-sync-alt" class:fa-spin=move || loading.get()></i>
                    <span>"Refresh"</span>
                </button>
            </div>

            {move || {
                if loading.get() {
                    view! {
                        <div class="supply-loading">
                            <i class="fas fa-spinner fa-spin"></i>
                            " Loading supply data..."
                        </div>
                    }.into_view()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="supply-error">
                            <i class="fas fa-exclamation-triangle"></i>
                            " " {err}
                        </div>
                    }.into_view()
                } else if let Some((supply, tier)) = supply_info.get() {
                    let progress = MintConfig::calculate_visual_progress_percentage(supply);
                    let tiers = MintConfig::get_supply_tiers();
                    let supply_tokens = supply as f64 / 1_000_000.0; // Convert to tokens (6 decimals)
                    
                    view! {
                        <div>
                            <div class="supply-progress-track">
                                <div 
                                    class="supply-progress-fill"
                                    style:width=format!("{}%", progress)
                                ></div>
                                <div class="supply-progress-markers">
                                    {tiers.iter().take(5).enumerate().map(|(i, tier)| {
                                        let marker_position = MintConfig::calculate_visual_marker_position(tier.max);
                                        view! {
                                            <div 
                                                class="supply-progress-marker"
                                                style:left=format!("{}%", marker_position)
                                            ></div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                            
                            <div class="supply-progress-labels">
                                {tiers.iter().map(|tier| {
                                    view! {
                                        <div class="supply-progress-label">
                                            {tier.label.clone()}
                                            <br/>
                                            {format!("{} token", tier.reward)}
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>

                            <div class="supply-current-info">
                                <div class="supply-info-item">
                                    <div class="supply-info-label">"Current Supply"</div>
                                    <div class="supply-info-value">
                                        {format_supply_display(supply_tokens)}
                                    </div>
                                </div>
                                <div class="supply-info-item">
                                    <div class="supply-info-label">"Current Reward"</div>
                                    <div class="supply-info-value">
                                        {format!("{} token", tier.reward)}
                                    </div>
                                </div>
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}
        </div>
    }
}

#[component]
pub fn SwapBridgeLink() -> impl IntoView {
    let handle_click = move |_| {
        if let Some(window) = window() {
            let bridge_url = "https://app-dev.bridge.x1.xyz/";
            let _ = window.open_with_url_and_target(bridge_url, "_blank");
        }
    };

    view! {
        <div class="swap-bridge-container">
            <button 
                class="swap-bridge-card"
                on:click=handle_click
            >
                <div class="swap-bridge-icon">
                    <i class="fas fa-exchange-alt"></i>
                </div>
                <div class="swap-bridge-content">
                    <h4 class="swap-bridge-title">"Official X1 Swap Bridge"</h4>
                    <p class="swap-bridge-description">"Atomic cross-chain swap of USDC → XNT"</p>
                </div>
                <div class="swap-bridge-arrow">
                    <i class="fas fa-external-link-alt"></i>
                </div>
            </button>
        </div>
    }
}

#[component]
pub fn TokenHoldersLeaderboard() -> impl IntoView {
    let (holders, set_holders) = create_signal::<Vec<(String, f64)>>(Vec::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal::<Option<String>>(None);
    let (is_loaded, set_is_loaded) = create_signal(false);
    
    const MAX_DISPLAY: usize = 100;
    
    // Fetch holders data (only once when component mounts)
    let fetch_holders = move || {
        // Only fetch if not already loaded
        if is_loaded.get() {
            return;
        }
        
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            let rpc = RpcConnection::new();
            
            match rpc.get_token_holders(MAX_DISPLAY).await {
                Ok(all_holders) => {
                    set_holders.set(all_holders);
                    set_loading.set(false);
                    set_is_loaded.set(true);
                    log::info!("Token holders leaderboard loaded successfully");
                },
                Err(e) => {
                    log::error!("Failed to fetch token holders: {}", e);
                    set_error.set(Some(format!("Failed to load leaderboard: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Fetch data on component mount
    create_effect(move |_| {
        fetch_holders();
    });
    
    // Format number with thousand separators
    let format_number = |num: f64| -> String {
        let int_part = num as u64;
        let dec_part = ((num - int_part as f64) * 100.0) as u64;
        
        let mut result = String::new();
        let int_str = int_part.to_string();
        let chars: Vec<char> = int_str.chars().collect();
        
        for (i, ch) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                result.push(',');
            }
            result.push(*ch);
        }
        
        if dec_part > 0 {
            result.push_str(&format!(".{:02}", dec_part));
        }
        
        result
    };
    
    // Shorten address (first 4 and last 4 characters)
    let shorten_address = |addr: &str| -> String {
        if addr.len() > 12 {
            format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
        } else {
            addr.to_string()
        }
    };

    view! {
        <div class="token-holders-leaderboard">
            <div class="leaderboard-header">
                <h3>
                    <i class="fas fa-trophy"></i>
                    "MEMO Token Holders - Top 100"
                </h3>
                <p>"Ranked by token balance"</p>
            </div>
            
            {move || {
                if loading.get() {
                    view! {
                        <div class="leaderboard-loading">
                            <i class="fas fa-spinner fa-spin"></i>
                            " Loading leaderboard..."
                        </div>
                    }.into_view()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="leaderboard-error">
                            <i class="fas fa-exclamation-triangle"></i>
                            " " {err}
                        </div>
                    }.into_view()
                } else if holders.get().is_empty() {
                    view! {
                        <div class="leaderboard-empty">
                            <i class="fas fa-inbox"></i>
                            " No token holders found"
                        </div>
                    }.into_view()
                } else {
                    let all_holders = holders.get();
                    
                    view! {
                        <div class="leaderboard-table">
                            <div class="leaderboard-table-header">
                                <div class="rank-col">"Rank"</div>
                                <div class="address-col">"Address"</div>
                                <div class="balance-col">"Balance"</div>
                            </div>
                            <div class="leaderboard-table-body">
                                {all_holders.iter().enumerate().map(|(idx, (addr, balance))| {
                                    let rank = idx + 1;
                                    let medal = if rank == 1 {
                                        "🥇"
                                    } else if rank == 2 {
                                        "🥈"
                                    } else if rank == 3 {
                                        "🥉"
                                    } else {
                                        ""
                                    };
                                    
                                    view! {
                                        <div class="leaderboard-row" class:top-three=rank <= 3>
                                            <div class="rank-col">
                                                {if !medal.is_empty() {
                                                    format!("{} #{}", medal, rank)
                                                } else {
                                                    format!("#{}", rank)
                                                }}
                                            </div>
                                            <div class="address-col" title=addr.clone()>
                                                {shorten_address(addr)}
                                            </div>
                                            <div class="balance-col">
                                                {format_number(*balance)}
                                                <span class="token-symbol">" MEMO"</span>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

#[component]
pub fn BurnerLeaderboard() -> impl IntoView {
    let (burners, set_burners) = create_signal::<Vec<(String, f64, u64)>>(Vec::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal::<Option<String>>(None);
    let (is_loaded, set_is_loaded) = create_signal(false);
    
    const MAX_DISPLAY: usize = 100;
    
    // Fetch burners data (only once when component mounts)
    let fetch_burners = move || {
        // Only fetch if not already loaded
        if is_loaded.get() {
            return;
        }
        
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            let rpc = RpcConnection::new();
            
            match rpc.get_top_burners(MAX_DISPLAY).await {
                Ok(all_burners) => {
                    set_burners.set(all_burners);
                    set_loading.set(false);
                    set_is_loaded.set(true);
                    log::info!("Token burners leaderboard loaded successfully");
                },
                Err(e) => {
                    log::error!("Failed to fetch token burners: {}", e);
                    set_error.set(Some(format!("Failed to load burner leaderboard: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    };

    // Fetch data on component mount
    create_effect(move |_| {
        fetch_burners();
    });
    
    // Format number with thousand separators
    let format_number = |num: f64| -> String {
        let int_part = num as u64;
        let dec_part = ((num - int_part as f64) * 100.0) as u64;
        
        let mut result = String::new();
        let int_str = int_part.to_string();
        let chars: Vec<char> = int_str.chars().collect();
        
        for (i, ch) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                result.push(',');
            }
            result.push(*ch);
        }
        
        if dec_part > 0 {
            result.push_str(&format!(".{:02}", dec_part));
        }
        
        result
    };
    
    // Shorten address (first 6 and last 4 characters)
    let shorten_address = |addr: &str| -> String {
        if addr.len() > 12 {
            format!("{}...{}", &addr[..6], &addr[addr.len()-4..])
        } else {
            addr.to_string()
        }
    };

    view! {
        <div class="token-burners-leaderboard">
            <div class="leaderboard-header">
                <h3>
                    <i class="fas fa-fire"></i>
                    "MEMO Token Burners - Top 100"
                </h3>
                <p>"Ranked by total burned tokens"</p>
            </div>
            
            {move || {
                if loading.get() {
                    view! {
                        <div class="leaderboard-loading">
                            <i class="fas fa-spinner fa-spin"></i>
                            " Loading leaderboard..."
                        </div>
                    }.into_view()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="leaderboard-error">
                            <i class="fas fa-exclamation-triangle"></i>
                            " " {err}
                        </div>
                    }.into_view()
                } else if burners.get().is_empty() {
                    view! {
                        <div class="leaderboard-empty">
                            <i class="fas fa-inbox"></i>
                            " No burners found"
                        </div>
                    }.into_view()
                } else {
                    let all_burners = burners.get();
                    
                    view! {
                        <div class="leaderboard-table">
                            <div class="leaderboard-table-header">
                                <div class="rank-col">"Rank"</div>
                                <div class="address-col">"Address"</div>
                                <div class="burned-col">"Total Burned"</div>
                                <div class="count-col">"Burn Count"</div>
                            </div>
                            <div class="leaderboard-table-body">
                                {all_burners.iter().enumerate().map(|(idx, (addr, total_burned, burn_count))| {
                                    let rank = idx + 1;
                                    let medal = if rank == 1 {
                                        "🥇"
                                    } else if rank == 2 {
                                        "🥈"
                                    } else if rank == 3 {
                                        "🥉"
                                    } else {
                                        ""
                                    };
                                    
                                    view! {
                                        <div class="leaderboard-row" class:top-three=rank <= 3>
                                            <div class="rank-col">
                                                {if !medal.is_empty() {
                                                    format!("{} #{}", medal, rank)
                                                } else {
                                                    format!("#{}", rank)
                                                }}
                                            </div>
                                            <div class="address-col" title=addr.clone()>
                                                {shorten_address(addr)}
                                            </div>
                                            <div class="burned-col">
                                                {format_number(*total_burned)}
                                                <span class="token-symbol">" MEMO"</span>
                                            </div>
                                            <div class="count-col">
                                                {format!("{}", burn_count)}
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

#[component]
pub fn MintPage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (last_result, set_last_result) = create_signal::<Option<String>>(None);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);
    let (minting_status, set_minting_status) = create_signal(String::new());
    
    // Mint mode and settings
    let (mint_mode, set_mint_mode) = create_signal(MintMode::Manual);
    let (auto_mint_count, set_auto_mint_count) = create_signal(0u32); // 0 = infinite
    let (auto_mint_current, set_auto_mint_current) = create_signal(0u32);
    let (auto_mint_running, set_auto_mint_running) = create_signal(false);
    
    // --- Manual signal to control immediate UI state on submit ---
    let (is_submitting, set_is_submitting) = create_signal(false);

    // Helper function to check XNT balance before minting
    let check_balance_before_mint = move || -> Result<(), String> {
        let current_session = session.get();
        let sol_balance = current_session.get_sol_balance();
        
        if sol_balance <= 0.0 {
            let wallet_address = current_session.get_public_key()
                .unwrap_or_else(|_| "Unknown".to_string());
            
            Err(format!(
                "Insufficient XNT balance. Your current balance is {} XNT. Please deposit XNT to your wallet address: {}",
                sol_balance,
                wallet_address
            ))
        } else {
            Ok(())
        }
    };

    let start_minting = create_action(move |_: &()| {
        async move {
            // Generate random memo
            let memo = generate_random_memo();
            log::info!("Generated memo with length: {} bytes", memo.len());
            log::info!("Memo content: {}", memo);
            
            // Call session mint_new_contract
            let result = session.with(|s| s.clone()).mint(&memo).await;
            
            match result {
                Ok(signature) => {
                    log::info!("Mint successful: {}", signature);
                    set_last_result.set(Some(signature));
                    
                    // Wait for blockchain confirmation before updating balance
                    let session_clone = session;
                    spawn_local(async move {
                        log::info!("Waiting 20 seconds for transaction confirmation...");
                        TimeoutFuture::new(20000).await; // Wait 20 seconds
                        
                        log::info!("Triggering balance refresh after mint");
                        session_clone.update(|s| {
                            s.mark_balance_update_needed();
                        });
                    });
                },
                Err(e) => {
                    log::error!("Mint failed: {}", e);
                    set_error_message.set(Some(format!("Mint failed: {}", e)));
                }
            }
        }
    });

    // Auto mint loop logic
    let auto_mint_loop = create_action(move |_: &()| {
        let target_count = auto_mint_count.get();
        async move {
            // UI state is already set, so we don't need to set it again here
            // set_auto_mint_running.set(true); // This is now done in the click handler
            set_auto_mint_current.set(0);
            
            let mut current_count = 0u32;
            let mut should_continue = true;
            const MAX_RETRIES: u32 = 3; // Maximum retry attempts
            
            // Start balance refresh timer (every 20 seconds)
            let session_for_timer = session;
            let auto_mint_running_for_timer = auto_mint_running;
            spawn_local(async move {
                log::info!("Starting balance refresh timer for auto mint (every 20 seconds)");
                loop {
                    // Wait 20 seconds
                    TimeoutFuture::new(20000).await;
                    
                    // Check if auto mint is still running
                    if auto_mint_running_for_timer.get() {
                        log::info!("Auto mint balance refresh timer triggered");
                        session_for_timer.update(|s| {
                            s.mark_balance_update_needed();
                        });
                    } else {
                        // Auto mint stopped, do one final refresh and exit
                        log::info!("Auto mint stopped, performing final balance refresh");
                        session_for_timer.update(|s| {
                            s.mark_balance_update_needed();
                        });
                        break;
                    }
                }
                log::info!("Balance refresh timer for auto mint stopped");
            });
            
            while should_continue {
                // Check if we should stop
                if !auto_mint_running.get() {
                    break;
                }
                
                // Update status
                if target_count == 0 {
                    set_minting_status.set(format!("Auto minting... (#{} - infinite)", current_count + 1));
                } else {
                    set_minting_status.set(format!("Auto minting... ({}/{})", current_count + 1, target_count));
                }
                
                // Generate random memo
                let memo = generate_random_memo();
                log::info!("Auto mint #{}: Generated memo with length: {} bytes", current_count + 1, memo.len());
                
                // Retry logic for minting
                let mut retry_count = 0;
                let mut mint_successful = false;
                
                while retry_count <= MAX_RETRIES && !mint_successful {
                    // Check if we should stop during retries
                    if !auto_mint_running.get() {
                        should_continue = false;
                        break;
                    }
                    
                    // Update status for retry attempts
                    if retry_count > 0 {
                        if target_count == 0 {
                            set_minting_status.set(format!("Auto minting... (#{} - infinite) - Retry {}/{}", 
                                current_count + 1, retry_count, MAX_RETRIES));
                        } else {
                            set_minting_status.set(format!("Auto minting... ({}/{}) - Retry {}/{}", 
                                current_count + 1, target_count, retry_count, MAX_RETRIES));
                        }
                    }
                    
                    // Call session mint_new_contract
                    let result = session.with(|s| s.clone()).mint(&memo).await;
                    
                    match result {
                        Ok(signature) => {
                            log::info!("Auto mint #{} successful: {}", current_count + 1, signature);
                            set_last_result.set(Some(format!("#{}: {}", current_count + 1, signature)));
                            
                            current_count += 1;
                            set_auto_mint_current.set(current_count);
                            mint_successful = true;
                            
                            // Check if we've reached the target count (if not infinite)
                            if target_count > 0 && current_count >= target_count {
                                should_continue = false;
                            }
                            
                            // Add delay between mints (2 seconds)
                            if should_continue {
                                TimeoutFuture::new(2000).await;
                            }
                        },
                        Err(e) => {
                            retry_count += 1;
                            log::warn!("Auto mint #{} attempt {} failed: {}", current_count + 1, retry_count, e);
                            
                            if retry_count > MAX_RETRIES {
                                // All retries exhausted
                                log::error!("Auto mint #{} failed after {} attempts: {}", current_count + 1, MAX_RETRIES, e);
                                set_error_message.set(Some(format!("Auto mint #{} failed after {} retry attempts: {}", 
                                    current_count + 1, MAX_RETRIES, e)));
                                should_continue = false;
                            } else {
                                // Wait before retry with exponential backoff (1s, 2s, 4s)
                                let retry_delay = 1000 * (1 << (retry_count - 1)); // 1000ms, 2000ms, 4000ms
                                log::info!("Retrying auto mint #{} in {}ms (attempt {}/{})", 
                                    current_count + 1, retry_delay, retry_count + 1, MAX_RETRIES + 1);
                                TimeoutFuture::new(retry_delay).await;
                            }
                        }
                    }
                }
            }
            
            set_auto_mint_running.set(false);
            set_minting_status.set(String::new());
            log::info!("Auto minting completed. Total mints: {}", current_count);
        }
    });

    // --- Effect for action result handling ---
    create_effect(move |_| {
        if let Some(result) = start_minting.value().get() {
            set_is_submitting.set(false); // reset manual state
            if mint_mode.get() == MintMode::Manual {
                set_minting_status.set(String::new()); // clear status for manual mode
            }
        }
    });

    view! {
        <div class="mint-page">
            // Remove the mint-page-header section entirely
            
            // Add the Swap Bridge link
            <SwapBridgeLink />
            
            // Add the supply progress bar here
            <SupplyProgressBar />
            
            <div class="mint-content">
                // Mint mode selection
                <div class="mint-mode-section">
                    <h3>
                        <i class="fas fa-cog"></i>
                        "Mint Mode"
                    </h3>
                    
                    <div class="mint-mode-options">
                        <label class="mint-mode-option">
                            <div class="mint-mode-radio-line">
                                <input 
                                    type="radio" 
                                    name="mint_mode"
                                    checked=move || mint_mode.get() == MintMode::Manual
                                    disabled=move || {
                                        let is_auto_running = auto_mint_running.get();
                                        let is_manual_pending = start_minting.pending().get() || is_submitting.get();
                                        is_auto_running || is_manual_pending
                                    }
                                    on:change=move |_| {
                                        set_mint_mode.set(MintMode::Manual);
                                        // Stop auto mining if running
                                        set_auto_mint_running.set(false);
                                    }
                                />
                                <span class="mint-mode-label">
                                    <i class="fas fa-hand-pointer"></i>
                                    "Manual"
                                </span>
                                <span class="mint-mode-description">"(Click to mint once)"</span>
                            </div>
                        </label>
                        
                        <label class="mint-mode-option">
                            <div class="mint-mode-radio-line">
                                <input 
                                    type="radio" 
                                    name="mint_mode"
                                    checked=move || mint_mode.get() == MintMode::Auto
                                    disabled=move || {
                                        use crate::core::session::WalletType;
                                        let current_session = session.get();
                                        let is_backpack = *current_session.get_wallet_type() == WalletType::Backpack;
                                        let is_auto_running = auto_mint_running.get();
                                        let is_manual_pending = start_minting.pending().get() || is_submitting.get();
                                        is_backpack || is_auto_running || is_manual_pending
                                    }
                                    on:change=move |_| {
                                        set_mint_mode.set(MintMode::Auto);
                                    }
                                />
                                <span class="mint-mode-label">
                                    <i class="fas fa-robot"></i>
                                    "Auto"
                                </span>
                                <span 
                                    class="mint-mode-description"
                                    class:backpack-not-supported=move || {
                                        use crate::core::session::WalletType;
                                        let current_session = session.get();
                                        *current_session.get_wallet_type() == WalletType::Backpack
                                    }
                                >
                                    {move || {
                                        use crate::core::session::WalletType;
                                        let current_session = session.get();
                                        if *current_session.get_wallet_type() == WalletType::Backpack {
                                            "(Not supported for Backpack wallet)"
                                        } else {
                                            "(Automatically mint multiple times)"
                                        }
                                    }}
                                </span>
                            </div>
                        </label>
                    </div>
                    
                    // Auto mint count setting
                    {move || {
                        if mint_mode.get() == MintMode::Auto {
                            view! {
                                <div class="auto-mint-settings">
                                    <div class="auto-count-line">
                                        <input 
                                            type="number"
                                            min="0"
                                            max="1000"
                                            class="auto-count-input"
                                            prop:value=move || auto_mint_count.get().to_string()
                                            disabled=move || {
                                                let is_auto_running = auto_mint_running.get();
                                                let is_manual_pending = start_minting.pending().get() || is_submitting.get();
                                                is_auto_running || is_manual_pending
                                            }
                                            on:input=move |ev| {
                                                let value = event_target_value(&ev);
                                                if let Ok(count) = value.parse::<u32>() {
                                                    set_auto_mint_count.set(count);
                                                }
                                            }
                                        />
                                        <span class="auto-count-label">"Number of mints (0 = infinite):"</span>
                                        <span class="auto-mint-info">
                                            {move || {
                                                let count = auto_mint_count.get();
                                                if count == 0 {
                                                    "Will mint continuously until stopped or insufficient balance".to_string()
                                                } else {
                                                    format!("Will mint {} times automatically", count)
                                                }
                                            }}
                                        </span>
                                    </div>
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}
                </div>
                
                <div class="mint-controls">
                    {move || {
                        let mode = mint_mode.get();
                        let is_auto_running = auto_mint_running.get();
                        let is_manual_pending = start_minting.pending().get() || is_submitting.get();
                        
                        if mode == MintMode::Manual {
                            view! {
                                <button 
                                    class="mint-button"
                                    disabled=move || is_manual_pending || is_auto_running
                                    on:click=move |_| {
                                        // Check balance before minting
                                        if let Err(error_msg) = check_balance_before_mint() {
                                            set_error_message.set(Some(error_msg));
                                            return;
                                        }
                                        
                                        // Clear any previous errors
                                        set_error_message.set(None);
                                        
                                        // 1. immediately update UI state (sync)
                                        set_is_submitting.set(true);
                                        set_minting_status.set("Minting in progress...".to_string());
                                        
                                        // 2. async delay execution (give UI time to update)
                                        spawn_local(async move {
                                            TimeoutFuture::new(100).await; // 100ms delay
                                            start_minting.dispatch(());
                                        });
                                    }
                                >
                                    {move || {
                                        if is_manual_pending {
                                            view! {
                                                <>
                                                    <i class="fas fa-spinner fa-spin"></i>
                                                    "Minting..."
                                                </>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <>
                                                    <i class="fas fa-rocket"></i>
                                                    "Start Minting"
                                                </>
                                            }.into_view()
                                        }
                                    }}
                                </button>
                            }.into_view()
                        } else {
                            // Auto mode
                            view! {
                                <div class="auto-mint-controls">
                                    <button 
                                        class="mint-button"
                                        class:mint-button-stop=is_auto_running
                                        disabled=is_manual_pending
                                        on:click=move |_| {
                                            if is_auto_running {
                                                // Stop auto minting
                                                set_auto_mint_running.set(false);
                                                set_minting_status.set(String::new());
                                            } else {
                                                // Check balance before starting auto minting
                                                if let Err(error_msg) = check_balance_before_mint() {
                                                    set_error_message.set(Some(error_msg));
                                                    return;
                                                }
                                                
                                                // Clear any previous errors
                                                set_error_message.set(None);
                                                
                                                // Start auto minting - apply same UI responsiveness pattern as manual mode
                                                // 1. immediately update UI state (sync)
                                                set_auto_mint_running.set(true);
                                                set_minting_status.set("Starting auto minting...".to_string());
                                                
                                                // 2. async delay execution (give UI time to update)
                                                spawn_local(async move {
                                                    TimeoutFuture::new(100).await; // 100ms delay
                                                    auto_mint_loop.dispatch(());
                                                });
                                            }
                                        }
                                    >
                                        {move || {
                                            if is_auto_running {
                                                view! {
                                                    <>
                                                        <i class="fas fa-stop"></i>
                                                        "Stop Auto Minting"
                                                    </>
                                                }.into_view()
                                            } else {
                                                view! {
                                                    <>
                                                        <i class="fas fa-play"></i>
                                                        "Start Auto Minting"
                                                    </>
                                                }.into_view()
                                            }
                                        }}
                                    </button>
                                    
                                    {move || {
                                        if is_auto_running {
                                            view! { <div></div> }.into_view()
                                        } else {
                                            view! { <div></div> }.into_view()
                                        }
                                    }}
                                </div>
                            }.into_view()
                        }
                    }}
                    
                </div>
                
                // Show minting status
                {move || {
                    let status = minting_status.get();
                    if !status.is_empty() {
                        view! {
                            <div class="minting-progress">
                                <i class="fas fa-spinner fa-spin"></i>
                                <span>{status}</span>
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}
                
                // Show results
                <div class="mint-results">
                    {move || {
                        if let Some(error) = error_message.get() {
                            view! {
                                <div class="error-message">
                                    <strong>
                                        <i class="fas fa-exclamation-triangle"></i>
                                        "Error:"
                                    </strong>
                                    <div>{error}</div>
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}
                    
                    {move || {
                        if let Some(signature) = last_result.get() {
                            view! {
                                <div class="success-message">
                                    <strong>
                                        <i class="fas fa-check-circle"></i>
                                        "Mint Successful!"
                                    </strong>
                                    <div class="transaction-id">
                                        "Transaction: " {signature}
                                    </div>
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}
                </div>
            </div>
            
            // Leaderboard with tabs
            <LeaderboardWithTabs />
        </div>
    }
}

#[component]
pub fn LeaderboardWithTabs() -> impl IntoView {
    let (active_tab, set_active_tab) = create_signal(LeaderboardTab::Holders);

    view! {
        <div class="leaderboard-section">
            // Tab buttons
            <div class="leaderboard-tabs">
                <button
                    class="tab-button"
                    class:active=move || active_tab.get() == LeaderboardTab::Holders
                    on:click=move |_| set_active_tab.set(LeaderboardTab::Holders)
                >
                    <i class="fas fa-wallet"></i>
                    " Holders"
                </button>
                <button
                    class="tab-button"
                    class:active=move || active_tab.get() == LeaderboardTab::Burners
                    on:click=move |_| set_active_tab.set(LeaderboardTab::Burners)
                >
                    <i class="fas fa-fire"></i>
                    " Burners"
                </button>
            </div>
            
            // Tab content
            <div class="leaderboard-tab-content">
                {move || {
                    match active_tab.get() {
                        LeaderboardTab::Holders => view! { <TokenHoldersLeaderboard /> }.into_view(),
                        LeaderboardTab::Burners => view! { <BurnerLeaderboard /> }.into_view(),
                    }
                }}
            </div>
        </div>
    }
}