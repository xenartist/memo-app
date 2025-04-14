use leptos::*;
use crate::core::session::Session;
use crate::core::pixel::Pixel;
use crate::pages::pixel_view::PixelView;
use web_sys::{HtmlInputElement, File, FileReader, Event, ProgressEvent};
use wasm_bindgen::{JsCast, closure::Closure};
use js_sys::Uint8Array;

#[derive(Clone, Copy, PartialEq)]
enum MiningMode {
    Manual,
    Auto,
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
    let (pixel_art, set_pixel_art) = create_signal(Pixel::new_with_size(64));
    let (is_mining, set_is_mining) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());

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
        
        let onchange = Closure::wrap(Box::new(move |event: Event| {
            let input: HtmlInputElement = event.target().unwrap().dyn_into().unwrap();
            if let Some(file) = input.files().unwrap().get(0) {
                let reader = FileReader::new().unwrap();
                let reader_clone = reader.clone();
                
                let onload = Closure::wrap(Box::new(move |_: ProgressEvent| {
                    if let Ok(buffer) = reader_clone.result() {
                        let array = Uint8Array::new(&buffer);
                        let data = array.to_vec();
                        
                        match Pixel::from_image_data_with_size(&data, 64) {
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
        
        // TODO: 实现挖矿逻辑
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
                    <div class="radio-group">
                        <label>
                            <input 
                                type="radio"
                                name="mining-mode"
                                checked=move || mining_mode.get() == MiningMode::Manual
                                on:change=move |_| set_mining_mode.set(MiningMode::Manual)
                            />
                            "Manual"
                        </label>
                        <label>
                            <input 
                                type="radio"
                                name="mining-mode"
                                checked=move || mining_mode.get() == MiningMode::Auto
                                on:change=move |_| set_mining_mode.set(MiningMode::Auto)
                            />
                            "Auto"
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

                <div class="pixel-art-editor">
                    <div class="pixel-art-header">
                        <label>"Mining Image (64x64 Pixel Art)"</label>
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
                        
                        view! {
                            <PixelView
                                art=art_string
                                size=512
                                editable=true
                                on_click=click_handler
                            />
                        }
                    }}
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
