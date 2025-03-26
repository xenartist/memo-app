use dioxus::prelude::*;

#[component]
pub fn LoginPage() -> Element {
    rsx! {
        div {
            class: "login-container",
            h1 { 
                class: "login-title",
                "Welcome to Memo App" 
            }
            
            div {
                class: "button-container",
                button {
                    class: "wallet-button new-wallet",
                    onclick: move |_| {
                        // Handle new wallet creation
                    },
                    "New Wallet"
                }
                
                button {
                    class: "wallet-button import-wallet",
                    onclick: move |_| {
                        // Handle wallet import
                    },
                    "Import Wallet"
                }
            }
        }
    }
} 