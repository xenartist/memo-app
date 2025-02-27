use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::wallet;
use crate::storage;
use crate::components::{MnemonicModal, WalletAddressDisplay};

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
    
    // Try to load existing wallet on component mount
    use_effect(move || {
        // Load wallet data
        match storage::load_wallet() {
            Ok(Some(wallet)) => {
                wallet_address.set(wallet.address);
                wallet_saved.set(true);
                log::info!("Loaded existing wallet");
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
        show_modal.set(true);
        wallet_saved.set(false);
        log::info!("New Wallet button clicked, showing mnemonic");
    };
    
    let save_wallet = move |_: ()| {
        let mnemonic_str = mnemonic.read().clone();
        
        match storage::create_and_save_wallet(mnemonic_str) {
            Ok(wallet) => {
                let address = wallet.address.clone();
                wallet_address.set(wallet.address);
                wallet_saved.set(true);
                error_message.set(String::new());
                log::info!("Wallet saved successfully with address: {}", address);
                log::info!("wallet_saved state is now: {}", *wallet_saved.read());
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
    
    rsx! {
        // Header with wallet address
        header {
            class: "app-header",
            div { class: "app-title", "memo" }
            
            // Show wallet address if available
            {
                let addr = wallet_address.read();
                if !addr.is_empty() {
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
                            h1 { "Welcome to Your X1 Wallet" }
                            
                            div { class: "wallet-balance",
                                h2 { "Balance" }
                                p { class: "balance-amount", "0 XNT" }
                                p { class: "balance-usd", "($ 0.00 USD)" }
                            }
                            
                            div { class: "wallet-actions",
                                button { class: "action-btn receive-btn", "Receive" }
                                button { class: "action-btn send-btn", "Send" }
                                button { class: "action-btn swap-btn", "Swap" }
                            }
                            
                            div { class: "transaction-history",
                                h2 { "Recent Transactions" }
                                p { class: "no-transactions", "No transactions yet" }
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
            
            // Render the mnemonic modal
            MnemonicModal {
                mnemonic: mnemonic.read().clone(),
                visible: *show_modal.read(),
                on_close: save_wallet
            }
        }
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