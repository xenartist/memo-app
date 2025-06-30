use leptos::*;
use crate::pages::pixel_view::PixelView;
use crate::pages::memo_card::MemoDetails;
use crate::pages::burn_onchain::{BurnOnchain, BurnOptions};
use crate::core::session::Session;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;

#[component]
pub fn MemoCardDetails(
    /// control modal visibility
    show_modal: ReadSignal<bool>,
    set_show_modal: WriteSignal<bool>,
    /// current details
    memo_details: ReadSignal<Option<MemoDetails>>,
    /// burn button callback (optional) - update to handle burn choice callback
    #[prop(optional)] on_burn_choice: Option<Callback<(String, BurnOptions)>>,
    /// custom close callback (optional)
    #[prop(optional)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    let (show_copied, set_show_copied) = create_signal(false);
    
    // add state for BurnOnchain dialog
    let (show_burn_onchain, set_show_burn_onchain) = create_signal(false);
    let (burn_signature, set_burn_signature) = create_signal(String::new());
    
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

    // handle burn button click - update to open BurnOnchain dialog
    let handle_burn = move |signature: String| {
        set_burn_signature.set(signature);
        set_show_burn_onchain.set(true);
    };

    // handle burn choice from BurnOnchain component
    let handle_burn_choice = move |signature: String, burn_options: BurnOptions| {
        if let Some(callback) = &on_burn_choice {
            callback.call((signature, burn_options));
        }
        set_show_burn_onchain.set(false);
        handle_close(); // also close details dialog
    };

    // handle burn onchain close
    let handle_burn_onchain_close = move |_: ()| {
        set_show_burn_onchain.set(false);
    };

    // copy signature to clipboard
    let copy_signature = move |signature: String| {
        if let Some(window) = window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&signature);
            
            // show copied success message
            set_show_copied.set(true);
            
            // hide copied success message after 1.5 seconds
            spawn_local(async move {
                TimeoutFuture::new(1500).await;
                set_show_copied.set(false);
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

                                        // Signature section
                                        <div class="detail-section">
                                            <h4 class="detail-label">
                                                <i class="fas fa-signature"></i>
                                                "Signature:"
                                            </h4>
                                            <div class="detail-value signature-container">
                                                <span class="signature-text">{details.signature.clone()}</span>
                                                <div class="copy-container">
                                                    <button 
                                                        class="copy-button"
                                                        on:click={
                                                            let sig = details.signature.clone();
                                                            move |_| copy_signature(sig.clone())
                                                        }
                                                        title="Copy signature"
                                                    >
                                                        <i class="fas fa-copy"></i>
                                                    </button>
                                                    <div class="copy-tooltip" class:show=show_copied>
                                                        "Copied!"
                                                    </div>
                                                </div>
                                            </div>
                                        </div>

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

                                        // Burn button (only show if callback is provided)
                                        {
                                            if on_burn_choice.is_some() {
                                                let sig = details.signature.clone();
                                                view! {
                                                    <div class="detail-actions">
                                                        <button 
                                                            class="detail-burn-btn"
                                                            on:click=move |_| {
                                                                log::info!("Burn clicked from details for signature: {}", sig);
                                                                handle_burn(sig.clone());
                                                            }
                                                        >
                                                            <i class="fas fa-fire"></i>
                                                            " Burn This MEMO"
                                                        </button>
                                                    </div>
                                                }.into_view()
                                            } else {
                                                view! { <div></div> }.into_view()
                                            }
                                        }
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

        // BurnOnchain dialog
        <BurnOnchain
            show_modal=show_burn_onchain.into()
            set_show_modal=set_show_burn_onchain
            signature=burn_signature.into()
            on_burn_choice=Callback::new(move |(sig, burn_options): (String, BurnOptions)| {
                handle_burn_choice(sig, burn_options);
            })
            on_close=Callback::new(handle_burn_onchain_close)
        />
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