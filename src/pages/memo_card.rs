use leptos::*;
use crate::pages::pixel_view::PixelView;

#[component]
pub fn MemoCard(
    #[prop(optional)] title: Option<String>,         // title (optional)
    #[prop(optional)] image: Option<String>,         // image data (optional - can be pixel art encoded or image URL)
    signature: String,                               // signature (required)
    pubkey: String,                                  // pubkey (required) 
    blocktime: i64,                                  // blocktime (required)
    #[prop(optional)] amount: Option<f64>,           // amount (optional - only when burn)
    #[prop(optional)] class: Option<&'static str>,   // optional CSS class
) -> impl IntoView {
    let class_str = class.unwrap_or("");
    
    // format timestamp
    let format_timestamp = move |timestamp: i64| -> String {
        let date = js_sys::Date::new(&(timestamp as f64 * 1000.0).into());
        date.to_locale_string("en-US", &js_sys::Object::new())
            .as_string()
            .unwrap_or_else(|| "Unknown".to_string())
    };

    view! {
        <div class=format!("memo-card {}", class_str)>
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

            // image area
            <div class="memo-image-container">
                {move || {
                    if let Some(ref image_data) = image {
                        // check if it's pixel art or normal image URL
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
                            // pixel art encoded
                            view! {
                                <PixelView
                                    art={image_data.clone()}
                                    size=128
                                    editable=false
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
                    <span class="value signature">{signature}</span>
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
        </div>
    }
}
