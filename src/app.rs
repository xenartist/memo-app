use leptos::prelude::*;
use leptos::IntoView;
use leptos::component;
use leptos::view;
use leptos::spawn_local;
use leptos::event_target_value;
use wasm_bindgen::prelude::*;

// create wallet step
#[derive(Clone, Debug, PartialEq)]
enum CreateWalletStep {
    Initial,
    ShowMnemonic(String),
    VerifyMnemonic(String),
    SetPassword,
    Complete,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    let (current_step, set_current_step) = create_signal(CreateWalletStep::Initial);
    let (mnemonic, set_mnemonic) = create_signal(String::new());
    let (verification_input, set_verification_input) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());

    view! {
        <main class="container">
            {move || {
                let step = current_step.get();
                match step {
                    CreateWalletStep::Initial => view! {
                        <div class="login-container">
                            <h1 class="app-title">"Memo App"</h1>
                            <div class="button-group">
                                <button class="wallet-btn new-wallet" 
                                    on:click=move |_| {
                                        if let Ok(new_mnemonic) = crate::wallet::generate_mnemonic(24) {
                                            set_mnemonic.set(new_mnemonic.clone());
                                            set_current_step.set(CreateWalletStep::ShowMnemonic(new_mnemonic));
                                        }
                                    }
                                >
                                    "New Wallet"
                                </button>
                                <button class="wallet-btn import-wallet">
                                    "Import Wallet"
                                </button>
                            </div>
                        </div>
                    },
                    CreateWalletStep::ShowMnemonic(phrase) => view! {
                        <div class="login-container">
                            <h2>"Backup Your Mnemonic Phrase"</h2>
                            <div class="mnemonic-display">{phrase.clone()}</div>
                            <div class="button-group">
                                <button class="wallet-btn" 
                                    on:click=move |_| {
                                        set_current_step.set(CreateWalletStep::VerifyMnemonic(phrase.clone()))
                                    }
                                >
                                    "I've Written It Down"
                                </button>
                            </div>
                        </div>
                    },
                    CreateWalletStep::VerifyMnemonic(_) => view! {
                        <div class="login-container">
                            <h2>"Verify Your Mnemonic Phrase"</h2>
                            <textarea
                                on:input=move |ev| set_verification_input.set(event_target_value(&ev))
                            />
                            <div class="button-group">
                                <button class="wallet-btn" 
                                    on:click=move |_| {
                                        if verification_input.get() == mnemonic.get() {
                                            set_current_step.set(CreateWalletStep::SetPassword);
                                        }
                                    }
                                >
                                    "Verify"
                                </button>
                            </div>
                        </div>
                    },
                    CreateWalletStep::SetPassword => view! {
                        <div class="login-container">
                            <h2>"Set Password"</h2>
                            <input type="password"
                                on:input=move |ev| set_password.set(event_target_value(&ev))
                            />
                            <div class="button-group">
                                <button class="wallet-btn" 
                                    on:click=move |_| {
                                        let m = mnemonic.get();
                                        let p = password.get();
                                        spawn_local(async move {
                                            if let Ok(_) = crate::wallet::store_encrypted_mnemonic(&m, &p).await {
                                                set_current_step.set(CreateWalletStep::Complete);
                                            }
                                        });
                                    }
                                >
                                    "Create Wallet"
                                </button>
                            </div>
                        </div>
                    },
                    CreateWalletStep::Complete => view! {
                        <div class="login-container">
                            <h2>"Wallet Created Successfully!"</h2>
                        </div>
                    }
                }
            }}
        </main>
    }
}
