use dioxus::prelude::*;

/// Modal dialog component for displaying wallet mnemonic
#[component]
pub fn MnemonicModal(
    mnemonic: String,
    visible: bool,
    on_close: EventHandler<()>,
) -> Element {
    if !visible {
        return rsx!{ Fragment {} };
    }

    // Split the mnemonic into words and add index
    let words: Vec<(usize, &str)> = mnemonic
        .split_whitespace()
        .enumerate()
        .map(|(i, word)| (i + 1, word))
        .collect();

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal-content",
                // Prevent clicks inside the modal from closing it
                onclick: move |evt| evt.stop_propagation(),
                h2 { "Your Recovery Phrase" }
                p { class: "warning", "Write these words down and keep them in a safe place. They are the only way to recover your wallet!" }
                
                div {
                    class: "mnemonic-grid",
                    for (index, word) in words {
                        div {
                            class: "mnemonic-word",
                            span { class: "word-number", "{index}." }
                            span { class: "word-text", "{word}" }
                        }
                    }
                }
                
                div {
                    class: "modal-actions",
                    button {
                        class: "modal-button",
                        onclick: move |_| {
                            log::info!("Save Wallet button clicked in modal");
                            on_close.call(())
                        },
                        "Save Wallet"
                    }
                }
            }
        }
    }
}

/// Wallet address display component for the header
#[component]
pub fn WalletAddressDisplay(address: String) -> Element {
    // State for copy success animation
    let mut copy_success = use_signal(|| false);
    // State to show full address temporarily
    let mut show_full_address = use_signal(|| false);
    
    // Format the address based on display state
    let display_address = if *show_full_address.read() {
        address.clone()
    } else if address.len() > 6 {
        format!("{}...{}", &address[..6], &address[address.len()-4..])
    } else {
        address.clone()
    };
    
    // Function to copy address to clipboard
    let copy_to_clipboard = {
        // Clone address to avoid ownership issues
        let address_clone = address.clone();
        
        move |_| {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::prelude::*;
                use js_sys::Reflect;
                use wasm_bindgen::JsCast;
                
                // Log the full address we're trying to copy
                log::info!("Attempting to copy full address: {}", address_clone);
                
                // Try to copy to clipboard using the Clipboard API
                if let Some(window) = web_sys::window() {
                    let navigator = match Reflect::get(&window, &JsValue::from_str("navigator")) {
                        Ok(nav) => nav,
                        Err(_) => {
                            log::error!("Failed to get navigator object");
                            return;
                        }
                    };
                    
                    let clipboard = match Reflect::get(&navigator, &JsValue::from_str("clipboard")) {
                        Ok(clip) => clip,
                        Err(_) => {
                            log::error!("Clipboard API not available");
                            return;
                        }
                    };
                    
                    // Call the writeText method on clipboard
                    let promise = match Reflect::get(&clipboard, &JsValue::from_str("writeText")) {
                        Ok(write_fn) => {
                            let write_fn = write_fn.dyn_into::<js_sys::Function>().unwrap_or_else(|_| {
                                log::error!("writeText is not a function");
                                return js_sys::Function::new_no_args("return null");
                            });
                            
                            let result = Reflect::apply(
                                &write_fn,
                                &clipboard,
                                &js_sys::Array::of1(&JsValue::from_str(&address_clone))
                            );
                            
                            match result {
                                Ok(promise) => promise,
                                Err(e) => {
                                    log::error!("Failed to call writeText: {:?}", e);
                                    return;
                                }
                            }
                        },
                        Err(_) => {
                            log::error!("writeText method not found on clipboard");
                            return;
                        }
                    };
                    
                    // Show success animation regardless of promise result
                    // In a production app, you'd want to wait for the promise to resolve
                    copy_success.set(true);
                    log::info!("Copied address to clipboard: {}", address_clone);
                    
                    // Reset after animation
                    let mut success = copy_success.clone();
                    let window_clone = window.clone();
                    let closure = Closure::once(move || {
                        success.set(false);
                        log::info!("Reset copy success animation");
                    });
                    
                    let _ = window_clone.set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        2000
                    );
                    closure.forget();
                }
            }
            
            #[cfg(not(target_arch = "wasm32"))]
            {
                log::info!("Copy to clipboard not implemented for desktop");
            }
        }
    };
    
    // Toggle full address display on click
    let toggle_address_display = move |_| {
        let current_state = *show_full_address.read();
        show_full_address.set(!current_state);
    };
    
    // Determine button class based on copy success state
    let button_class = if *copy_success.read() {
        "copy-address-btn copy-success"
    } else {
        "copy-address-btn"
    };
    
    // Address container class based on display state
    let address_class = if *show_full_address.read() {
        "wallet-address full-address"
    } else {
        "wallet-address"
    };
    
    rsx! {
        div { class: "wallet-address-container",
            div {
                class: "{address_class}",
                title: "Click to toggle full address display",
                onclick: toggle_address_display,
                "{display_address}"
            }
            button {
                class: "{button_class}",
                title: "Copy full address: {address}",
                onclick: copy_to_clipboard,
                // Empty button content, using CSS for the icon
                ""
            }
        }
    }
}

/// A component that renders a 50x50 pixel grid from a hex string
#[component]
pub fn PixelCanvas(hex_string: String) -> Element {
    let grid_size = 50;
    
    // Function to convert hex string to a 2D grid of 0s and 1s
    let generate_grid = |hex_str: &str| -> Vec<Vec<u8>> {
        let mut grid = vec![vec![0; grid_size]; grid_size];
        let hex_str = hex_str.trim().to_uppercase();
        
        let mut hex_index = 0;
        
        // Process each row
        for row in 0..grid_size {
            let mut col = 0;
            while col < grid_size && hex_index < hex_str.len() {
                // Get the hex digit and convert to binary (4 bits)
                if let Some(hex_digit) = hex_str.chars().nth(hex_index) {
                    if let Ok(decimal) = u8::from_str_radix(&hex_digit.to_string(), 16) {
                        // Convert to binary (4 bits)
                        let binary = format!("{:04b}", decimal);
                        
                        // Set the pixels according to binary representation
                        for (bit_idx, bit) in binary.chars().enumerate() {
                            if col + bit_idx < grid_size {
                                if bit == '1' {
                                    grid[row][col + bit_idx] = 1;
                                }
                            }
                        }
                    }
                    hex_index += 1;
                    col += 4; // Move to the next 4 columns
                } else {
                    break;
                }
            }
        }
        
        log::info!("Generated pixel grid from hex string. Used {} of {} hex chars", hex_index, hex_str.len());
        grid
    };
    
    // Generate the grid from the hex string
    let grid = generate_grid(&hex_string);
    
    rsx! {
        div {
            class: "pixel-canvas-container",
            div {
                class: "pixel-grid",
                // Render each pixel in the grid
                {
                    (0..grid_size).map(|row_idx| {
                        rsx! {
                            div {
                                class: "pixel-row",
                                key: "{row_idx}",
                                {
                                    (0..grid_size).map(|col_idx| {
                                        let pixel_class = if grid[row_idx][col_idx] == 1 { "pixel active" } else { "pixel" };
                                        rsx! {
                                            div {
                                                class: "{pixel_class}",
                                                key: "{row_idx}-{col_idx}"
                                            }
                                        }
                                    })
                                }
                            }
                        }
                    })
                }
            }
        }
    }
} 