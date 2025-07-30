use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_base::RpcConnection;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use rand::Rng;

// Mint mode enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MintMode {
    Manual,
    Auto,
}

// Supply tier configuration
#[derive(Debug, Clone)]
pub struct SupplyTier {
    pub min: u64,
    pub max: u64,
    pub reward: f64,
    pub label: String,
}

impl SupplyTier {
    fn get_tiers() -> Vec<SupplyTier> {
        vec![
            SupplyTier { min: 0, max: 100_000_000_000_000, reward: 1.0, label: "0-100M".to_string() },
            SupplyTier { min: 100_000_000_000_000, max: 1_000_000_000_000_000, reward: 0.1, label: "100M-1B".to_string() },
            SupplyTier { min: 1_000_000_000_000_000, max: 10_000_000_000_000_000, reward: 0.01, label: "1B-10B".to_string() },
            SupplyTier { min: 10_000_000_000_000_000, max: 100_000_000_000_000_000, reward: 0.001, label: "10B-100B".to_string() },
            SupplyTier { min: 100_000_000_000_000_000, max: 1_000_000_000_000_000_000, reward: 0.0001, label: "100B-1T".to_string() },
            SupplyTier { min: 1_000_000_000_000_000_000, max: u64::MAX, reward: 0.000001, label: "1T+".to_string() },
        ]
    }
    
    fn get_current_tier(supply: u64) -> SupplyTier {
        Self::get_tiers().into_iter()
            .find(|tier| supply >= tier.min && supply < tier.max)
            .unwrap_or_else(|| Self::get_tiers().last().unwrap().clone())
    }
    
    fn get_total_max_supply() -> u64 {
        1_000_000_000_000_000_000 // 1T as reasonable max for progress bar
    }
    
    fn calculate_progress_percentage(supply: u64) -> f64 {
        let max_supply = Self::get_total_max_supply();
        (supply as f64 / max_supply as f64 * 100.0).min(100.0)
    }
}

// Generate random JSON memo between 69-800 bytes
fn generate_random_memo() -> String {
    let mut rng = rand::thread_rng();
    
    // Create base JSON structure
    let base_json = serde_json::json!({
        "action": "mint",
        "timestamp": js_sys::Date::now() as u64,
        "user_id": format!("user_{}", rng.gen::<u32>()),
        "session_id": format!("session_{}", rng.gen::<u64>()),
        "platform": "memo-app",
        "version": "1.0.0"
    });
    
    let mut memo = serde_json::to_string(&base_json).unwrap();
    
    // Ensure minimum length of 69 bytes
    while memo.len() < 69 {
        memo = serde_json::to_string(&serde_json::json!({
            "action": "mint",
            "timestamp": js_sys::Date::now() as u64,
            "user_id": format!("user_{}", rng.gen::<u32>()),
            "session_id": format!("session_{}", rng.gen::<u64>()),
            "platform": "memo-app",
            "version": "1.0.0",
            "random_data": format!("random_string_{}", rng.gen::<u64>()),
            "extra_padding": " ".repeat(10 + rng.gen_range(0..20))
        })).unwrap();
    }
    
    // Ensure maximum length of 800 bytes
    if memo.len() > 800 {
        memo.truncate(797);
        memo.push_str("..."); // Keep it as valid ending
    }
    
    memo
}

