use leptos::*;
use wasm_bindgen::prelude::*;
use crate::components::*;

// create wallet step
#[derive(Clone, Debug, PartialEq)]
pub enum CreateWalletStep {
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
