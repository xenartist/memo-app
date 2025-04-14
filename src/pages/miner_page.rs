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

#[derive(Clone, Copy, PartialEq)]
enum MiningMode {
    Manual,
    Auto,
}

#[derive(Clone, Copy, PartialEq)]
enum GridSize {
    Size64,
    Size96,
}

#[component]
pub fn MinerPage(
    session: RwSignal<Session>
) -> impl IntoView {
    let wallet_address = move || {
        match session.get().get_public_key() {
            Ok(addr) => addr,
            Err(_) => "Not initialized".to_string()
        }
    };

    let (mining_mode, set_mining_mode) = create_signal(MiningMode::Manual);
    let (auto_count, set_auto_count) = create_signal(0); // 0 means infinite
    let (grid_size, set_grid_size) = create_signal(GridSize::Size64);
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(64));
    let (is_mining, set_is_mining) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());
    let (show_copied, set_show_copied) = create_signal(false);

    // when the size changes, recreate the pixel art
    create_effect(move |_| {
        let size = match grid_size.get() {
            GridSize::Size64 => 64,
            GridSize::Size96 => 96,
        };
        set_pixel_art.set(Pixel::new_with_size(size));
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
                        
                        let size = match current_grid_size {
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

    // handle mining
    let handle_start_mining = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_is_mining.set(true);
        
        // TODO: mining logic
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
        <div class="miner-page">
            <h2>"Miner"</h2>
            
            <div class="miner-content">
                <div class="miner-status">
                    <h3>"Mining Status"</h3>
                    <div class="status-info">
                        <p>"Wallet: " {wallet_address}</p>
                        // Add more mining status information here
                    </div>
                </div>

                <div class="mining-controls">
                    <h3>"Controls"</h3>
                    // Add mining control buttons and options here
                </div>

                <div class="mining-stats">
                    <h3>"Statistics"</h3>
                    // Add mining statistics here
                </div>
            </div>

            <form class="miner-form" on:submit=handle_start_mining>
                <div class="form-group">
                    <label>"Mining Mode"</label>
                    <div class="mining-mode-group">
                        <label class="radio-label">
                            <input 
                                type="radio"
                                name="mining-mode"
                                checked=move || mining_mode.get() == MiningMode::Manual
                                on:change=move |_| set_mining_mode.set(MiningMode::Manual)
                            />
                            <span class="radio-text">"Manual"</span>
                        </label>
                        <label class="radio-label">
                            <input 
                                type="radio"
                                name="mining-mode"
                                checked=move || mining_mode.get() == MiningMode::Auto
                                on:change=move |_| set_mining_mode.set(MiningMode::Auto)
                            />
                            <span class="radio-text">"Auto"</span>
                        </label>
                    </div>
                </div>

                // number of iterations in auto mode
                {move || {
                    if mining_mode.get() == MiningMode::Auto {
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
                                    prop:disabled=is_mining
                                />
                            </div>
                        }
                    } else {
                        view! { <div></div> }
                    }
                }}

                // add size selection
                <div class="form-group">
                    <label>"Grid Size"</label>
                    <div class="grid-size-group">
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
                                    GridSize::Size64 => "64x64",
                                    GridSize::Size96 => "96x96",
                                };
                                format!("Mining Image ({} Pixel Art)", size)
                            }}
                        </label>
                        <button 
                            type="button"
                            class="import-btn"
                            on:click=handle_import
                            prop:disabled=is_mining
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
                            GridSize::Size64 => 512,
                            GridSize::Size96 => 768,  // larger display size to fit more pixels
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

                {move || {
                    let message = error_message.get();
                    view! {
                        <div class="error-message" 
                            class:success=message.contains("success")
                            style:display={if message.is_empty() { "none" } else { "block" }}
                        >
                            {message}
                        </div>
                    }
                }}

                <div class="button-group">
                    <button
                        type="submit"
                        class="start-mining-btn"
                        prop:disabled=is_mining
                    >
                        {move || if is_mining.get() { "Mining..." } else { "Start Mining" }}
                    </button>
                </div>
            </form>
        </div>
    }
}
