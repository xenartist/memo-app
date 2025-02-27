use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::wallet;
use crate::storage;
use crate::components::{MnemonicModal, WalletAddressDisplay, PixelCanvas};
use crate::config;
use crate::rpc::RpcService;

// Define application routes
#[derive(Routable, Clone)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

// Home page component
pub fn Home() -> Element {
    // State for modal visibility and mnemonic
    let mut show_modal = use_signal(|| false);
    let mut mnemonic = use_signal(|| String::new());
    let mut wallet_saved = use_signal(|| false);
    let mut error_message = use_signal(|| String::new());
    let mut wallet_address = use_signal(|| String::new());
    let mut wallet_balance = use_signal(|| None::<f64>);
    let mut is_loading_balance = use_signal(|| false);
    
    // Password related states
    let mut password = use_signal(|| String::new());
    let mut confirm_password = use_signal(|| String::new());
    let mut show_password_modal = use_signal(|| false);
    let mut is_decrypting = use_signal(|| false);
    let mut decryption_password = use_signal(|| String::new());
    let mut show_decrypt_modal = use_signal(|| false);
    
    // Hex string for the pixel canvas
    let pixel_hex = "00003FFF000000001FFFFE00000007FFFFF8000001FFFFFFE000003FFFFFFF000007FFFFFFF80000FFFFFFFFC0001FFFFFFFFE0003FFFFFFFFF0007FFFFFFFFF800FFFFFFFFFFC01FFCFFFFCFFE01FFCFFFF8FFE03FFE7FFF9FFF03FFF3FFF3FFF07FFF9FFE7FFF87FFF9FFCFFFF87FFFCFFCFFFF8FFFFE7F9FFFFCFFFFF3F3FFFFCFFFFF1E7FFFFCFFFFF9E7FFFFCFFFFFCCFFFFFCFFFFFE1FFFFFCFFFFFE3FFFFFCFFFFFF3FFFFFCFFFFFE1FFFFFCFFFFFCCFFFFFCFFFFF9C7FFFFCFFFFF9E7FFFFCFFFFF3F3FFFFCFFFFE7F9FFFFC7FFFCFF8FFFF87FFFCFFCFFFF87FFF9FFE7FFF83FFF3FFF3FFF03FFE7FFF1FFF01FFC7FFF9FFE01FFCFFFFCFFE00FFFFFFFFFFC007FFFFFFFFF8003FFFFFFFFF0001FFFFFFFFE0000FFFFFFFFC00007FFFFFFF800003FFFFFFF000001FFFFFFE0000007FFFFF80000001FFFFE000000003FFF00000";
    
    // Try to load existing wallet on component mount
    use_effect(move || {
        // Debug: Print localStorage content
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    if let Ok(Some(wallet_data)) = local_storage.get_item("wallet_data") {
                        log::info!("Raw wallet data in localStorage: {}", wallet_data);
                    } else {
                        log::info!("No wallet_data found in localStorage");
                    }
                }
            }
        }
        
        // Load wallet data
        match storage::load_wallet() {
            Ok(Some(wallet)) => {
                let address = wallet.address.clone();
                wallet_address.set(address.clone());
                wallet_saved.set(true);
                log::info!("Loaded existing wallet with address: {}", address);
                
                // Fetch wallet balance
                fetch_balance(address, wallet_balance.clone(), is_loading_balance.clone());
            },
            Ok(None) => {
                log::info!("No existing wallet found");
            },
            Err(err) => {
                error_message.set(format!("Failed to load wallet: {}", err));
                log::error!("Failed to load wallet: {}", err);
            }
        }
    });
    
    let generate_new_wallet = move |_: MouseEvent| {
        // Generate a new mnemonic
        let new_mnemonic = wallet::generate_mnemonic();
        mnemonic.set(new_mnemonic);
        
        // Show password input modal
        password.set(String::new());
        confirm_password.set(String::new());
        show_password_modal.set(true);
        
        log::info!("New Wallet button clicked, showing password modal");
    };
    
    let confirm_password_and_show_mnemonic = move |_: MouseEvent| {
        let pwd = password.read().clone();
        let confirm_pwd = confirm_password.read().clone();
        
        // Validate password
        if pwd.is_empty() {
            error_message.set("Password cannot be empty".to_string());
            return;
        }
        
        if pwd != confirm_pwd {
            error_message.set("Passwords do not match".to_string());
            return;
        }
        
        // Password validation passed, show mnemonic
        show_password_modal.set(false);
        show_modal.set(true);
        wallet_saved.set(false);
        error_message.set(String::new());
        
        log::info!("Password confirmed, showing mnemonic");
    };
    
    let save_wallet = move |_: ()| {
        let mnemonic_str = mnemonic.read().clone();
        let pwd = password.read().clone();
        
        match storage::create_and_save_wallet(mnemonic_str, &pwd) {
            Ok(wallet) => {
                let address = wallet.address.clone();
                wallet_address.set(address.clone());
                wallet_saved.set(true);
                error_message.set(String::new());
                log::info!("Wallet saved successfully with address: {}", address);
                log::info!("wallet_saved state is now: {}", *wallet_saved.read());
                
                // Fetch wallet balance for the new wallet
                fetch_balance(address, wallet_balance.clone(), is_loading_balance.clone());
            },
            Err(err) => {
                error_message.set(format!("Failed to save wallet: {}", err));
                log::error!("Failed to save wallet: {}", err);
            }
        }
        
        show_modal.set(false);
    };
    
    let close_modal = move |_: ()| {
        if !*wallet_saved.read() {
            log::warn!("Modal closed without saving wallet");
        }
        show_modal.set(false);
    };
    
    let close_password_modal = move |_: MouseEvent| {
        show_password_modal.set(false);
    };
    
    let close_decrypt_modal = move |_: MouseEvent| {
        show_decrypt_modal.set(false);
        decryption_password.set(String::new());
    };
    
    let clear_wallet = move |_: MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    let _ = local_storage.remove_item("wallet_data");
                    log::info!("Wallet data cleared from localStorage");
                    
                    // Reset state
                    wallet_address.set(String::new());
                    wallet_saved.set(false);
                    mnemonic.set(String::new());
                    error_message.set(String::new());
                    wallet_balance.set(None);
                }
            }
        }
    };
    
    let show_decrypt_dialog = move |_: MouseEvent| {
        decryption_password.set(String::new());
        show_decrypt_modal.set(true);
        is_decrypting.set(false);
        error_message.set(String::new());
    };
    
    let decrypt_mnemonic = move |_: MouseEvent| {
        is_decrypting.set(true);
        let pwd = decryption_password.read().clone();
        
        if pwd.is_empty() {
            error_message.set("Password cannot be empty".to_string());
            is_decrypting.set(false);
            return;
        }
        
        match storage::load_wallet() {
            Ok(Some(wallet)) => {
                match storage::decrypt_mnemonic(&wallet, &pwd) {
                    Ok(decrypted_mnemonic) => {
                        mnemonic.set(decrypted_mnemonic);
                        show_modal.set(true);
                        show_decrypt_modal.set(false);
                        error_message.set(String::new());
                        log::info!("Mnemonic decrypted successfully");
                    },
                    Err(err) => {
                        error_message.set(format!("Failed to decrypt mnemonic: {}", err));
                        log::error!("Failed to decrypt mnemonic: {}", err);
                    }
                }
            },
            Ok(None) => {
                error_message.set("No wallet found".to_string());
                log::error!("No wallet found");
            },
            Err(err) => {
                error_message.set(format!("Failed to load wallet: {}", err));
                log::error!("Failed to load wallet: {}", err);
            }
        }
        
        is_decrypting.set(false);
    };
    
    let refresh_balance = move |_: MouseEvent| {
        let address = wallet_address.read().clone();
        if !address.is_empty() {
            fetch_balance(address, wallet_balance.clone(), is_loading_balance.clone());
        }
    };
    
    rsx! {
        // Header with wallet address
        header {
            class: "app-header",
            div { class: "app-title", "memo" }
            
            // Show wallet address if available
            {
                let addr = wallet_address.read();
                if !addr.is_empty() {
                    log::info!("Displaying wallet address: {}", addr);
                    rsx! {
                        WalletAddressDisplay { address: addr.clone() }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
        }
        
        div {
            class: "container",
            
            // Show wallet creation button only if no wallet exists
            {
                let is_wallet_saved = *wallet_saved.read();
                log::info!("Rendering UI with wallet_saved: {}", is_wallet_saved);
                
                if !is_wallet_saved {
                    rsx! {
                        button {
                            class: "new-wallet-btn",
                            onclick: generate_new_wallet,
                            "New Wallet"
                        }
                    }
                } else {
                    rsx! { 
                        div { class: "wallet-dashboard",
                            h1 { "Memo Inscription - X1 Testnet" }
                            
                            div { class: "wallet-balance",
                                div { class: "balance-header",
                                    h2 { "Balance" }
                                    button {
                                        class: "refresh-btn",
                                        onclick: refresh_balance,
                                        disabled: *is_loading_balance.read(),
                                        i { class: "refresh-icon" }
                                    }
                                }
                                
                                {
                                    if *is_loading_balance.read() {
                                        rsx! {
                                            p { class: "balance-loading", "Loading..." }
                                        }
                                    } else if let Some(balance) = *wallet_balance.read() {
                                        rsx! {
                                            p { class: "balance-amount", "{balance:.9} {config::TOKEN_SYMBOL}" }
                                            p { class: "balance-usd", "($ 0.00 USD)" }
                                            p { class: "balance-network", "Network: {config::NETWORK_NAME}" }
                                        }
                                    } else {
                                        rsx! {
                                            p { class: "balance-amount", "0 {config::TOKEN_SYMBOL}" }
                                            p { class: "balance-usd", "($ 0.00 USD)" }
                                            p { class: "balance-network", "Network: {config::NETWORK_NAME}" }
                                        }
                                    }
                                }
                            }
                            
                            div { class: "wallet-actions",
                                button { class: "action-btn receive-btn", "Receive" }
                                button { class: "action-btn send-btn", "Send" }
                            }
                            
                            // Add Memo NFT Display with Pixel Canvas
                            div { class: "memo-nft",
                                div { class: "memo-nft-title", "Memo Inscription" }
                                PixelCanvas { hex_string: pixel_hex.to_string() }
                            }
                            
                            div { class: "transaction-history",
                                h2 { "Recent Transactions" }
                                p { class: "no-transactions", "No transactions yet" }
                            }
                            
                            // Add buttons for wallet management
                            div { class: "wallet-management",
                                button {
                                    class: "action-btn view-mnemonic-btn",
                                    onclick: show_decrypt_dialog,
                                    "View Recovery Phrase"
                                }
                                
                                button {
                                    class: "action-btn clear-btn",
                                    onclick: clear_wallet,
                                    "Clear Wallet (Debug)"
                                }
                            }
                        }
                    }
                }
            }
            
            // Show success message if wallet was saved
            {
                if *wallet_saved.read() {
                    rsx! {
                        div { class: "success-message",
                            "Wallet created and saved successfully!"
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Show error message if there was an error
            {
                let err = error_message.read();
                if !err.is_empty() {
                    rsx! {
                        div { class: "error-message",
                            "{err}"
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Password input modal
            {
                if *show_password_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal password-modal",
                                h2 { "Set Wallet Password" }
                                p { "This password will be used to encrypt your recovery phrase." }
                                
                                div { class: "form-group",
                                    label { "Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{password}",
                                        oninput: move |evt| password.set(evt.value().clone()),
                                        placeholder: "Enter password"
                                    }
                                }
                                
                                div { class: "form-group",
                                    label { "Confirm Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{confirm_password}",
                                        oninput: move |evt| confirm_password.set(evt.value().clone()),
                                        placeholder: "Confirm password"
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_password_modal,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: confirm_password_and_show_mnemonic,
                                        "Confirm"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Mnemonic decryption modal
            {
                if *show_decrypt_modal.read() {
                    rsx! {
                        div { class: "modal-overlay",
                            div { class: "modal decrypt-modal",
                                h2 { "Enter Wallet Password" }
                                p { "Enter your password to view your recovery phrase." }
                                
                                div { class: "form-group",
                                    label { "Password:" }
                                    input {
                                        r#type: "password",
                                        value: "{decryption_password}",
                                        oninput: move |evt| decryption_password.set(evt.value().clone()),
                                        placeholder: "Enter password"
                                    }
                                }
                                
                                div { class: "modal-actions",
                                    button {
                                        class: "cancel-btn",
                                        onclick: close_decrypt_modal,
                                        "Cancel"
                                    }
                                    button {
                                        class: "confirm-btn",
                                        onclick: decrypt_mnemonic,
                                        disabled: *is_decrypting.read(),
                                        if *is_decrypting.read() {
                                            "Decrypting..."
                                        } else {
                                            "View Phrase"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    rsx! { Fragment {} }
                }
            }
            
            // Render the mnemonic modal
            MnemonicModal {
                mnemonic: mnemonic.read().clone(),
                visible: *show_modal.read(),
                on_close: save_wallet
            }
        }
    }
}

// Function to fetch balance from RPC
fn fetch_balance(address: String, mut balance: Signal<Option<f64>>, mut is_loading: Signal<bool>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen_futures::spawn_local;
        
        is_loading.set(true);
        log::info!("Fetching balance for address: {}", address);
        
        let rpc_service = RpcService::new();
        
        spawn_local(async move {
            match rpc_service.get_balance(&address).await {
                Ok(bal) => {
                    log::info!("Balance fetched: {} XNT", bal);
                    balance.set(Some(bal));
                },
                Err(e) => {
                    log::error!("Failed to fetch balance: {}", e);
                    // Keep the old balance, don't set to None
                }
            }
            
            is_loading.set(false);
        });
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        log::warn!("Balance fetching not implemented for desktop/mobile");
    }
}

// 404 page component
#[component]
pub fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div {
            class: "container",
            h1 { "Page Not Found" }
            p { "We couldn't find the page: {route:?}" }
            nav {
                Link { to: Route::Home {}, "Back to Home" }
            }
        }
    }
} 