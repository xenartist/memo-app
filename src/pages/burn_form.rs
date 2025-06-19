use leptos::*;
use crate::core::session::Session;
use crate::pages::pixel_view::PixelView;

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
    let (is_burning, set_is_burning) = create_signal(false);
    let (memo_loaded, set_memo_loaded) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());

    // Dummy memo details
    let (memo_title, set_memo_title) = create_signal(Option::<String>::None);
    let (memo_image, set_memo_image) = create_signal(Option::<String>::None);
    let (memo_content, set_memo_content) = create_signal(Option::<String>::None);
    let (memo_signature, set_memo_signature) = create_signal(String::new());
    let (memo_pubkey, set_memo_pubkey) = create_signal(String::new());
    let (memo_blocktime, set_memo_blocktime) = create_signal(0i64);
    let (memo_amount, set_memo_amount) = create_signal(0.0f64);

    // Format timestamp function
    let format_timestamp = move |timestamp: i64| -> String {
        let date = js_sys::Date::new(&(timestamp as f64 * 1000.0).into());
        date.to_locale_string("en-US", &js_sys::Object::new())
            .as_string()
            .unwrap_or_else(|| "Unknown".to_string())
    };

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

            set_memo_title.set(Some("Test MEMO Token".to_string()));
            set_memo_image.set(Some("data:image/png;base64,test".to_string()));
            set_memo_content.set(Some("This is a test MEMO token with some content for burning demonstration.".to_string()));
            set_memo_signature.set(signature);
            set_memo_pubkey.set("BurnTestAddress1234567890".to_string());
            set_memo_blocktime.set(1700000000);
            set_memo_amount.set(100.0);

            set_memo_loaded.set(true);
            set_is_loading.set(false);
            set_error_message.set("✅ MEMO information loaded successfully".to_string());
        });
    };

    // Handle burn operation - 像 memo_card_details.rs 一样定义
    let handle_burn = move |signature: String, amount: u64| {
        if let Some(callback) = &on_burn_success {
            callback.call((signature, amount));
        }
    };

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
                                    prop:disabled=is_loading.get() || is_burning.get()
                                    on:input=move |ev| {
                                        set_signature_input.set(event_target_value(&ev));
                                        set_memo_loaded.set(false);
                                        set_error_message.set(String::new());
                                    }
                                />
                                <button
                                    type="button"
                                    class="load-btn"
                                    prop:disabled=signature_input.get().trim().is_empty() || is_loading.get() || is_burning.get()
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

                    // MEMO details section
                    <Show when=move || memo_loaded.get()>
                        <div class="memo-details-content">
                            // Title
                            <div class="detail-section">
                                <h4 class="detail-label">
                                    <i class="fas fa-quote-left"></i>
                                    "Title:"
                                </h4>
                                <div class="detail-value">
                                    {move || memo_title.get().unwrap_or_else(|| "No title".to_string())}
                                </div>
                            </div>

                            // Image
                            <div class="detail-section">
                                <h4 class="detail-label">
                                    <i class="fas fa-image"></i>
                                    "Image:"
                                </h4>
                                <div class="detail-value">
                                    <div class="detail-image">
                                        {move || {
                                            if let Some(image_data) = memo_image.get() {
                                                if image_data.starts_with("http") || image_data.starts_with("data:") {
                                                    view! {
                                                        <img 
                                                            src=image_data
                                                            alt="Memory Image"
                                                            class="detail-image-display"
                                                        />
                                                    }.into_view()
                                                } else {
                                                    view! {
                                                        <div class="detail-pixel-art">
                                                            <PixelView
                                                                art=image_data
                                                                size=200
                                                                editable=false
                                                                on_click=Box::new(|_, _| {})
                                                            />
                                                        </div>
                                                    }.into_view()
                                                }
                                            } else {
                                                view! {
                                                    <div class="no-image-placeholder">
                                                        <p>"No image"</p>
                                                    </div>
                                                }.into_view()
                                            }
                                        }}
                                    </div>
                                </div>
                            </div>

                            // Content
                            <div class="detail-section">
                                <h4 class="detail-label">
                                    <i class="fas fa-file-text"></i>
                                    "Content:"
                                </h4>
                                <div class="detail-value">
                                    <div class="content-text">
                                        {move || memo_content.get().unwrap_or_else(|| "No content".to_string())}
                                    </div>
                                </div>
                            </div>

                            // Signature (readonly display)
                            <div class="detail-section">
                                <h4 class="detail-label">
                                    <i class="fas fa-signature"></i>
                                    "Signature:"
                                </h4>
                                <div class="detail-value">
                                    <div class="signature-text">
                                        {move || memo_signature.get()}
                                    </div>
                                </div>
                            </div>

                            // From
                            <div class="detail-section">
                                <h4 class="detail-label">
                                    <i class="fas fa-user"></i>
                                    "From:"
                                </h4>
                                <div class="detail-value">
                                    <div class="pubkey-text">
                                        {move || memo_pubkey.get()}
                                    </div>
                                </div>
                            </div>

                            // Time
                            <div class="detail-section">
                                <h4 class="detail-label">
                                    <i class="fas fa-clock"></i>
                                    "Time:"
                                </h4>
                                <div class="detail-value">
                                    {move || format_timestamp(memo_blocktime.get())}
                                </div>
                            </div>

                            // Burn button
                            <div class="detail-actions">
                                <button 
                                    class="detail-burn-btn"
                                    prop:disabled=is_burning.get()
                                    on:click=move |_| {
                                        if !memo_loaded.get() {
                                            set_error_message.set("❌ Please load MEMO details first".to_string());
                                            return;
                                        }

                                        set_is_burning.set(true);
                                        set_error_message.set(String::new());

                                        let signature = memo_signature.get();
                                        let amount = memo_amount.get() as u64;

                                        wasm_bindgen_futures::spawn_local(async move {
                                            gloo_timers::future::TimeoutFuture::new(2000).await;

                                            set_is_burning.set(false);
                                            set_error_message.set("✅ Burn transaction completed successfully".to_string());

                                            handle_burn(signature, amount);
                                        });
                                    }
                                >
                                    <i class="fas fa-fire"></i>
                                    {move || {
                                        if is_burning.get() {
                                            " Burning..."
                                        } else {
                                            " Burn This Memory"
                                        }
                                    }}
                                </button>
                            </div>
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
                </div>
            </Show>

            // Show warning when no profile
            <Show when=move || !session.get().has_user_profile()>
                <div class="no-profile-message">
                    <h3>"Profile Required"</h3>
                    <p>"Please create your mint profile in the Profile page before you can burn tokens."</p>
                </div>
            </Show>
        </div>
    }
}
