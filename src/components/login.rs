use dioxus::prelude::*;
use bip39::{Mnemonic, Language};
use rand::RngCore;

#[component]
pub fn LoginPage() -> Element {
    let mut show_dialog = use_signal(|| false);
    let mut seed_words = use_signal(|| Vec::<String>::new());
    let mut selected_length = use_signal(|| 12);

    let mut generate_seed = move || {
        // calculate entropy bytes
        let entropy_bytes = if selected_length() == 24 { 32 } else { 16 };
        let mut entropy = vec![0u8; entropy_bytes];
        rand::rng().fill_bytes(&mut entropy);
        
        // create mnemonic
        if let Ok(mnemonic) = Mnemonic::from_entropy_in(Language::English, &entropy) {
            let words: Vec<String> = mnemonic.words().map(String::from).collect();
            seed_words.set(words);
        }
    };

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
                        show_dialog.set(true);
                        generate_seed();
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

            dialog {
                open: show_dialog(),
                class: "seed-dialog",
                div {
                    class: "dialog-content",
                    h2 { "Select Seed Words Length" }
                    
                    div {
                        class: "radio-group",
                        label {
                            input {
                                r#type: "radio",
                                name: "seed-length",
                                value: "12",
                                checked: selected_length() == 12,
                                oninput: move |_| {
                                    selected_length.set(12);
                                    generate_seed();
                                }
                            }
                            " 12 Words"
                        }
                        
                        label {
                            input {
                                r#type: "radio",
                                name: "seed-length",
                                value: "24",
                                checked: selected_length() == 24,
                                oninput: move |_| {
                                    selected_length.set(24);
                                    generate_seed();
                                }
                            }
                            " 24 Words"
                        }
                    }

                    div {
                        class: "seed-words-container",
                        for (i, word) in seed_words().iter().enumerate() {
                            div {
                                class: "seed-word",
                                span { class: "word-index", "{i + 1}." }
                                span { class: "word", "{word}" }
                            }
                        }
                    }
                }
            }
        }
    }
} 