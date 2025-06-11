use leptos::*;
use crate::core::session::Session;
use crate::core::pixel::Pixel;
use crate::pages::pixel_view::PixelView;
use web_sys::{HtmlInputElement, File, FileReader, Event, ProgressEvent, window, Clipboard};
use wasm_bindgen::{JsCast, closure::Closure};
use js_sys::Uint8Array;
use gloo_utils::format::JsValueSerdeExt;
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use crate::core::rpc_base::RpcConnection;
use hex;
use serde_json;

#[derive(Clone, Copy, PartialEq)]
enum MintingMode {
    Manual,
    Auto,
}

#[derive(Clone, Copy, PartialEq)]
enum GridSize {
    Size32,
    Size64,
    Size96,
}

#[component]
pub fn MintPage(
    session: RwSignal<Session>
) -> impl IntoView {
    let (minting_mode, set_minting_mode) = create_signal(MintingMode::Manual);
    let (auto_count, set_auto_count) = create_signal(0); // 0 means infinite
    let (grid_size, set_grid_size) = create_signal(GridSize::Size32);
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(32));
    let (is_minting, set_is_minting) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);
    let (minting_status, set_minting_status) = create_signal(String::new());
    
    // title and content fields
    let (title_text, set_title_text) = create_signal(String::new());
    let (content_text, set_content_text) = create_signal(String::new());

    // when the size changes, recreate the pixel art
    create_effect(move |_| {
        let size = match grid_size.get() {
            GridSize::Size32 => 32,
            GridSize::Size64 => 64,
            GridSize::Size96 => 96,
        };
        set_pixel_art.set(Pixel::new_with_size(size));
    });

    // create combined memo function
    let create_combined_memo = |title: &str, content: &str, pixel_data: &str| -> String {
        let mut memo_object = serde_json::Map::new();
        
        // add fields in specific order: title, content, image
        if !title.trim().is_empty() {
            memo_object.insert("title".to_string(), serde_json::Value::String(title.trim().to_string()));
        }
        
        if !content.trim().is_empty() {
            memo_object.insert("content".to_string(), serde_json::Value::String(content.trim().to_string()));
        }
        
        if !pixel_data.trim().is_empty() {
            memo_object.insert("image".to_string(), serde_json::Value::String(pixel_data.trim().to_string()));
        }
        
        let memo_value = serde_json::Value::Object(memo_object);
        memo_value.to_string()
    };

    // handle image import
    let handle_import = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let input: HtmlInputElement = document
            .create_element("input")
            .unwrap()
            .dyn_into()
            .unwrap();
        
        input.set_type("file");
        input.set_accept("image/*");
        
        let pixel_art_write = set_pixel_art;
        let error_signal = set_error_message;
        let current_grid_size = grid_size.get();  // get the current selected size
        
        let onchange = Closure::wrap(Box::new(move |event: Event| {
            let input: HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
            if let Some(file) = input.files().unwrap().get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();
                
                let onload = Closure::wrap(Box::new(move |_: ProgressEvent| {
                    if let Ok(buffer) = reader_clone.result() {
                        let array = Uint8Array::new(&buffer);
                        let data = array.to_vec();
                        
                        let size = match current_grid_size {
                            GridSize::Size32 => 32,
                            GridSize::Size64 => 64,
                            GridSize::Size96 => 96,
                        };
                        
                        match Pixel::from_image_data_with_size(&data, size) {
                            Ok(new_art) => {
                                pixel_art_write.set(new_art);
                                error_signal.set(String::new());
                            }
                            Err(e) => {
                                error_signal.set(format!("Failed to process image: {}", e));
                            }
                        }
                    }
                }) as Box<dyn FnMut(ProgressEvent)>);
                
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                
                reader.read_as_array_buffer(&file).unwrap();
            }
        }) as Box<dyn FnMut(_)>);
        
        input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
        onchange.forget();
        
        input.click();
    };

    // handle minting
    let handle_start_minting = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_is_minting.set(true);
        set_error_message.set(String::new());
        set_minting_status.set("Preparing to mint...".to_string());

        spawn_local(async move {
            // give UI some time to update status
            TimeoutFuture::new(100).await;

            // get all input data
            let title = title_text.get_untracked();
            let content = content_text.get_untracked();
            let pixel_data = pixel_art.get_untracked().to_optimal_string();
            
            // create combined memo JSON
            let memo_json = create_combined_memo(&title, &content, &pixel_data);
            
            // validate at least one content
            if title.trim().is_empty() && content.trim().is_empty() && pixel_data.is_empty() {
                set_error_message.set("❌ Please enter at least one field (title, content, or create pixel art)".to_string());
                set_is_minting.set(false);
                set_minting_status.set(String::new());
                return;
            }

            // validate memo length
            if memo_json.len() < 69 {
                set_error_message.set(format!("❌ Content too short. Need at least 69 characters (current: {})", memo_json.len()));
                set_is_minting.set(false);
                set_minting_status.set(String::new());
                return;
            }

            if memo_json.len() > 700 {
                set_error_message.set(format!("❌ Content too long. Maximum 700 characters (current: {})", memo_json.len()));
                set_is_minting.set(false);
                set_minting_status.set(String::new());
                return;
            }

            set_minting_status.set("Sending mint transaction...".to_string());

            // record the total_minted number before mint
            let pre_mint_total = session.with_untracked(|s| {
                s.get_user_profile()
                    .map(|profile| profile.total_minted)
                    .unwrap_or(0)
            });

            // use the new session.mint() method
            let mut session_update = session.get_untracked();
            match session_update.mint(&memo_json).await {
                Ok(signature) => {
                    log::info!("Mint transaction sent: {}", signature);
                    
                    // display the waiting status and countdown
                    for i in (1..=30).rev() {
                        set_minting_status.set(format!("Transaction confirmed! Updating data... {}s", i));
                        TimeoutFuture::new(1_000).await;
                    }
                    
                    set_minting_status.set("Finalizing...".to_string());
                    
                    // re-fetch and update user profile
                    match session_update.fetch_and_cache_user_profile().await {
                        Ok(Some(updated_profile)) => {
                            // calculate the actual minted number
                            let tokens_minted = updated_profile.total_minted.saturating_sub(pre_mint_total);
                            
                            // update the profile in session and mark balance update needed
                            session.update(|s| {
                                s.set_user_profile(Some(updated_profile.clone()));
                                s.mark_balance_update_needed(); // mark balance update needed
                            });
                            
                            set_minting_status.set("Minting completed successfully!".to_string());
                            set_error_message.set(format!(
                                "✅ Minting successful! Transaction: {} - Minted: {} tokens, Total: {}", 
                                signature, tokens_minted, updated_profile.total_minted
                            ));

                            // clear the form (optional)
                            set_title_text.set(String::new());
                            set_content_text.set(String::new());
                            set_pixel_art.set(Pixel::new_with_size(match grid_size.get_untracked() {
                                GridSize::Size32 => 32,
                                GridSize::Size64 => 64,
                                GridSize::Size96 => 96,
                            }));
                        },
                        Ok(None) => {
                            set_minting_status.set("Profile update failed".to_string());
                            set_error_message.set(format!(
                                "✅ Minting successful! Transaction: {} (Profile not found)", 
                                signature
                            ));
                            // even if profile is not found, mark balance update needed
                            session.update(|s| s.mark_balance_update_needed());
                        },
                        Err(e) => {
                            log::error!("Failed to refresh user profile after mint: {}", e);
                            set_minting_status.set("Profile update failed".to_string());
                            set_error_message.set(format!(
                                "✅ Minting successful! Transaction: {} (Profile refresh error: {})", 
                                signature, e
                            ));
                            // even if profile is not found, mark balance update needed
                            session.update(|s| s.mark_balance_update_needed());
                        }
                    }
                },
                Err(e) => {
                    set_minting_status.set("Minting failed".to_string());
                    set_error_message.set(format!("❌ Minting failed: {}", e));
                }
            }

            set_is_minting.set(false);
            set_minting_status.set(String::new());
        });
    };

    // handle copy string
    let copy_string = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();  // prevent default behavior
        ev.stop_propagation();  // prevent event propagation
        
        let art_string = pixel_art.get().to_optimal_string();
        if let Some(window) = window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&art_string);
            set_show_copied.set(true);
            
            spawn_local(async move {
                TimeoutFuture::new(3000).await;
                set_show_copied.set(false);
            });
        }
    };

    // format display string
    let format_display_string = |s: &str| {
        if s.len() <= 20 {
            s.to_string()
        } else {
            format!("{}....{}", &s[..10], &s[s.len()-10..])
        }
    };

    view! {
        <div class="mint-page">
            <h2>"Mint"</h2>
            
            // display minting progress (only show when minting)
            {move || {
                let status = minting_status.get();
                if !status.is_empty() {
                    view! {
                        <div class="minting-progress">
                            <i class="fas fa-spinner fa-spin"></i>
                            <span>{status}</span>
                        </div>
                    }
                } else {
                    view! { <div></div> }
                }
            }}

            // only show minting form when user has profile
            <Show when=move || session.get().has_user_profile()>
                <form class="mint-form" on:submit=handle_start_minting>
                    <div class="form-layout">
                        // Left side: Minting Mode, Title, Content
                        <div class="form-left">
                            // Minting Mode
                            <div class="form-group">
                                <label>"Minting Mode"</label>
                                <div class="minting-mode-group">
                                    <label class="radio-label">
                                        <input 
                                            type="radio"
                                            name="minting-mode"
                                            checked=move || minting_mode.get() == MintingMode::Manual
                                            on:change=move |_| set_minting_mode.set(MintingMode::Manual)
                                        />
                                        <span class="radio-text">"Manual"</span>
                                    </label>
                                    <label class="radio-label">
                                        <input 
                                            type="radio"
                                            name="minting-mode"
                                            checked=move || minting_mode.get() == MintingMode::Auto
                                            on:change=move |_| set_minting_mode.set(MintingMode::Auto)
                                        />
                                        <span class="radio-text">"Auto"</span>
                                    </label>
                                </div>
                            </div>

                            // number of iterations in auto mode
                            {move || {
                                if minting_mode.get() == MintingMode::Auto {
                                    view! {
                                        <div class="form-group">
                                            <label for="auto-count">"Number of Iterations (0 for infinite)"</label>
                                            <input 
                                                type="number"
                                                id="auto-count"
                                                min="0"
                                                value=auto_count
                                                on:input=move |ev| {
                                                    let input = event_target::<HtmlInputElement>(&ev);
                                                    if let Ok(count) = input.value().parse::<u32>() {
                                                        set_auto_count.set(count);
                                                    }
                                                }
                                                prop:disabled=is_minting
                                            />
                                        </div>
                                    }
                                } else {
                                    view! { <div></div> }
                                }
                            }}

                            // Title field
                            <div class="form-group">
                                <label for="title">"Title (optional):"</label>
                                <input
                                    type="text"
                                    id="title"
                                    prop:value=title_text
                                    on:input=move |ev| {
                                        let value = event_target_value(&ev);
                                        set_title_text.set(value);
                                    }
                                    placeholder="Enter title..."
                                    prop:disabled=is_minting
                                />
                            </div>

                            // Content field
                            <div class="form-group">
                                <label for="content">"Content (optional):"</label>
                                <textarea
                                    id="content"
                                    prop:value=content_text
                                    on:input=move |ev| {
                                        let value = event_target_value(&ev);
                                        set_content_text.set(value);
                                    }
                                    placeholder="Enter your content..."
                                    rows="15"
                                    prop:disabled=is_minting
                                ></textarea>
                            </div>
                        </div>

                        // Right side: Grid Size and Pixel Art
                        <div class="form-right">
                            // Grid Size selection
                            <div class="form-group">
                                <label>"Grid Size"</label>
                                <div class="grid-size-group">
                                    <label class="radio-label">
                                        <input 
                                            type="radio"
                                            name="grid-size"
                                            checked=move || grid_size.get() == GridSize::Size32
                                            on:change=move |_| set_grid_size.set(GridSize::Size32)
                                        />
                                        <span class="radio-text">"32x32"</span>
                                    </label>
                                    <label class="radio-label">
                                        <input 
                                            type="radio"
                                            name="grid-size"
                                            checked=move || grid_size.get() == GridSize::Size64
                                            on:change=move |_| set_grid_size.set(GridSize::Size64)
                                        />
                                        <span class="radio-text">"64x64"</span>
                                    </label>
                                    <label class="radio-label">
                                        <input 
                                            type="radio"
                                            name="grid-size"
                                            checked=move || grid_size.get() == GridSize::Size96
                                            on:change=move |_| set_grid_size.set(GridSize::Size96)
                                        />
                                        <span class="radio-text">"96x96"</span>
                                    </label>
                                </div>
                            </div>

                            <div class="pixel-art-editor">
                                <div class="pixel-art-header">
                                    <label>
                                        {move || {
                                            let size = match grid_size.get() {
                                                GridSize::Size32 => "32x32",
                                                GridSize::Size64 => "64x64",
                                                GridSize::Size96 => "96x96",
                                            };
                                            format!("Minting Image ({} Pixel Art)", size)
                                        }}
                                    </label>
                                    <button 
                                        type="button"
                                        class="import-btn"
                                        on:click=handle_import
                                        prop:disabled=is_minting
                                    >
                                        "Import Image"
                                    </button>
                                </div>
                                {move || {
                                    let art_string = pixel_art.get().to_optimal_string();
                                    let click_handler = Box::new(move |row, col| {
                                        let mut new_art = pixel_art.get();
                                        new_art.toggle_pixel(row, col);
                                        set_pixel_art.set(new_art);
                                    });
                                    
                                    let display_size = match grid_size.get() {
                                        GridSize::Size32 => 320,
                                        GridSize::Size64 => 400,
                                        GridSize::Size96 => 480,
                                    };
                                    
                                    view! {
                                        <PixelView
                                            art=art_string
                                            size=display_size
                                            editable=true
                                            on_click=click_handler
                                        />
                                    }
                                }}

                                // add string information display
                                <div class="pixel-string-info">
                                    <div class="string-display">
                                        <span class="label">"Encoded String: "</span>
                                        <span class="value">
                                            {move || format_display_string(&pixel_art.get().to_optimal_string())}
                                        </span>
                                        <div class="copy-container">
                                            <button
                                                type="button"
                                                class="copy-button"
                                                on:click=copy_string
                                                title="Copy encoded string to clipboard"
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
                                    <div class="string-length">
                                        <span class="label">"Length: "</span>
                                        <span class="value">
                                            {move || format!("{} bytes", pixel_art.get().to_optimal_string().len())}
                                        </span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>

                    // display the total memo length preview (full width below the two columns)
                    <div class="memo-preview">
                        <div class="memo-length">
                            <span class="label">"Total Memo Length: "</span>
                            <span class="value">
                                {move || {
                                    let title = title_text.get();
                                    let content = content_text.get();
                                    let pixel_data = pixel_art.get().to_optimal_string();
                                    let memo_json = create_combined_memo(&title, &content, &pixel_data);
                                    let len = memo_json.len();
                                    let color = if len < 69 { "red" } else if len > 700 { "red" } else { "green" };
                                    view! {
                                        <span style=format!("color: {}", color)>
                                            {format!("{}/700 characters (minimum 69)", len)}
                                        </span>
                                    }
                                }}
                            </span>
                        </div>
                    </div>

                    {move || {
                        let message = error_message.get();
                        view! {
                            <div class="error-message" 
                                class:success=message.contains("✅")
                                class:error=message.contains("❌")
                                class:warning=message.contains("⚠️")
                                style:display={if message.is_empty() { "none" } else { "block" }}
                            >
                                {message}
                            </div>
                        }
                    }}

                    <div class="button-group">
                        <button
                            type="submit"
                            class="start-minting-btn"
                            prop:disabled=move || {
                                is_minting.get() || 
                                !session.get().has_user_profile() ||
                                {
                                    let title = title_text.get();
                                    let content = content_text.get();
                                    let pixel_data = pixel_art.get().to_optimal_string();
                                    let memo_json = create_combined_memo(&title, &content, &pixel_data);
                                    memo_json.len() < 69 || memo_json.len() > 700
                                }
                            }
                        >
                            {move || if is_minting.get() { "Minting..." } else { "Start Minting" }}
                        </button>
                    </div>
                </form>
            </Show>

            // show warning when no profile
            <Show when=move || !session.get().has_user_profile()>
                <div class="no-profile-message">
                    <h3>"Profile Required"</h3>
                    <p>"Please create your mint profile in the Profile page before you can start minting."</p>
                </div>
            </Show>
        </div>
    }
}
