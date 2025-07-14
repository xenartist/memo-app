use leptos::*;
use leptos::leptos_dom::ev::SubmitEvent;
use crate::core::session::Session;
use crate::core::pixel::Pixel;
use crate::core::storage_mint::get_mint_storage;
use crate::pages::pixel_view::PixelView;
use web_sys::{HtmlInputElement, File, FileReader, Event, ProgressEvent, window};
use wasm_bindgen::{JsCast, closure::Closure};
use js_sys::Uint8Array;
use gloo_utils::format::JsValueSerdeExt;
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;
use hex;
use serde_json;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq)]
pub enum MintingMode {
    Manual,
    Auto,
}

#[derive(Clone, Copy, PartialEq)]
pub enum GridSize {
    Size8,
    Size16,
    Size32,
    Size64,
    Size96,
    Size128,
    Size256,
    Size512,
    Size1024,
}

impl GridSize {
    pub fn to_size(self) -> usize {
        match self {
            GridSize::Size8 => 8,
            GridSize::Size16 => 16,
            GridSize::Size32 => 32,
            GridSize::Size64 => 64,
            GridSize::Size96 => 96,
            GridSize::Size128 => 128,
            GridSize::Size256 => 256,
            GridSize::Size512 => 512,
            GridSize::Size1024 => 1024,
        }
    }

    pub fn to_display_string(self) -> &'static str {
        match self {
            GridSize::Size8 => "8x8",
            GridSize::Size16 => "16x16",
            GridSize::Size32 => "32x32",
            GridSize::Size64 => "64x64",
            GridSize::Size96 => "96x96",
            GridSize::Size128 => "128x128",
            GridSize::Size256 => "256x256",
            GridSize::Size512 => "512x512",
            GridSize::Size1024 => "1024x1024",
        }
    }

    pub fn all_sizes() -> Vec<Self> {
        vec![
            GridSize::Size8,
            GridSize::Size16,
            GridSize::Size32,
            GridSize::Size64,
            GridSize::Size96,
            GridSize::Size128,
            GridSize::Size256,
            GridSize::Size512,
            GridSize::Size1024,
        ]
    }
}