#[component]
pub fn SupplyProgressBar() -> impl IntoView {
    let (supply_info, set_supply_info) = create_signal::<Option<(u64, SupplyTier)>>(None);
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal::<Option<String>>(None);

    // Fetch supply information on component mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            
            let rpc = RpcConnection::new();
            match rpc.get_token_supply().await {
                Ok(supply) => {
                    let tier = SupplyTier::get_current_tier(supply);
                    set_supply_info.set(Some((supply, tier)));
                    set_loading.set(false);
                },
                Err(e) => {
                    log::error!("Failed to fetch token supply: {}", e);
                    set_error.set(Some(format!("Failed to load supply data: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="supply-progress-container">
            <div class="supply-progress-header">
                <h3>
                    <i class="fas fa-chart-line"></i>
                    "Token Supply Progress"
                </h3>
                <p>"Current mining tier based on total supply"</p>
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
                    let progress = SupplyTier::calculate_progress_percentage(supply);
                    let tiers = SupplyTier::get_tiers();
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
                                        let marker_position = SupplyTier::calculate_progress_percentage(tier.max);
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
                                        {format!("{:.2}M tokens", supply_tokens / 1_000_000.0)}
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
                    
                    // Update session to trigger balance refresh
                    session.update(|s| {
                        s.mark_balance_update_needed();
                    });
                },
                Err(e) => {
                    log::error!("Mint failed: {}", e);
                    set_error_message.set(Some(format!("Mint failed: {}", e)));
                }
            }
        }
    });

    // Auto mint logic
    let auto_mint_loop = create_action(move |_: &()| {
        let target_count = auto_mint_count.get();
        async move {
            // UI state is already set, so we don't need to set it again here
            // set_auto_mint_running.set(true); // This is now done in the click handler
            set_auto_mint_current.set(0);
            
            let mut current_count = 0u32;
            let mut should_continue = true;
            
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
                
                // Call session mint_new_contract
                let result = session.with(|s| s.clone()).mint(&memo).await;
                
                match result {
                    Ok(signature) => {
                        log::info!("Auto mint #{} successful: {}", current_count + 1, signature);
                        set_last_result.set(Some(format!("#{}: {}", current_count + 1, signature)));
                        
                        // Update session to trigger balance refresh
                        session.update(|s| {
                            s.mark_balance_update_needed();
                        });
                        
                        current_count += 1;
                        set_auto_mint_current.set(current_count);
                        
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
                        log::error!("Auto mint #{} failed: {}", current_count + 1, e);
                        set_error_message.set(Some(format!("Auto mint #{} failed: {}", current_count + 1, e)));
                        should_continue = false;
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
                            <span class="mint-mode-description">"Click to mint once"</span>
                        </label>
                        
                        <label class="mint-mode-option">
                            <input 
                                type="radio" 
                                name="mint_mode"
                                checked=move || mint_mode.get() == MintMode::Auto
                                disabled=move || {
                                    let is_auto_running = auto_mint_running.get();
                                    let is_manual_pending = start_minting.pending().get() || is_submitting.get();
                                    is_auto_running || is_manual_pending
                                }
                                on:change=move |_| {
                                    set_mint_mode.set(MintMode::Auto);
                                }
                            />
                            <span class="mint-mode-label">
                                <i class="fas fa-robot"></i>
                                "Auto"
                            </span>
                            <span class="mint-mode-description">"Automatically mint multiple times"</span>
                        </label>
                    </div>
                    
                    // Auto mint count setting
                    {move || {
                        if mint_mode.get() == MintMode::Auto {
                            view! {
                                <div class="auto-mint-settings">
                                    <label class="auto-count-label">
                                        "Number of mints (0 = infinite):"
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
                                    </label>
                                    <div class="auto-mint-info">
                                        {move || {
                                            let count = auto_mint_count.get();
                                            if count == 0 {
                                                "Will mint continuously until stopped or insufficient balance".to_string()
                                            } else {
                                                format!("Will mint {} times automatically", count)
                                            }
                                        }}
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
                                            let current = auto_mint_current.get();
                                            let total = auto_mint_count.get();
                                            view! {
                                                <div class="auto-mint-progress">
                                                    {if total == 0 {
                                                        format!("Completed: {}", current)
                                                    } else {
                                                        format!("Progress: {}/{}", current, total)
                                                    }}
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! { <div></div> }.into_view()
                                        }
                                    }}
                                </div>
                            }.into_view()
                        }
                    }}
                    
                    <div class="mint-description">
                        {move || {
                            match mint_mode.get() {
                                MintMode::Manual => "This will generate a random JSON memo (69-800 bytes) and mint tokens once".to_string(),
                                MintMode::Auto => "This will automatically mint tokens multiple times with a 2-second delay between each mint".to_string(),
                            }
                        }}
                    </div>
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
        </div>
    }
}