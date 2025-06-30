use leptos::*;
use wasm_bindgen::JsCast;
use crate::core::session::{Session, SessionError};
use crate::core::rpc_token::ProgramConfig;
use crate::core::storage_burn::get_burn_storage;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug)]
pub struct BurnOptions {
    pub personal_collection: bool,        // burn to personal onchain collection
    pub global_glory_collection: bool,    // burn to global glory onchain collection
}

impl BurnOptions {
    pub fn new() -> Self {
        Self {
            personal_collection: false,
            global_glory_collection: false,
        }
    }
}

#[component]
pub fn BurnOnchain(
    /// control modal visibility
    show_modal: ReadSignal<bool>,
    set_show_modal: WriteSignal<bool>,
    /// transaction signature to burn
    signature: ReadSignal<String>,
    /// session for signing transactions
    session: RwSignal<Session>,
    /// callback when user makes a choice
    #[prop(optional)] on_burn_choice: Option<Callback<(String, BurnOptions)>>,
    /// custom close callback (optional)
    #[prop(optional)] on_close: Option<Callback<()>>,
    /// burn success callback
    #[prop(default = Callback::new(|_: (String, u64)| {}))]
    on_burn_success: Callback<(String, u64)>,
    /// burn error callback
    #[prop(default = Callback::new(|_: String| {}))]
    on_burn_error: Callback<String>,
) -> impl IntoView {
    // State for selected options
    let (personal_collection_checked, set_personal_collection_checked) = create_signal(false);
    let (global_glory_collection_checked, set_global_glory_collection_checked) = create_signal(false);
    
    // Loading states
    let (is_burning, set_is_burning) = create_signal(false);
    let (burn_error, set_burn_error) = create_signal(String::new());

    // Handle backdrop click to close modal
    let handle_backdrop_click = move |ev: ev::MouseEvent| {
        if let Some(target) = ev.target() {
            if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                if element.class_list().contains("burn-onchain-overlay") {
                    set_show_modal.set(false);
                    if let Some(callback) = on_close {
                        callback.call(());
                    }
                }
            }
        }
    };

    // Handle close button click
    let handle_close = move |_| {
        set_show_modal.set(false);
        if let Some(callback) = on_close {
            callback.call(());
        }
    };

    // Handle burn choice confirmation
    let handle_confirm = move |_| {
        let sig = signature.get();
        let burn_options = BurnOptions {
            personal_collection: personal_collection_checked.get(),
            global_glory_collection: global_glory_collection_checked.get(),
        };
        
        set_is_burning.set(true);
        set_burn_error.set(String::new());
        
        let session_clone = session;
        let success_callback = on_burn_success;
        let error_callback = on_burn_error;
        
        spawn_local(async move {
            let result = perform_burn(sig.clone(), burn_options, session_clone).await;
            
            match result {
                Ok((transaction_sig, amount)) => {
                    log::info!("✅ Burn successful! Transaction: {}, Amount: {}", transaction_sig, amount);
                    set_is_burning.set(false);
                    set_show_modal.set(false);
                    
                    // Call success callback
                    success_callback.call((transaction_sig, amount));
                }
                Err(e) => {
                    log::error!("❌ Burn failed: {}", e);
                    set_is_burning.set(false);
                    set_burn_error.set(format!("❌ Burn failed: {}", e));
                    
                    // Call error callback
                    error_callback.call(e.to_string());
                }
            }
        });
    };

    view! {
        <div 
            class="burn-onchain-overlay"
            class:show=show_modal
            on:click=handle_backdrop_click
        >
            <div class="burn-onchain-modal">
                // Header
                <div class="burn-onchain-header">
                    <h3 class="burn-onchain-title">
                        <i class="fas fa-fire"></i>
                        " Choose Burn Options"
                    </h3>
                    <button class="burn-onchain-close-btn" on:click=handle_close>
                        <i class="fas fa-times"></i>
                    </button>
                </div>

                // Content
                <div class="burn-onchain-body">
                    <p class="description">
                        "Select your burn options (you can choose multiple):"
                    </p>

                    <div class="burn-options">
                        // Personal onchain collection option
                        <label class="burn-option">
                            <input 
                                type="checkbox"
                                checked=personal_collection_checked
                                prop:disabled=is_burning.get()
                                on:change=move |ev| {
                                    set_personal_collection_checked.set(event_target_checked(&ev));
                                }
                            />
                            <div class="option-content">
                                <div class="option-icon">
                                    <i class="fas fa-archive"></i>
                                </div>
                                <div class="option-text">
                                    <div class="option-title">"Personal Onchain Collection"</div>
                                    <div class="option-desc">"Add to your personal onchain burn history with detailed records"</div>
                                </div>
                            </div>
                        </label>

                        // Global glory onchain collection option
                        <label class="burn-option">
                            <input 
                                type="checkbox"
                                checked=global_glory_collection_checked
                                prop:disabled=is_burning.get()
                                on:change=move |ev| {
                                    set_global_glory_collection_checked.set(event_target_checked(&ev));
                                }
                            />
                            <div class="option-content">
                                <div class="option-icon">
                                    <i class="fas fa-trophy"></i>
                                </div>
                                <div class="option-text">
                                    <div class="option-title">"Global Glory Onchain Collection"</div>
                                    <div class="option-desc">"Add to the global onchain collection (requires ≥420 MEMO tokens)"</div>
                                </div>
                            </div>
                        </label>
                    </div>

                    // Error message
                    <Show when=move || !burn_error.get().is_empty()>
                        <div class="burn-error">
                            <i class="fas fa-exclamation-triangle"></i>
                            {burn_error}
                        </div>
                    </Show>

                    // Information note
                    <div class="burn-info">
                        <p class="info-note">
                            <i class="fas fa-info-circle"></i>
                            " Note: You can select both options, one option, or neither."
                        </p>
                        <p class="info-note">
                            <i class="fas fa-trophy"></i>
                            " Global Glory Onchain Collection requires at least 420 MEMO tokens to participate."
                        </p>
                        <p class="info-note">
                            <i class="fas fa-database"></i>
                            " Without Personal Onchain Collection: records are saved locally only (latest 100 records)."
                        </p>
                        <p class="info-note">
                            <i class="fas fa-link"></i>
                            " Without Global Glory Onchain Collection: records are saved to onchain latest burns only (latest 69 records)."
                        </p>
                    </div>
                </div>

                // Footer
                <div class="burn-onchain-footer">
                    <button 
                        class="btn confirm-btn" 
                        prop:disabled=is_burning.get()
                        on:click=handle_confirm
                    >
                        {move || {
                            if is_burning.get() {
                                view! {
                                    <>
                                        <i class="fas fa-spinner fa-spin"></i>
                                        " Burning..."
                                    </>
                                }
                            } else {
                                view! {
                                    <>
                                        <i class="fas fa-fire"></i>
                                        " Confirm Burn"
                                    </>
                                }
                            }
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}

// 🎯 修正的Burn执行函数
async fn perform_burn(
    signature: String, 
    burn_options: BurnOptions, 
    session: RwSignal<Session>
) -> Result<(String, u64), String> {
    // Calculate burn amount based on options
    let amount = if burn_options.global_glory_collection {
        // Global Glory Collection requires at least 420 tokens
        ProgramConfig::TOP_BURN_THRESHOLD
    } else {
        // Regular burn requires at least 1 token
        ProgramConfig::MIN_BURN_AMOUNT
    };
    
    log::info!("🔥 Starting burn process:");
    log::info!("  - Signature: {}", signature);
    log::info!("  - Amount: {} lamports ({} tokens)", amount, amount / 1_000_000_000);
    log::info!("  - Personal Collection: {}", burn_options.personal_collection);
    log::info!("  - Global Glory Collection: {}", burn_options.global_glory_collection);

    // Create burn message
    let burn_message = if burn_options.personal_collection && burn_options.global_glory_collection {
        "Burned for both personal onchain collection and global glory onchain collection"
    } else if burn_options.personal_collection {
        "Burned for personal onchain collection"
    } else if burn_options.global_glory_collection {
        "Burned for global glory onchain collection"
    } else {
        "Regular burn transaction"
    };

    // 🎯 正确的方式：直接调用session的异步burn方法
    let transaction_signature = {
        let mut session_data = session.get();
        
        if burn_options.personal_collection {
            // TODO: 未来支持burn_with_history
            session_data.burn(amount, burn_message, &signature).await
        } else {
            session_data.burn(amount, burn_message, &signature).await
        }
    }.map_err(|e| format!("Session burn failed: {}", e))?;

    // 更新session状态（标记余额需要更新）
    session.update(|s| s.mark_balance_update_needed());

    log::info!("✅ Burn transaction submitted: {}", transaction_signature);

    // Create memo JSON for storage
    let memo_json = serde_json::json!({
        "signature": signature,
        "message": burn_message,
        "personal_collection": burn_options.personal_collection,
        "global_glory_collection": burn_options.global_glory_collection,
        "transaction_signature": transaction_signature
    }).to_string();

    // Save to local storage
    let burn_storage = get_burn_storage();
    burn_storage.save_burn_record_async(&transaction_signature, &memo_json, amount).await
        .map_err(|e| format!("Failed to save burn record: {}", e))?;

    log::info!("✅ Burn record saved to local storage");

    Ok((transaction_signature, amount))
} 