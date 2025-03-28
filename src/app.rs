use leptos::*;
use wasm_bindgen::prelude::*;
use crate::login::*;
use crate::pages::main_page::MainPage;

// create wallet step
#[derive(Clone, Debug, PartialEq)]
pub enum CreateWalletStep {
    Initial,
    ImportMnemonic,
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
    let (password, set_password) = create_signal(String::new());
    let (wallet_address, set_wallet_address) = create_signal(String::new());
    let (show_main_page, set_show_main_page) = create_signal(false);

    view! {
        <main class="container">
            {move || {
                if show_main_page.get() {
                    view! { <MainPage/> }
                } else {
                    match current_step.get() {
                        CreateWalletStep::Initial => view! {
                            <InitialStep
                                set_current_step=set_current_step
                            />
                        },
                        CreateWalletStep::ImportMnemonic => view! {
                            <ImportMnemonicStep
                                set_current_step=set_current_step
                                set_mnemonic=set_mnemonic
                            />
                        },
                        CreateWalletStep::ShowMnemonic(_) => view! {
                            <ShowMnemonicStep
                                set_mnemonic=set_mnemonic
                                set_current_step=set_current_step
                            />
                        },
                        CreateWalletStep::VerifyMnemonic(_) => view! {
                            <VerifyMnemonicStep
                                mnemonic=mnemonic
                                set_current_step=set_current_step
                            />
                        },
                        CreateWalletStep::SetPassword => view! {
                            <SetPasswordStep
                                mnemonic=mnemonic
                                password=password
                                set_password=set_password
                                set_current_step=set_current_step
                                set_wallet_address=set_wallet_address
                            />
                        },
                        CreateWalletStep::Complete => view! {
                            <CompleteStep
                                wallet_address=wallet_address
                                set_show_main_page=set_show_main_page
                            />
                        }
                    }
                }
            }}
        </main>
    }
}
