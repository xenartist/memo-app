use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use web_sys::window;
use crate::core::session::Session;
use crate::pages::log_view::add_log_entry;
use gloo_timers::future::TimeoutFuture;

// API request/response structures (matching backend)
#[derive(Clone, Debug, Serialize, Deserialize)]
struct MathChallenge {
    question: String,
    session_id: String,
    expires_at: String,
}

#[derive(Serialize)]
struct AirdropRequest {
    public_key: String,
    math_session_id: String,
    math_answer: i32,
}

#[derive(Deserialize)]
struct AirdropResponse {
    signature: String,
    message: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

// Faucet API client
struct FaucetApi {
    base_url: String,
}

impl FaucetApi {
    fn new() -> Self {
        Self {
            base_url: "https://faucet.x1.wiki".to_string(), // use actual faucet API URL
        }
    }

    async fn get_challenge(&self) -> Result<MathChallenge, String> {
        let url = format!("{}/challenge", self.base_url);
        
        let response = gloo_net::http::Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if response.ok() {
            response
                .json::<MathChallenge>()
                .await
                .map_err(|e| format!("Failed to parse challenge: {}", e))
        } else {
            Err(format!("Server error: {}", response.status()))
        }
    }

    async fn request_airdrop(&self, request: AirdropRequest) -> Result<AirdropResponse, String> {
        let url = format!("{}/airdrop", self.base_url);
        
        let response = gloo_net::http::Request::post(&url)
            .json(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if response.ok() {
            response
                .json::<AirdropResponse>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))
        } else {
            let error_response = response
                .json::<ErrorResponse>()
                .await
                .map_err(|_| format!("Server error: {}", response.status()))?;
            Err(error_response.error)
        }
    }

    async fn check_health(&self) -> Result<(), String> {
        let url = format!("{}/health", self.base_url);
        
        let response = gloo_net::http::Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if response.ok() {
            Ok(())
        } else {
            Err(format!("Health check failed: {}", response.status()))
        }
    }
}

