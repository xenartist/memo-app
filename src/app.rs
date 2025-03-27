use leptos::*;
use leptos::{
    component,
    view,
    IntoView,
    spawn_local,
    create_signal,
    ReadSignal,
    WriteSignal,
};
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

// initial step
#[component]
fn InitialStep(
    set_current_step: WriteSignal<CreateWalletStep>,
    set_mnemonic: WriteSignal<String>
) -> impl IntoView {
    let handle_new_wallet = move |_| {
        if let Ok(new_mnemonic) = crate::wallet::generate_mnemonic(24) {
            set_mnemonic.set(new_mnemonic.clone());
            set_current_step.set(CreateWalletStep::ShowMnemonic(new_mnemonic));
        }
    };

    view! {
        <div class="login-container">
            <h1 class="app-title">"Memo App"</h1>
            <div class="button-group">
                <button class="wallet-btn new-wallet" on:click=handle_new_wallet>
                    "New Wallet"
                </button>
                <button class="wallet-btn import-wallet">
                    "Import Wallet"
                </button>
            </div>
        </div>
    }
}

// show mnemonic step
#[component]
fn ShowMnemonicStep(
    phrase: String,
    set_current_step: WriteSignal<CreateWalletStep>
) -> impl IntoView {
    view! {
        <div class="login-container">
            <h2>"Backup Your Mnemonic Phrase"</h2>
            <div class="mnemonic-display">{phrase.clone()}</div>
            <div class="button-group">
                <button class="wallet-btn" 
                    on:click=move |_| set_current_step.set(CreateWalletStep::VerifyMnemonic(phrase.clone()))>
                    "I've Written It Down"
                </button>
            </div>
        </div>
    }
}

// verify mnemonic step
#[component]
fn VerifyMnemonicStep(
    mnemonic: ReadSignal<String>,
    verification_input: ReadSignal<String>,
    set_verification_input: WriteSignal<String>,
    set_current_step: WriteSignal<CreateWalletStep>
) -> impl IntoView {
    let handle_verify = move |_| {
        if mnemonic.get() == verification_input.get() {
            set_current_step.set(CreateWalletStep::SetPassword);
        }
    };

    view! {
        <div class="login-container">
            <h2>"Verify Your Mnemonic Phrase"</h2>
            <textarea
                on:input=move |ev| set_verification_input.set(event_target_value(&ev))
            />
            <div class="button-group">
                <button class="wallet-btn" on:click=handle_verify>
                    "Verify"
                </button>
            </div>
        </div>
    }
}

// set password step
#[component]
fn SetPasswordStep(
    mnemonic: ReadSignal<String>,
    password: ReadSignal<String>,
    set_password: WriteSignal<String>,
    set_current_step: WriteSignal<CreateWalletStep>
) -> impl IntoView {
    let handle_create = move |_| {
        let m = mnemonic.get();
        let p = password.get();
        spawn_local(async move {
            if let Ok(_) = crate::wallet::store_encrypted_mnemonic(&m, &p).await {
                set_current_step.set(CreateWalletStep::Complete);
            }
        });
    };

    view! {
        <div class="login-container">
            <h2>"Set Password"</h2>
            <input type="password"
                on:input=move |ev| set_password.set(event_target_value(&ev))
            />
            <div class="button-group">
                <button class="wallet-btn" on:click=handle_create>
                    "Create Wallet"
                </button>
            </div>
        </div>
    }
}

// complete step
#[component]
fn CompleteStep() -> impl IntoView {
    view! {
        <div class="login-container">
            <h2>"Wallet Created Successfully!"</h2>
        </div>
    }
}

// main app component
#[component]
pub fn App() -> impl IntoView {
    let (current_step, set_current_step) = create_signal(CreateWalletStep::Initial);
    let (mnemonic, set_mnemonic) = create_signal(String::new());
    let (verification_input, set_verification_input) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());

    view! {
        <main class="container">
            {move || match current_step.get() {
                CreateWalletStep::Initial => view! {
                    <InitialStep
                        set_current_step=set_current_step
                        set_mnemonic=set_mnemonic
                    />
                },
                CreateWalletStep::ShowMnemonic(phrase) => view! {
                    <ShowMnemonicStep
                        phrase=phrase
                        set_current_step=set_current_step
                    />
                },
                CreateWalletStep::VerifyMnemonic(_) => view! {
                    <VerifyMnemonicStep
                        mnemonic=mnemonic
                        verification_input=verification_input
                        set_verification_input=set_verification_input
                        set_current_step=set_current_step
                    />
                },
                CreateWalletStep::SetPassword => view! {
                    <SetPasswordStep
                        mnemonic=mnemonic
                        password=password
                        set_password=set_password
                        set_current_step=set_current_step
                    />
                },
                CreateWalletStep::Complete => view! {
                    <CompleteStep/>
                }
            }}
        </main>
    }
}
