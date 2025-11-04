use leptos::*;
use crate::pages::pixel_view::PixelView;
use crate::pages::memo_card::MemoDetails;
use crate::core::session::Session;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;

// formats signature display
fn format_signature_display(signature: &str) -> String {
    if signature.len() >= 24 {
        format!("{}...{}", &signature[..12], &signature[signature.len()-12..])
    } else {
        signature.to_string()
    }
}

#[component]
pub fn MemoCardDetails(
    /// control modal visibility
    show_modal: ReadSignal<bool>,
    set_show_modal: WriteSignal<bool>,
    /// current details
    memo_details: ReadSignal<Option<MemoDetails>>,
    /// session for signing transactions
    session: RwSignal<Session>,
    /// custom close callback (optional)
    #[prop(optional)] on_close: Option<Callback<()>>,
    /// burn success callback (optional)
    #[prop(optional)] on_burn_success: Option<Callback<(String, u64)>>,
    /// burn error callback (optional)
    #[prop(optional)] on_burn_error: Option<Callback<String>>,
) -> impl IntoView {
    // ✅ create independent state for each copy button
    let (show_copied_mint, set_show_copied_mint) = create_signal(false);
    let (show_copied_burn, set_show_copied_burn) = create_signal(false);
    
    // format timestamp function
    let format_timestamp = move |timestamp: i64| -> String {
        let date = js_sys::Date::new(&(timestamp as f64 * 1000.0).into());
        date.to_locale_string("en-US", &js_sys::Object::new())
            .as_string()
            .unwrap_or_else(|| "Unknown".to_string())
    };

    // handle modal close
    let handle_close = move || {
        set_show_modal.set(false);
        if let Some(callback) = &on_close {
            callback.call(());
        }
    };

    // Burn functionality removed - burn_onchain has been deprecated

    // ✅ create copy functions for mint and burn signatures
    let copy_mint_signature = move |signature: String| {
        if let Some(window) = window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&signature);
            
            // show copied success message for mint signature
            set_show_copied_mint.set(true);
            
            // hide copied success message after 1.5 seconds
            spawn_local(async move {
                TimeoutFuture::new(1500).await;
                set_show_copied_mint.set(false);
            });
        }
    };

    let copy_burn_signature = move |signature: String| {
        if let Some(window) = window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&signature);
            
            // show copied success message for burn signature
            set_show_copied_burn.set(true);
            
            // hide copied success message after 1.5 seconds
            spawn_local(async move {
                TimeoutFuture::new(1500).await;
                set_show_copied_burn.set(false);
            });
        }
    };

    view! {
        <Show when=move || show_modal.get()>
            <div class="modal-overlay" on:click=move |_| handle_close()>
                <div class="modal-content details-modal" on:click=|e| e.stop_propagation()>
                    <div class="modal-header">
                        <h3>"🔍 MEMO Details"</h3>
                        <button 
                            class="modal-close-btn"
                            on:click=move |_| handle_close()
                            title="Close"
                        >
                            "×"
                        </button>
                    </div>
                    
                    <div class="modal-body">
                        {move || {
                            if let Some(details) = memo_details.get() {
                                // ✅ clone values that might be used multiple times
                                let has_burn_signature = details.burn_signature.is_some();
                                let burn_signature_clone = details.burn_signature.clone();
                                
                                view! {
                                    <div class="memo-details-content">
                                        // Title section
                                        {move || {
                                            if let Some(ref title) = details.title {
                                                view! {
                                                    <div class="detail-section">
                                                        <h4 class="detail-label">
                                                            <i class="fas fa-pencil"></i>
                                                            "Title:"
                                                        </h4>
                                                        <div class="detail-value">{title.clone()}</div>
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! { <div></div> }.into_view()
                                            }
                                        }}

                                        // Image section
                                        {move || {
                                            if let Some(ref image) = details.image {
                                                if image.starts_with("http") || image.starts_with("data:image") {
                                                    view! {
                                                        <div class="detail-section">
                                                            <h4 class="detail-label">
                                                                <i class="fas fa-image"></i>
                                                                "Image:"
                                                            </h4>
                                                            <div class="detail-value">
                                                                <img src={image.clone()} alt="MEMO Image" class="detail-image" />
                                                            </div>
                                                        </div>
                                                    }.into_view()
                                                } else {
                                                    // pixel art encoded
                                                    view! {
                                                        <div class="detail-section">
                                                            <h4 class="detail-label">
                                                                <i class="fas fa-palette"></i>
                                                                "Pixel Art:"
                                                            </h4>
                                                            <div class="detail-value">
                                                                <PixelView
                                                                    art={image.clone()}
                                                                    size=256
                                                                />
                                                            </div>
                                                        </div>
                                                    }.into_view()
                                                }
                                            } else {
                                                view! {
                                                    <div class="detail-section">
                                                        <h4 class="detail-label">
                                                            <i class="fas fa-image"></i>
                                                            "Image:"
                                                        </h4>
                                                        <div class="detail-value">
                                                            <div class="no-image-placeholder">
                                                                <p>"No image available"</p>
                                                            </div>
                                                        </div>
                                                    </div>
                                                }.into_view()
                                            }
                                        }}

                                        // Content section
                                        {move || {
                                            if let Some(ref content) = details.content {
                                                if !content.trim().is_empty() {
                                                    view! {
                                                        <div class="detail-section">
                                                            <h4 class="detail-label">
                                                                <i class="fas fa-file-text"></i>
                                                                "Content:"
                                                            </h4>
                                                            <div class="detail-value content-text">{content.clone()}</div>
                                                        </div>
                                                    }.into_view()
                                                } else {
                                                    view! { <div></div> }.into_view()
                                                }
                                            } else {
                                                view! { <div></div> }.into_view()
                                            }
                                        }}

                                        // Signature (Mint) section
                                        <div class="detail-section">
                                            <h4 class="detail-label">
                                                <i class="fas fa-signature"></i>
                                                {if has_burn_signature { "Signature (Mint):" } else { "Signature:" }}
                                            </h4>
                                            <div class="detail-value signature-container">
                                                <span class="signature-text">{format_signature_display(&details.signature)}</span>
                                                <div class="copy-container">
                                                    <button 
                                                        class="copy-button"
                                                        on:click={
                                                            let sig = details.signature.clone();
                                                            move |_| copy_mint_signature(sig.clone())
                                                        }
                                                        title="Copy mint signature"
                                                    >
                                                        <i class="fas fa-copy"></i>
                                                    </button>
                                                    // ✅ use independent state
                                                    <div class="copy-tooltip" class:show=show_copied_mint>
                                                        "Copied!"
                                                    </div>
                                                </div>
                                            </div>
                                        </div>

                                        // ✅ Burn Signature section (only show when burn_signature is present)
                                        {if let Some(burn_sig) = burn_signature_clone {
                                            view! {
                                                <div class="detail-section">
                                                    <h4 class="detail-label">
                                                        <i class="fas fa-fire"></i>
                                                        "Signature (Burn):"
                                                    </h4>
                                                    <div class="detail-value signature-container">
                                                        <span class="signature-text">{format_signature_display(&burn_sig)}</span>
                                                        <div class="copy-container">
                                                            <button 
                                                                class="copy-button"
                                                                on:click={
                                                                    let sig = burn_sig.clone();
                                                                    move |_| copy_burn_signature(sig.clone())
                                                                }
                                                                title="Copy burn signature"
                                                            >
                                                                <i class="fas fa-copy"></i>
                                                            </button>
                                                            // ✅ use independent state
                                                            <div class="copy-tooltip" class:show=show_copied_burn>
                                                                "Copied!"
                                                            </div>
                                                        </div>
                                                    </div>
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! { <div></div> }.into_view()
                                        }}

                                        // From section
                                        <div class="detail-section">
                                            <h4 class="detail-label">
                                                <i class="fas fa-user"></i>
                                                "From:"
                                            </h4>
                                            <div class="detail-value">
                                                {
                                                    let pubkey = details.pubkey.clone();
                                                    if pubkey.len() >= 16 {
                                                        format!("{}...{}", &pubkey[..8], &pubkey[pubkey.len()-8..])
                                                    } else {
                                                        pubkey
                                                    }
                                                }
                                            </div>
                                        </div>

                                        // Time section
                                        <div class="detail-section">
                                            <h4 class="detail-label">
                                                <i class="fas fa-clock"></i>
                                                "Time:"
                                            </h4>
                                            <div class="detail-value">
                                                {format_timestamp(details.blocktime)}
                                            </div>
                                        </div>

                                        // Amount section
                                        {move || {
                                            if let Some(amount_value) = details.amount {
                                                view! {
                                                    <div class="detail-section">
                                                        <h4 class="detail-label">
                                                            <i class="fas fa-coins"></i>
                                                            "Amount:"
                                                        </h4>
                                                        <div class="detail-value">
                                                            {format!("{:.2} tokens", amount_value)}
                                                        </div>
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! { <div></div> }.into_view()
                                            }
                                        }}

                                        // Burn button removed - burn_onchain functionality has been deprecated
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <div class="no-details">
                                        <p>"No details available"</p>
                                    </div>
                                }.into_view()
                            }
                        }}
                    </div>
                </div>
            </div>
        </Show>

    }
}

fn format_timestamp(timestamp: i64) -> String {
    use js_sys::Date;
    let date = Date::new(&wasm_bindgen::JsValue::from_f64(timestamp as f64 * 1000.0));
    format!("{}/{}/{} {}:{}:{}", 
        date.get_month() + 1,
        date.get_date(),
        date.get_full_year(),
        date.get_hours(),
        date.get_minutes(),
        date.get_seconds()
    )
}