#[component]
pub fn MintForm(
    session: RwSignal<Session>,
    #[prop(optional)] class: Option<&'static str>,
    #[prop(optional)] on_mint_success: Option<Rc<dyn Fn(String, u64, u64)>>,
    #[prop(optional)] on_mint_error: Option<Rc<dyn Fn(String)>>,
    #[prop(optional)] on_close: Option<Rc<dyn Fn()>>,
) -> impl IntoView {
    let class_str = class.unwrap_or("");
    
    // --- Wrap non-Copy props in signals ---
    // This is the key to solving the ownership problem.
    // RwSignal is Copy, so it can be captured by closures without moving.
    let on_close_signal = create_rw_signal(on_close);
    let on_mint_success_signal = create_rw_signal(on_mint_success);
    let on_mint_error_signal = create_rw_signal(on_mint_error);

    // 所有表单相关信号
    let (minting_mode, set_minting_mode) = create_signal(MintingMode::Manual);
    let (auto_count, set_auto_count) = create_signal(0); // 0 means infinite
    let (grid_size, set_grid_size) = create_signal(GridSize::Size32);
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(32));
    let (is_minting, set_is_minting) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);
    let (minting_status, set_minting_status) = create_signal(String::new());
    let (title_text, set_title_text) = create_signal(String::new());
    let (content_text, set_content_text) = create_signal(String::new());
    
    // Auto minting related signals
    let (is_auto_minting, set_is_auto_minting) = create_signal(false);
    let (auto_progress, set_auto_progress) = create_signal((0u32, 0u32)); // (current, total)
    let (auto_should_stop, set_auto_should_stop) = create_signal(false);
    let (auto_success_count, set_auto_success_count) = create_signal(0u32);
    let (auto_error_count, set_auto_error_count) = create_signal(0u32);

    // Success countdown related signals
    let (success_countdown, set_success_countdown) = create_signal(0u32);
    let (is_success_countdown_active, set_is_success_countdown_active) = create_signal(false);
    
    // Re-add the detailed minting status signal
    let (minting_status, set_minting_status) = create_signal(String::new());

    // --- NEW: Manual signal to control immediate UI state on submit ---
    let (is_submitting, set_is_submitting) = create_signal(false);

    // when the size changes, recreate the pixel art
    create_effect(move |_| {
        let size = grid_size.get().to_size();
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
    
    // --- Reusable Core Minting Logic ---
    let perform_one_mint = {
        let session = session;
        move |memo_json: String| {
            let session = session.clone();
            async move {
                let mut session_update = session.with_untracked(|s| s.clone());
                match session_update.mint(&memo_json).await {
                    Ok(signature) => {
                        log::info!("Mint transaction confirmed: {}", signature);
                        let sig_clone = signature.clone();
                        let memo_clone = memo_json.clone();
                        spawn_local(async move {
                            if let Err(e) = get_mint_storage().save_mint_record_async(&sig_clone, &memo_clone).await {
                                log::error!("Failed to save mint record: {}", e);
                            }
                        });

                        match session_update.fetch_and_cache_user_profile().await {
                            Ok(Some(profile)) => {
                                session.update(|s| {
                                    s.set_user_profile(Some(profile.clone()));
                                    s.mark_balance_update_needed();
                                });
                                Ok((signature, 1u64, profile.total_minted))
                            },
                            _ => {
                                session.update(|s| s.mark_balance_update_needed());
                                Ok((signature, 1u64, 0u64))
                            }
                        }
                    },
                    Err(e) => {
                        Err(format!("Minting failed: {}", e))
                    }
                }
            }
        }
    };
    
    // --- Action for a Single Mint ---
    let single_mint_action = create_action(move |memo_json: &String| {
        perform_one_mint(memo_json.clone())
    });

    // --- Action for Auto Minting Loop ---
    let auto_mint_action = create_action(move |(memo_json, count): &(String, u32)| {
        let memo_json = memo_json.clone();
        let count = *count;
        let perform_one_mint_clone = perform_one_mint.clone();

        let on_success_cb = on_mint_success_signal.get_untracked();
        let on_error_cb = on_mint_error_signal.get_untracked();

        async move {
            set_is_auto_minting.set(true);
            set_auto_should_stop.set(false);
            set_auto_success_count.set(0);
            set_auto_error_count.set(0);

            let total_rounds = if count == 0 { 0 } else { count };
            set_auto_progress.set((0, total_rounds));

            let mut current_round = 0u32;
            let mut successes = 0u32;
            let mut errors = 0u32;

            while (count == 0 || current_round < count) && !auto_should_stop.get_untracked() {
                current_round += 1;
                set_auto_progress.set((current_round, total_rounds));
                set_minting_status.set(format!("Auto Minting: Round {}/{}...", current_round, if count == 0 { "∞".to_string() } else { count.to_string() }));
                
                match perform_one_mint_clone(memo_json.clone()).await {
                    Ok((sig, tokens, total)) => {
                        successes += 1;
                        set_auto_success_count.set(successes);
                        if let Some(cb) = on_success_cb.as_ref() { cb(sig, tokens, total); }
                    },
                    Err(e) => {
                        errors += 1;
                        set_auto_error_count.set(errors);
                        if let Some(cb) = on_error_cb.as_ref() { cb(e); }
                    }
                }

                if auto_should_stop.get_untracked() {
                    set_minting_status.set("Stopping auto minting...".to_string());
                    break;
                }
                if count == 0 || current_round < count {
                    set_minting_status.set(format!("Waiting for next mint... (Success: {}, Errors: {})", successes, errors));
                    TimeoutFuture::new(30_000).await;
                }
            }
            
            set_is_auto_minting.set(false);
            set_minting_status.set(String::new()); // Clear status when done
            (current_round, successes, errors)
        }
    });

    // --- Effect for starting countdown timer and closing form ---
    let start_countdown_to_close = {
        move |message: String| {
            set_error_message.set(message);
            set_success_countdown.set(10);
            set_is_success_countdown_active.set(true);

            spawn_local(async move {
                for i in (1..=10).rev() {
                    set_success_countdown.set(i);
                    if !is_success_countdown_active.get_untracked() { return; }
                    TimeoutFuture::new(1000).await;
                    if !is_success_countdown_active.get_untracked() { return; }
                }
                
                if is_success_countdown_active.get_untracked() {
                    set_is_success_countdown_active.set(false);
                    on_close_signal.with_untracked(|cb_opt| {
                        if let Some(callback) = cb_opt.as_ref() {
                            callback();
                        }
                    });
                }
            });
        }
    };
    
    // --- Effect for Single Mint Action Result ---
    create_effect({
        let start_countdown_to_close = start_countdown_to_close.clone();
        move |_| {
            if let Some(result) = single_mint_action.value().get() {
                set_is_submitting.set(false); // <-- Reset on completion
                set_minting_status.set(String::new()); // Clear status when done
                match result {
                    Ok((signature, tokens_minted, total_minted)) => {
                        on_mint_success_signal.with_untracked(|cb_opt| {
                            if let Some(cb) = cb_opt.as_ref() {
                                cb(signature.clone(), tokens_minted, total_minted);
                            }
                        });

                        let success_message = format!(
                            "✅ Minting successful! Transaction: {} - Minted: {} tokens, Total: {}", 
                            signature, tokens_minted, total_minted
                        );
                        start_countdown_to_close(success_message);
                        set_title_text.set(String::new());
                        set_content_text.set(String::new());
                        set_pixel_art.set(Pixel::new_with_size(grid_size.get_untracked().to_size()));
                    },
                    Err(e) => {
                        on_mint_error_signal.with_untracked(|cb_opt| {
                             if let Some(cb) = cb_opt.as_ref() {
                                cb(e.clone());
                            }
                        });
                        set_error_message.set(format!("❌ {}", e));
                    }
                }
            }
        }
    });

    // --- Effect for Auto Mint Action Result ---
    create_effect({
        let start_countdown_to_close = start_countdown_to_close.clone();
        move |_| {
            if let Some((rounds, successes, errors)) = auto_mint_action.value().get() {
                set_is_submitting.set(false); // <-- Reset on completion
                set_minting_status.set(String::new()); // Clear status when done
                let message = if errors == 0 && successes > 0 {
                    format!("✅ Auto minting finished: {} rounds completed, {} successful, {} failed", rounds, successes, errors)
                } else if successes == 0 && errors > 0 {
                    format!("❌ Auto minting finished: {} rounds completed, {} successful, {} failed", rounds, successes, errors)
                } else if successes > 0 && errors > 0 {
                    format!("⚠️ Auto minting finished: {} rounds completed, {} successful, {} failed", rounds, successes, errors)
                } else {
                    format!("🎯 Auto minting finished: {} rounds completed, {} successful, {} failed", rounds, successes, errors)
                };
                start_countdown_to_close(message);
            }
        }
    });

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
                        
                        let size = current_grid_size.to_size();
                        
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

    // handle stop auto minting
    let handle_stop_auto = move |_: web_sys::MouseEvent| {
        set_auto_should_stop.set(true);
        set_minting_status.set("Stopping auto minting...".to_string());
    };

    // --- We will NOT create an external handler. All logic will be inline. ---

    view! {
        <div class=format!("mint-form-component {}", class_str)>
            // only show minting form when user has profile
            <Show when=move || session.get().has_user_profile()>
                <form class="mint-form" on:submit=move |ev: SubmitEvent| {
                    ev.prevent_default();

                    // --- Imitating the successful pattern from burn_onchain.rs ---

                    // 1. Get data and update UI state immediately (synchronous part)
                    let mode = minting_mode.get_untracked();
                    let is_pending = match mode {
                        MintingMode::Manual => single_mint_action.pending().get(),
                        MintingMode::Auto => auto_mint_action.pending().get(),
                    };
                    if is_pending { return; }

                    let title = title_text.get_untracked();
                    let content = content_text.get_untracked();
                    let pixel_data = pixel_art.get_untracked().to_optimal_string();
                    let memo_json = create_combined_memo(&title, &content, &pixel_data);

                    // Validation
                    if title.trim().is_empty() && content.trim().is_empty() && pixel_data.is_empty() {
                        set_error_message.set("❌ Please enter at least one field (title, content, or create pixel art)".to_string());
                        return;
                    }
                    if memo_json.len() < 69 || memo_json.len() > 700 {
                        set_error_message.set(format!("❌ Invalid content length: {}. Must be between 69 and 700.", memo_json.len()));
                        return;
                    }

                    // This is the key: set status *before* spawning the async block.
                    set_is_submitting.set(true); // <-- Set manual pending state immediately
                    set_minting_status.set("Preparing to mint...".to_string());
                    set_error_message.set(String::new());


                    // 2. Spawn the async logic which includes the delay.
                    spawn_local(async move {
                        // Delay is inside the async block.
                        TimeoutFuture::new(100).await;
                        
                        // Dispatch the correct action based on the mode captured earlier.
                        match mode {
                            MintingMode::Manual => {
                                single_mint_action.dispatch(memo_json);
                            },
                            MintingMode::Auto => {
                                let count = auto_count.get_untracked();
                                auto_mint_action.dispatch((memo_json, count));
                            }
                        }
                    });
                }>
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
                                            prop:disabled=move || single_mint_action.pending().get() || auto_mint_action.pending().get()
                                        />
                                        <span class="radio-text">"Manual"</span>
                                    </label>
                                    <label class="radio-label">
                                        <input 
                                            type="radio"
                                            name="minting-mode"
                                            checked=move || minting_mode.get() == MintingMode::Auto
                                            on:change=move |_| set_minting_mode.set(MintingMode::Auto)
                                            prop:disabled=move || single_mint_action.pending().get() || auto_mint_action.pending().get()
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
                                                prop:disabled=move || single_mint_action.pending().get() || auto_mint_action.pending().get()
                                            />
                                            <div class="auto-mode-info">
                                                <small>"Auto mode will mint repeatedly with 30-second intervals between transactions"</small>
                                            </div>
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
                                    prop:disabled=move || single_mint_action.pending().get() || auto_mint_action.pending().get()
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
                                    prop:disabled=move || single_mint_action.pending().get() || auto_mint_action.pending().get()
                                ></textarea>
                            </div>
                        </div>

                        // Right side: Grid Size and Pixel Art
                        <div class="form-right">
                            // Grid Size selection
                            <div class="form-group">
                                <label>"Grid Size"</label>
                                <div class="grid-size-group">
                                    {GridSize::all_sizes().into_iter().map(|size| {
                                        view! {
                                            <label class="radio-label">
                                                <input 
                                                    type="radio"
                                                    name="grid-size"
                                                    checked=move || grid_size.get() == size
                                                    on:change=move |_| set_grid_size.set(size)
                                                    prop:disabled=move || single_mint_action.pending().get() || auto_mint_action.pending().get()
                                                />
                                                <span class="radio-text">{size.to_display_string()}</span>
                                            </label>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>

                            <div class="pixel-art-editor">
                                <div class="pixel-art-header">
                                    <label>
                                        {move || {
                                            let size = grid_size.get().to_display_string();
                                            format!("Image ({} pixels)", size)
                                        }}
                                    </label>
                                    <button 
                                        type="button"
                                        class="import-btn"
                                        on:click=handle_import
                                        prop:disabled=move || single_mint_action.pending().get() || auto_mint_action.pending().get()
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
                                    
                                    // fixed display area size to 320px, not change with grid_size
                                    let display_size = 320;
                                    
                                    view! {
                                        <PixelView
                                            art=art_string
                                            size=display_size
                                            editable=true
                                            show_grid=true
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
                                <div class="message-content">
                                    {message}
                                </div>
                                
                                // Show countdown and close button when success countdown is active
                                {move || {
                                    if is_success_countdown_active.get() {
                                        let countdown = success_countdown.get();
                                        view! {
                                            <div class="success-countdown">
                                                <span class="countdown-text">
                                                    {format!("Auto close countdown: {} seconds", countdown)}
                                                </span>
                                                <button
                                                    type="button"
                                                    class="close-success-btn"
                                                    on:click=move |_| {
                                                        set_is_success_countdown_active.set(false);
                                                        // Access the on_close function through the signal
                                                        on_close_signal.with_untracked(|cb_opt| {
                                                            if let Some(callback) = cb_opt.as_ref() {
                                                                callback();
                                                            } else {
                                                                // Fallback if no on_close is provided
                                                                set_error_message.set(String::new());
                                                            }
                                                        });
                                                    }
                                                >
                                                    "Close"
                                                </button>
                                            </div>
                                        }
                                    } else {
                                        view! { <div></div> }
                                    }
                                }}
                            </div>
                        }
                    }}

                    <div class="button-group">
                        <button
                            type="submit"
                            class="start-minting-btn"
                            prop:disabled=move || {
                                // Now also depends on the manual signal
                                let is_pending = single_mint_action.pending().get() || auto_mint_action.pending().get() || is_submitting.get();
                                is_pending ||
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
                            {move || {
                                // Now also depends on the manual signal
                                let is_pending = single_mint_action.pending().get() || auto_mint_action.pending().get() || is_submitting.get();
                                if is_pending {
                                    "Minting...".to_string()
                                } else {
                                    match minting_mode.get() {
                                        MintingMode::Manual => "Start Minting".to_string(),
                                        MintingMode::Auto => "Start Auto Minting".to_string(),
                                    }
                                }
                            }}
                        </button>
                    </div>
                </form>

                // display minting progress (RESTORED)
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

                // Auto minting progress and controls (below minting progress)
                {move || {
                    if is_auto_minting.get() {
                        let (current, total) = auto_progress.get();
                        let success = auto_success_count.get();
                        let errors = auto_error_count.get();
                        
                        view! {
                            <div class="auto-minting-controls">
                                <div class="auto-progress">
                                    <span class="progress-text">
                                        {if total == 0 {
                                            format!("Round: {} | Success: {} | Errors: {}", current, success, errors)
                                        } else {
                                            format!("Progress: {}/{} | Success: {} | Errors: {}", current, total, success, errors)
                                        }}
                                    </span>
                                    {if total > 0 {
                                        let percentage = if total > 0 { (current as f32 / total as f32 * 100.0) } else { 0.0 };
                                        view! {
                                            <div class="progress-bar">
                                                <div class="progress-fill" style=format!("width: {}%", percentage)></div>
                                            </div>
                                        }
                                    } else {
                                        view! { <div></div> }
                                    }}
                                </div>
                                <button
                                    type="button"
                                    class="stop-auto-btn"
                                    on:click=handle_stop_auto
                                >
                                    "Stop Auto Minting"
                                </button>
                            </div>
                        }
                    } else {
                        view! { <div></div> }
                    }
                }}
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