use leptos::*;
use crate::core::session::Session;
use crate::core::rpc_base::RpcConnection;
use crate::pages::memo_card_details::MemoCardDetails;
use crate::pages::memo_card::MemoDetails;
use crate::pages::burn_onchain::BurnOptions;

#[component]
pub fn BurnForm(
    session: RwSignal<Session>,
    #[prop(optional)] class: Option<&'static str>,
    #[prop(optional)] on_burn_success: Option<Callback<(String, u64)>>,
    #[prop(optional)] on_burn_error: Option<Callback<String>>,
) -> impl IntoView {
    let class_str = class.unwrap_or("");
    
    // Form state
    let (signature_input, set_signature_input) = create_signal(String::new());
    let (is_loading, set_is_loading) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    
    // State for memo details modal (used by MemoCardDetails component)
    let (show_details_modal, set_show_details_modal) = create_signal(false);
    let (current_memo_details, set_current_memo_details) = create_signal(Option::<MemoDetails>::None);

    // Parse memo JSON to extract structured data
    let parse_memo_content = |memo_string: &str| -> (Option<String>, Option<String>, Option<String>) {
        match serde_json::from_str::<serde_json::Value>(memo_string) {
            Ok(memo_json) => {
                // Try to extract structured memo data (for mint transactions)
                let title = memo_json.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
                let image = memo_json.get("image").and_then(|v| v.as_str()).map(|s| s.to_string());
                let content = memo_json.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                // If it's a burn memo (has signature and message fields)
                if memo_json.get("signature").is_some() && memo_json.get("message").is_some() {
                    let burn_message = memo_json.get("message").and_then(|v| v.as_str()).map(|s| s.to_string());
                    return (
                        Some("Burn Transaction".to_string()),
                        None, // burn transactions don't have images
                        burn_message
                    );
                }
                
                (title, image, content)
            }
            Err(_) => {
                // If it's not JSON, treat as plain text content
                (None, None, Some(memo_string.to_string()))
            }
        }
    };

    // Load memo details from blockchain
    let load_memo_details = move || {
        let signature = signature_input.get().trim().to_string();
        if signature.is_empty() {
            set_error_message.set("❌ Please enter a transaction signature".to_string());
            return;
        }

        set_is_loading.set(true);
        set_error_message.set(String::new());

        wasm_bindgen_futures::spawn_local(async move {
            let rpc = RpcConnection::new();
            
            // Get the memo info from the transaction
            match rpc.get_transaction_memo(&signature).await {
                Ok(Some(memo_info)) => {
                    let (title, image, content) = parse_memo_content(&memo_info.memo);
                    
                    // Format signer address to show first 8 and last 8 characters
                    let formatted_signer = if memo_info.signer.len() >= 16 {
                        format!("{}...{}", &memo_info.signer[..8], &memo_info.signer[memo_info.signer.len()-8..])
                    } else {
                        memo_info.signer.clone()
                    };
                    
                    let memo_details = MemoDetails {
                        title,
                        image,
                        content,
                        signature: signature.clone(),
                        pubkey: formatted_signer,
                        blocktime: memo_info.timestamp,
                        amount: None, // We don't have amount info from memo
                    };

                    set_current_memo_details.set(Some(memo_details));
                    set_show_details_modal.set(true);
                    set_is_loading.set(false);
                    set_error_message.set("✅ MEMO information loaded successfully".to_string());
                }
                Ok(None) => {
                    set_is_loading.set(false);
                    set_error_message.set("❌ No MEMO data found in this transaction".to_string());
                }
                Err(e) => {
                    log::error!("Failed to get memo from transaction: {}", e);
                    set_is_loading.set(false);
                    set_error_message.set(format!("❌ Error loading MEMO: {}", e));
                }
            }
        });
    };

    // Handle burn callback from MemoCardDetails
    let handle_burn_from_details = Callback::new(move |(signature, burn_options): (String, BurnOptions)| {
        log::info!("Burn choice made from burn form for signature: {}, options: {:?}", signature, burn_options);
        
        // handle different burn options combinations
        if burn_options.personal_collection && burn_options.global_glory_board {
            log::info!("Burning to both personal collection and global glory board: {}", signature);
            // TODO: implement logic to add to both personal collection and global glory board
        } else if burn_options.personal_collection {
            log::info!("Burning to personal collection only: {}", signature);
            // TODO: implement logic to add to personal collection
        } else if burn_options.global_glory_board {
            log::info!("Burning to global glory board only: {}", signature);
            // TODO: implement logic to add to global glory board
        } else {
            log::info!("Regular burn (no special options): {}", signature);
            // TODO: implement regular burn logic
        }
        
        // simulate burn completion, call success callback
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(2000).await;
            
            // Close the details modal
            set_show_details_modal.set(false);
            set_error_message.set("✅ Burn transaction completed successfully".to_string());
            
            // call the success callback if provided
            if let Some(callback) = on_burn_success {
                callback.call((signature, 100)); // dummy amount for now
            }
        });
    });

    // Handle modal close
    let handle_details_close = Callback::new(move |_: ()| {
        set_show_details_modal.set(false);
    });

    view! {
        <div class=format!("burn-form-component {}", class_str)>
            <Show when=move || session.get().has_user_profile()>
                <div class="burn-form">
                    <div class="form-header">
                        <h3>
                            <i class="fas fa-fire"></i>
                            " Burn MEMO"
                        </h3>
                        <p>"Enter the transaction signature to load and burn the MEMO"</p>
                    </div>

                    // Signature input section
                    <div class="signature-section">
                        <div class="form-group">
                            <label for="signature">
                                <i class="fas fa-signature"></i>
                                " Transaction Signature:"
                            </label>
                            <div class="input-group">
                                <input
                                    type="text"
                                    id="signature"
                                    class="form-input"
                                    placeholder="Enter transaction signature..."
                                    prop:value=signature_input
                                    prop:disabled=is_loading.get()
                                    on:input=move |ev| {
                                        set_signature_input.set(event_target_value(&ev));
                                        set_error_message.set(String::new());
                                    }
                                />
                                <button
                                    type="button"
                                    class="load-btn"
                                    prop:disabled=signature_input.get().trim().is_empty() || is_loading.get()
                                    on:click=move |_| load_memo_details()
                                >
                                    {move || {
                                        if is_loading.get() {
                                            view! {
                                                <span>
                                                    <i class="fas fa-spinner fa-spin"></i>
                                                    " Loading..."
                                                </span>
                                            }
                                        } else {
                                            view! {
                                                <span>
                                                    <i class="fas fa-search"></i>
                                                    " Load MEMO"
                                                </span>
                                            }
                                        }
                                    }}
                                </button>
                            </div>
                        </div>

                        // Status message
                        <Show when=move || !error_message.get().is_empty()>
                            <div class="status-message" class:error=move || error_message.get().starts_with("❌") class:success=move || error_message.get().starts_with("✅")>
                                {error_message}
                            </div>
                        </Show>
                    </div>

                    // Show loading state when searching for memo
                    <Show when=move || is_loading.get() && current_memo_details.get().is_none()>
                        <div class="loading-placeholder">
                            <div class="loading-content">
                                <i class="fas fa-spinner fa-spin"></i>
                                <p>"Loading MEMO details from blockchain..."</p>
                            </div>
                        </div>
                    </Show>
                    
                    // Show helper text when no signature is entered
                    <Show when=move || signature_input.get().trim().is_empty() && !is_loading.get() && current_memo_details.get().is_none()>
                        <div class="helper-text">
                            <div class="helper-content">
                                <i class="fas fa-info-circle"></i>
                                <p>"Enter a transaction signature above and click 'Load MEMO' to view the MEMO details and burn it."</p>
                            </div>
                        </div>
                    </Show>
                </div>
            </Show>

            // Show warning when no profile
            <Show when=move || !session.get().has_user_profile()>
                <div class="no-profile-message">
                    <h3>"Profile Required"</h3>
                    <p>"Please create your mint profile in the Profile page before you can burn tokens."</p>
                </div>
            </Show>

            // Reuse MemoCardDetails component for displaying loaded memo details
            <MemoCardDetails
                show_modal=show_details_modal.into()
                set_show_modal=set_show_details_modal
                memo_details=current_memo_details.into()
                on_burn_choice=handle_burn_from_details
                on_close=handle_details_close
            />
        </div>
    }
}
