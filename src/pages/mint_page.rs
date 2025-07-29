use leptos::*;
use crate::core::session::Session;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use rand::Rng;

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
pub fn MintPage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (minting, set_minting) = create_signal(false);
    let (last_result, set_last_result) = create_signal::<Option<String>>(None);
    let (error_message, set_error_message) = create_signal::<Option<String>>(None);
    let (minting_status, set_minting_status) = create_signal(String::new());
    
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

    // --- Effect for action result handling ---
    create_effect(move |_| {
        if let Some(result) = start_minting.value().get() {
            set_is_submitting.set(false); // reset manual state
            set_minting_status.set(String::new()); // clear status
        }
    });

    view! {
        <div class="mint-page">
            <div class="mint-page-header">
                <h1>"New Mint Contract"</h1>
                <p>"Mint tokens using the new memo mint contract"</p>
            </div>
            
            <div class="mint-content">
                <div class="mint-controls" style="text-align: center; padding: 2rem;">
                    <button 
                        class="mint-button"
                        style="
                            padding: 1rem 2rem;
                            font-size: 1.2rem;
                            background: linear-gradient(135deg, #28a745, #20c997);
                            color: white;
                            border: none;
                            border-radius: 8px;
                            cursor: pointer;
                            transition: all 0.3s ease;
                            box-shadow: 0 4px 8px rgba(0,0,0,0.1);
                        "
                        disabled=move || start_minting.pending().get() || is_submitting.get()
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
                            let is_pending = start_minting.pending().get() || is_submitting.get();
                            if is_pending {
                                "Minting... 🔄"
                            } else {
                                "Start Minting 🚀"
                            }
                        }}
                    </button>
                    
                    <div style="margin-top: 1rem; font-size: 0.9rem; color: #666;">
                        "This will generate a random JSON memo (69-800 bytes) and mint tokens"
                    </div>
                </div>
                
                // Show minting status
                {move || {
                    let status = minting_status.get();
                    if !status.is_empty() {
                        view! {
                            <div class="minting-progress" style="
                                text-align: center;
                                padding: 1rem;
                                margin: 1rem auto;
                                max-width: 600px;
                                background: #f8f9fa;
                                border-radius: 8px;
                                border: 1px solid #dee2e6;
                            ">
                                <i class="fas fa-spinner fa-spin" style="margin-right: 8px; color: #28a745;"></i>
                                <span>{status}</span>
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                }}
                
                // Show results
                <div class="mint-results" style="max-width: 600px; margin: 2rem auto; padding: 0 1rem;">
                    {move || {
                        if let Some(error) = error_message.get() {
                            view! {
                                <div class="error-message" style="
                                    background: #f8d7da;
                                    color: #721c24;
                                    padding: 1rem;
                                    border-radius: 8px;
                                    margin-bottom: 1rem;
                                    border: 1px solid #f5c6cb;
                                ">
                                    <strong>"Error: "</strong> {error}
                                </div>
                            }.into_view()
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}
                    
                    {move || {
                        if let Some(signature) = last_result.get() {
                            view! {
                                <div class="success-message" style="
                                    background: #d4edda;
                                    color: #155724;
                                    padding: 1rem;
                                    border-radius: 8px;
                                    border: 1px solid #c3e6cb;
                                ">
                                    <strong>"✅ Mint Successful!"</strong>
                                    <div style="margin-top: 0.5rem; font-family: monospace; word-break: break-all;">
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