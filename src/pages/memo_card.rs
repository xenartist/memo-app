use leptos::*;
use crate::pages::pixel_view::PixelView;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn MemoCard(
    #[prop(optional)] title: Option<String>,
    #[prop(optional)] image: Option<String>,
    signature: String,
    pubkey: String,
    blocktime: i64,
    #[prop(optional)] amount: Option<f64>,
    #[prop(optional)] class: Option<&'static str>,
    #[prop(optional)] on_details_click: Option<Callback<String>>,
    #[prop(optional)] on_burn_click: Option<Callback<String>>,
) -> impl IntoView {
    let class_str = class.unwrap_or("");
    
    // lazy loading state
    let (is_visible, set_is_visible) = create_signal(false);
    
    // simplified visibility detection - use lazy loading instead of Intersection Observer
    let card_ref = create_node_ref::<leptos::html::Div>();
    
    // delay a bit and automatically set to visible (simplified solution)
    create_effect(move |_| {
        spawn_local(async move {
            // short delay and set to visible, simulate lazy loading
            TimeoutFuture::new(100).await;
            set_is_visible.set(true);
        });
    });

    // format timestamp
    let format_timestamp = move |timestamp: i64| -> String {
        let date = js_sys::Date::new(&(timestamp as f64 * 1000.0).into());
        date.to_locale_string("en-US", &js_sys::Object::new())
            .as_string()
            .unwrap_or_else(|| "Unknown".to_string())
    };

    view! {
        <div class=format!("memo-card {}", class_str) node_ref=card_ref>
            // title area
            <div class="memo-header">
                {move || {
                    if let Some(ref title_text) = title {
                        view! {
                            <h4 class="memo-title">{title_text.clone()}</h4>
                        }
                    } else {
                        view! {
                            <h4 class="memo-title">"Memory"</h4>
                        }
                    }
                }}
            </div>

            // image area with lazy loading
            <div class="memo-image-container">
                {move || {
                    if !is_visible.get() {
                        // show placeholder until delay time passes
                        view! {
                            <div class="memo-image-placeholder">
                                <i class="fas fa-image"></i>
                                <span>"Loading..."</span>
                            </div>
                        }.into_view()
                    } else if let Some(ref image_data) = image {
                        // handle image after delay
                        if image_data.starts_with("http") || image_data.starts_with("data:") {
                            // normal image URL
                            view! {
                                <img 
                                    src={image_data.clone()}
                                    alt="Memory Image"
                                    class="memo-image"
                                />
                            }.into_view()
                        } else {
                            // pixel art encoded - use lazy loading PixelView
                            view! {
                                <LazyPixelView
                                    art={image_data.clone()}
                                    size=128
                                />
                            }.into_view()
                        }
                    } else {
                        // no image, show placeholder
                        view! {
                            <div class="memo-image-placeholder">
                                <i class="fas fa-image"></i>
                                <span>"No Image"</span>
                            </div>
                        }.into_view()
                    }
                }}
            </div>
            
            // info area
            <div class="memo-info">
                <div class="memo-info-item">
                    <span class="label">"Signature:"</span>
                    <span class="value signature">{signature.clone()}</span>
                </div>
                
                <div class="memo-info-item">
                    <span class="label">"From:"</span>
                    <span class="value pubkey">{pubkey}</span>
                </div>
                
                <div class="memo-info-item">
                    <span class="label">"Time:"</span>
                    <span class="value blocktime">{format_timestamp(blocktime)}</span>
                </div>
                
                // only show amount when it exists
                {move || {
                    if let Some(amount_value) = amount {
                        view! {
                            <div class="memo-info-item">
                                <span class="label">"Amount:"</span>
                                <span class="value amount">{format!("{:.2} tokens", amount_value)}</span>
                            </div>
                        }
                    } else {
                        view! { <div></div> }
                    }
                }}
            </div>

            // hover action buttons - only show when callbacks are provided
            {move || {
                let details_callback = on_details_click.clone();
                let burn_callback = on_burn_click.clone();
                
                if details_callback.is_some() || burn_callback.is_some() {
                    view! {
                        <div class="memo-card-actions">
                            // Details button
                            {
                                let details_cb = details_callback.clone();
                                let sig_for_details = signature.clone();
                                move || {
                                    if let Some(details_cb) = details_cb.clone() {
                                        let sig_clone = sig_for_details.clone();
                                        view! {
                                            <button 
                                                class="action-btn details-btn"
                                                on:click=move |e| {
                                                    e.stop_propagation();
                                                    details_cb.call(sig_clone.clone());
                                                }
                                            >
                                                <i class="fas fa-info-circle"></i>
                                                <span>"Details"</span>
                                            </button>
                                        }.into_view()
                                    } else {
                                        view! { <div style="display: none;"></div> }.into_view()
                                    }
                                }
                            }
                            
                            // Burn button
                            {
                                let burn_cb = burn_callback.clone();
                                let sig_for_burn = signature.clone();
                                move || {
                                    if let Some(burn_cb) = burn_cb.clone() {
                                        let sig_clone = sig_for_burn.clone();
                                        view! {
                                            <button 
                                                class="action-btn burn-btn"
                                                on:click=move |e| {
                                                    e.stop_propagation();
                                                    burn_cb.call(sig_clone.clone());
                                                }
                                            >
                                                <i class="fas fa-fire"></i>
                                                <span>"Burn"</span>
                                            </button>
                                        }.into_view()
                                    } else {
                                        view! { <div style="display: none;"></div> }.into_view()
                                    }
                                }
                            }
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}
        </div>
    }
}

// lazy loading PixelView component
#[component]
pub fn LazyPixelView(
    art: String,
    size: u32,
) -> impl IntoView {
    let (is_loaded, set_is_loaded) = create_signal(false);
    
    // use signal to store art string, avoid moving issues
    let (art_signal, _) = create_signal(art);
    
    // async decode, add delay to avoid blocking UI
    create_effect(move |_| {
        spawn_local(async move {
            // add delay, give UI time to render placeholder
            TimeoutFuture::new(200).await;
            set_is_loaded.set(true);
        });
    });
    
    view! {
        {move || {
            if is_loaded.get() {
                view! {
                    <PixelView
                        art={art_signal.get()}
                        size=size
                        editable=false
                    />
                }.into_view()
            } else {
                view! {
                    <div class="pixel-loading" style="display: flex; align-items: center; justify-content: center; height: 128px; color: #666; background-color: #f8f9fa; border-radius: 6px;">
                        <i class="fas fa-spinner fa-spin" style="margin-right: 8px;"></i>
                        <span>"Decoding..."</span>
                    </div>
                }.into_view()
            }
        }}
    }
}