#[component]
pub fn FaucetPage(session: RwSignal<Session>) -> impl IntoView {
    // State management
    let (current_challenge, set_current_challenge) = create_signal::<Option<MathChallenge>>(None);
    let (math_answer, set_math_answer) = create_signal(String::new());
    let (loading, set_loading) = create_signal(false);
    let (requesting, set_requesting) = create_signal(false);
    let (message, set_message) = create_signal::<Option<(String, String)>>(None); // (message, type)
    let (faucet_status, set_faucet_status) = create_signal("checking".to_string());

    // Load math challenge on component mount
    create_effect(move |_| {
        spawn_local(async move {
            set_loading.set(true);
            let api = FaucetApi::new();
            
            // Check faucet health first
            match api.check_health().await {
                Ok(_) => {
                    set_faucet_status.set("online".to_string());
                    add_log_entry("INFO", "Faucet service is online");
                },
                Err(e) => {
                    set_faucet_status.set("offline".to_string());
                    add_log_entry("ERROR", &format!("Faucet service is offline: {}", e));
                }
            }
            
            // Load initial challenge
            match api.get_challenge().await {
                Ok(challenge) => {
                    set_current_challenge.set(Some(challenge));
                    add_log_entry("INFO", "Math challenge loaded");
                },
                Err(e) => {
                    add_log_entry("ERROR", &format!("Failed to load math challenge: {}", e));
                    set_message.set(Some((format!("Failed to load challenge: {}", e), "error".to_string())));
                }
            }
            
            set_loading.set(false);
        });
    });

    // Refresh challenge function
    let refresh_challenge = move |_| {
        spawn_local(async move {
            set_loading.set(true);
            let api = FaucetApi::new();
            
            match api.get_challenge().await {
                Ok(challenge) => {
                    set_current_challenge.set(Some(challenge));
                    set_math_answer.set(String::new());
                    set_message.set(None);
                    add_log_entry("INFO", "New math challenge loaded");
                },
                Err(e) => {
                    add_log_entry("ERROR", &format!("Failed to refresh challenge: {}", e));
                    set_message.set(Some((format!("Failed to refresh challenge: {}", e), "error".to_string())));
                }
            }
            
            set_loading.set(false);
        });
    };

    // Request airdrop function
    let request_airdrop = move |_| {
        // Get current wallet public key
        let public_key = session.with_untracked(|s| s.get_public_key());
        let current_challenge_data = current_challenge.get();
        let answer_str = math_answer.get();

        // Validation
        let pubkey = match public_key {
            Ok(key) => key,
            Err(_) => {
                set_message.set(Some(("Please connect your wallet first".to_string(), "error".to_string())));
                return;
            }
        };

        let challenge = match current_challenge_data {
            Some(ch) => ch,
            None => {
                set_message.set(Some(("Please wait for challenge to load".to_string(), "error".to_string())));
                return;
            }
        };

        let answer = match answer_str.parse::<i32>() {
            Ok(num) => num,
            Err(_) => {
                set_message.set(Some(("Please enter a valid number for the math answer".to_string(), "error".to_string())));
                return;
            }
        };

        // Clear previous messages
        set_message.set(None);
        set_requesting.set(true);

        spawn_local(async move {
            let api = FaucetApi::new();
            let request = AirdropRequest {
                public_key: pubkey,
                math_session_id: challenge.session_id,
                math_answer: answer,
            };

            add_log_entry("INFO", "Requesting airdrop...");

            match api.request_airdrop(request).await {
                Ok(response) => {
                    let success_message = format!(
                        "Airdrop successful! 0.1 XNT sent to your wallet. Transaction: {}",
                        response.signature
                    );
                    set_message.set(Some((success_message, "success".to_string())));
                    add_log_entry("SUCCESS", &format!("Airdrop successful: {}", response.signature));
                    
                    // Update session balance - wait for blockchain confirmation before fetching
                    let session_clone = session;
                    spawn_local(async move {
                        add_log_entry("INFO", "Waiting for blockchain confirmation (20 seconds)...");
                        
                        // Wait 20 seconds for blockchain confirmation
                        TimeoutFuture::new(20_000).await;
                        
                        add_log_entry("INFO", "Fetching updated balance...");
                        let mut session_update = session_clone.get_untracked();
                        match session_update.fetch_and_update_balances().await {
                            Ok(()) => {
                                log::info!("Successfully updated balances after airdrop");
                                add_log_entry("SUCCESS", &format!("Balance updated: {:.4} XNT", session_update.get_sol_balance()));
                                session_clone.update(|s| {
                                    s.set_balances(session_update.get_sol_balance(), session_update.get_token_balance());
                                });
                            },
                            Err(e) => {
                                log::error!("Failed to update balances after airdrop: {}", e);
                                add_log_entry("WARNING", "Failed to update balance automatically, will retry later");
                                // Fallback: mark that we need to update balances later
                                session_clone.update(|s| {
                                    s.mark_balance_update_needed();
                                });
                            }
                        }
                    });

                    // Clear form and refresh challenge
                    set_math_answer.set(String::new());
                    
                    // Load new challenge
                    let api = FaucetApi::new();
                    if let Ok(new_challenge) = api.get_challenge().await {
                        set_current_challenge.set(Some(new_challenge));
                    }

                    // Open transaction in explorer if user wants
                    if let Some(window) = window() {
                        let explorer_url = format!("https://explorer.x1.xyz/tx/{}", response.signature);
                        let _ = window.open_with_url_and_target(&explorer_url, "_blank");
                    }
                },
                Err(e) => {
                    set_message.set(Some((format!("Airdrop failed: {}", e), "error".to_string())));
                    add_log_entry("ERROR", &format!("Airdrop failed: {}", e));
                    
                    // If math verification failed, refresh challenge
                    if e.contains("Math challenge") {
                        let api = FaucetApi::new();
                        if let Ok(new_challenge) = api.get_challenge().await {
                            set_current_challenge.set(Some(new_challenge));
                            set_math_answer.set(String::new());
                        }
                    }
                }
            }

            set_requesting.set(false);
        });
    };

    // Copy donation address
    let copy_donation_address = move |_| {
        let donation_address = "CgYxQ5MsPmsyeUTroVm5DX8hzz48ufg8U7k12R58ftcV";
        if let Some(window) = window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(donation_address);
            set_message.set(Some(("Donation address copied to clipboard!".to_string(), "success".to_string())));
        }
    };

    view! {
        <div class="faucet-page">
            <div class="faucet-header">
                <h1>
                    <i class="fas fa-faucet"></i>
                    "X1 Testnet Faucet"
                </h1>
                <p class="faucet-description">
                    "Get free XNT tokens for testing on the X1 Testnet. Each address can request airdrop once every 24 hours."
                </p>
                
                // Faucet status indicator
                <div class="faucet-status">
                    <span class="status-label">"Faucet Status: "</span>
                    <span 
                        class="status-indicator"
                        class:status-online=move || faucet_status.get() == "online"
                        class:status-offline=move || faucet_status.get() == "offline"
                        class:status-checking=move || faucet_status.get() == "checking"
                    >
                        {move || match faucet_status.get().as_str() {
                            "online" => "🟢 Online",
                            "offline" => "🔴 Offline", 
                            _ => "🟡 Checking..."
                        }}
                    </span>
                </div>
            </div>

            // Current wallet info
            <div class="wallet-info-card">
                <h3>
                    <i class="fas fa-wallet"></i>
                    "Current Wallet"
                </h3>
                <div class="wallet-details">
                    <div class="wallet-item">
                        <label>"Public Key:"</label>
                        <span class="wallet-value" title={move || session.with(|s| s.get_public_key().unwrap_or_else(|_| "Not connected".to_string()))}>
                            {move || {
                                match session.with(|s| s.get_public_key()) {
                                    Ok(pubkey) => {
                                        if pubkey.len() >= 16 {
                                            format!("{}...{}", &pubkey[..8], &pubkey[pubkey.len()-8..])
                                        } else {
                                            pubkey
                                        }
                                    },
                                    Err(_) => "❌ Not connected".to_string()
                                }
                            }}
                        </span>
                    </div>
                    <div class="wallet-item">
                        <label>"XNT Balance:"</label>
                        <span class="wallet-value balance-value">
                            {move || format!("{:.4} XNT", session.with(|s| s.get_sol_balance()))}
                        </span>
                    </div>
                </div>
            </div>

            // Airdrop request form
            <div class="airdrop-form-card">
                <h3>
                    <i class="fas fa-coins"></i>
                    "Request Airdrop (0.1 XNT)"
                </h3>

                // Math challenge section
                <Show 
                    when=move || current_challenge.get().is_some()
                    fallback=|| view! {
                        <div class="math-challenge loading">
                            <div class="loading-spinner"></div>
                            <p>"Loading math challenge..."</p>
                        </div>
                    }
                >
                    {move || {
                        current_challenge.get().map(|challenge| {
                            view! {
                                <div class="math-challenge">
                                    <div class="challenge-header">
                                        <label>"Solve this math problem:"</label>
                                        <button 
                                            class="refresh-challenge-btn"
                                            on:click=refresh_challenge
                                            disabled=move || loading.get()
                                            title="Get new question"
                                        >
                                            <i class="fas fa-refresh"></i>
                                        </button>
                                    </div>
                                    <div class="math-question">
                                        {challenge.question}
                                    </div>
                                    <input
                                        type="number"
                                        class="math-answer-input"
                                        placeholder="Enter your answer"
                                        prop:value=move || math_answer.get()
                                        on:input=move |ev| {
                                            set_math_answer.set(event_target_value(&ev));
                                        }
                                        disabled=move || requesting.get()
                                    />
                                </div>
                            }
                        })
                    }}
                </Show>

                // Request button
                <button
                    class="airdrop-request-btn"
                    on:click=request_airdrop
                    disabled=move || {
                        requesting.get() || 
                        current_challenge.get().is_none() ||
                        math_answer.get().trim().is_empty() ||
                        session.with(|s| s.get_public_key().is_err()) ||
                        faucet_status.get() != "online"
                    }
                >
                    <Show
                        when=move || requesting.get()
                        fallback=|| view! { 
                            <>
                                <i class="fas fa-faucet"></i>
                                "Request 0.1 XNT Airdrop"
                            </> 
                        }
                    >
                        <div class="spinner"></div>
                        "Processing..."
                    </Show>
                </button>
            </div>

            // Message display
            <Show when=move || message.get().is_some()>
                {move || {
                    message.get().map(|(msg, msg_type)| {
                        let is_success = msg_type == "success";
                        let is_error = msg_type == "error";
                        
                        view! {
                            <div 
                                class="message-card"
                                class:success=is_success
                                class:error=is_error
                            >
                                {msg}
                            </div>
                        }
                    })
                }}
            </Show>

            // Information cards
            <div class="info-grid">
                <div class="info-card">
                    <div class="card-icon">
                        <i class="fas fa-coins"></i>
                    </div>
                    <div class="card-content">
                        <h4>"Airdrop Amount"</h4>
                        <p>"0.1 XNT per request"</p>
                    </div>
                </div>

                <div class="info-card">
                    <div class="card-icon">
                        <i class="fas fa-clock"></i>
                    </div>
                    <div class="card-content">
                        <h4>"Request Frequency"</h4>
                        <p>"Once every 24 hours"</p>
                    </div>
                </div>

                <div class="info-card">
                    <div class="card-icon">
                        <i class="fas fa-network-wired"></i>
                    </div>
                    <div class="card-content">
                        <h4>"Network"</h4>
                        <p>"X1 Testnet"</p>
                    </div>
                </div>
            </div>

            // Support section
            <div class="support-card">
                <h3>
                    <i class="fas fa-heart"></i>
                    "Support the Faucet"
                </h3>
                <p>"Help keep this faucet running by donating XNT tokens on X1 Testnet:"</p>
                <div class="donation-address">
                    <code>"CgYxQ5MsPmsyeUTroVm5DX8hzz48ufg8U7k12R58ftcV"</code>
                    <button 
                        class="copy-btn"
                        on:click=copy_donation_address
                        title="Copy donation address"
                    >
                        <i class="fas fa-copy"></i>
                    </button>
                </div>
                <p class="support-credit">
                    <i class="fas fa-info-circle"></i>
                    "Faucet maintained by "
                    <a href="https://x.com/xen_artist" target="_blank" rel="noopener noreferrer">"xen_artist"</a>
                    " at "
                    <a href="https://x1.wiki" target="_blank" rel="noopener noreferrer">"x1.wiki"</a>
                </p>
            </div>

            // Tips section
            <div class="tips-card">
                <h3>
                    <i class="fas fa-lightbulb"></i>
                    "Tips"
                </h3>
                <ul class="tips-list">
                    <li>
                        <i class="fas fa-check"></i>
                        "Make sure your wallet is connected before requesting tokens"
                    </li>
                    <li>
                        <i class="fas fa-check"></i>
                        "You can only request tokens once every 24 hours per address"
                    </li>
                    <li>
                        <i class="fas fa-check"></i>
                        "Testnet tokens have no real value and are only for testing"
                    </li>
                    <li>
                        <i class="fas fa-check"></i>
                        "Each request includes a simple math challenge to prevent abuse"
                    </li>
                </ul>
            </div>
        </div>
    }
}
