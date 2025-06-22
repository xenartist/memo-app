use leptos::*;
use crate::pages::memo_card::MemoDetails;
use crate::pages::pixel_view::PixelView;
use web_sys::window;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn MemoCardDetails(
    /// control modal visibility
    show_modal: ReadSignal<bool>,
    set_show_modal: WriteSignal<bool>,
    /// current details
    memo_details: ReadSignal<Option<MemoDetails>>,
    /// burn button callback (optional)
    #[prop(optional)] on_burn_click: Option<Callback<String>>,
    /// custom close callback (optional)
    #[prop(optional)] on_close: Option<Callback<()>>,
) -> impl IntoView {
    
    // copy status
    let (show_copied, set_show_copied) = create_signal(false);
    
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

    // handle burn button click
    let handle_burn = move |signature: String| {
        if let Some(callback) = &on_burn_click {
            callback.call(signature);
        }
        // can close modal after burn
        // handle_close();
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
                                        // Title
                                        <div class="detail-section">
                                            <h4 class="detail-label">
                                                <i class="fas fa-pencil"></i>
                                                "Title:"
                                            </h4>
                                            <div class="detail-value">
                                                {details.title.clone().unwrap_or_else(|| "Memory".to_string())}
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
                                                    {if let Some(ref image_data) = details.image {
                                                        if image_data.starts_with("http") || image_data.starts_with("data:") {
                                                            view! {
                                                                <img 
                                                                    src={image_data.clone()}
                                                                    alt="Memory Image"
                                                                    class="detail-image-display"
                                                                />
                                                            }.into_view()
                                                        } else {
                                                            view! {
                                                                <div class="detail-pixel-art">
                                                                    <PixelView
                                                                        art={image_data.clone()}
                                                                        size=200
                                                                        editable=false
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
                                                    {details.content.clone().unwrap_or_else(|| "No content".to_string())}
                                                </div>
                                            </div>
                                        </div>

                                        // Signature - add copy button
                                        <div class="detail-section">
                                            <h4 class="detail-label">
                                                <i class="fas fa-signature"></i>
                                                "Signature:"
                                            </h4>
                                            <div class="detail-value">
                                                <div class="signature-container">
                                                    <div class="signature-text">
                                                        {details.signature.clone()}
                                                    </div>
                                                    <div class="copy-container">
                                                        <button
                                                            class="copy-button"
                                                            on:click={
                                                                let sig = details.signature.clone();
                                                                move |e| {
                                                                    e.stop_propagation();
                                                                    copy_signature(sig.clone());
                                                                }
                                                            }
                                                            title="Copy signature to clipboard"
                                                        >
                                                            <i class="fas fa-copy"></i>
                                                        </button>
                                                        <div 
                                                            class="copy-tooltip"
                                                            class:show=move || show_copied.get()
                                                        >
                                                            "Copied!"
                                                        </div>
                                                    </div>
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
                                                    {details.pubkey.clone()}
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
                                                {format_timestamp(details.blocktime)}
                                            </div>
                                        </div>

                                        // Burn button (only show if callback is provided)
                                        {move || {
                                            if on_burn_click.is_some() {
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
                                        }}
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