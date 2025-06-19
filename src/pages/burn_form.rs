use leptos::*;
use crate::core::session::Session;
use crate::pages::memo_card_details::MemoCardDetails;
use crate::pages::memo_card::MemoDetails;

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

    // Load memo details with dummy data
    let load_memo_details = move || {
        let signature = signature_input.get().trim().to_string();
        if signature.is_empty() {
            set_error_message.set("❌ Please enter a transaction signature".to_string());
            return;
        }

        set_is_loading.set(true);
        set_error_message.set(String::new());

        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(1500).await;

            // Create dummy MemoDetails for the loaded MEMO
            let memo_details = MemoDetails {
                title: Some("Test MEMO Token".to_string()),
                image: Some("data:image/png;base64,test".to_string()), // dummy image data
                content: Some("This is a test MEMO token with some content for burning demonstration.".to_string()),
                signature: signature,
                pubkey: "BurnTestAddress1234567890".to_string(),
                blocktime: 1700000000, // timestamp
                amount: Some(100.0),
            };

            set_current_memo_details.set(Some(memo_details));
            set_show_details_modal.set(true); // Show the details modal
            set_is_loading.set(false);
            set_error_message.set("✅ MEMO information loaded successfully".to_string());
        });
    };

    // Handle burn callback from MemoCardDetails
    let handle_burn_from_details = Callback::new(move |signature: String| {
        log::info!("Burn initiated from details for signature: {}", signature);
        
        // Simulate burn process
        wasm_bindgen_futures::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(2000).await;
            
            // Close the details modal
            set_show_details_modal.set(false);
            set_error_message.set("✅ Burn transaction completed successfully".to_string());
            
            // Call the success callback if provided
            if let Some(callback) = on_burn_success {
                callback.call((signature, 100)); // dummy amount
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
                            " Burn MEMO Token"
                        </h3>
                        <p>"Enter the transaction signature to load and burn the MEMO token"</p>
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
                                            "Loading..."
                                        } else {
                                            "Load MEMO"
                                        }
                                    }}
                                </button>
                            </div>
                        </div>
                    </div>

                    // Loading indicator
                    <Show when=move || is_loading.get()>
                        <div class="loading-status">
                            <i class="fas fa-spinner fa-spin"></i>
                            " Loading MEMO information from blockchain..."
                        </div>
                    </Show>

                    // Error/Success messages
                    {move || {
                        let message = error_message.get();
                        if !message.is_empty() {
                            view! {
                                <div class="error-message" 
                                    class:success=message.contains("✅")
                                    class:error=message.contains("❌")
                                    class:warning=message.contains("⚠️")
                                >
                                    {message}
                                </div>
                            }
                        } else {
                            view! { <div></div> }
                        }
                    }}

                    // Instructions when no memo loaded
                    <Show when=move || !is_loading.get() && current_memo_details.get().is_none()>
                        <div class="instructions">
                            <div class="instruction-content">
                                <i class="fas fa-info-circle"></i>
                                <p>"Enter a transaction signature above and click 'Load MEMO' to view the token details and burn it."</p>
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
                on_burn_click=handle_burn_from_details
                on_close=handle_details_close
            />
        </div>
    }
}
