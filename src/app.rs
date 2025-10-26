use leptos::*;
use wasm_bindgen::prelude::*;
use crate::login::*;
use crate::pages::main_page::MainPage;
use crate::core::session::Session;
use crate::core::wallet::Wallet;
use crate::core::NetworkType;

// create wallet step
#[derive(Clone, Debug, PartialEq)]
pub enum CreateWalletStep {
    Initial,
    Login,
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
    let (encrypted_seed, set_encrypted_seed) = create_signal(String::new());
    
    // create session manager
    let session = create_rw_signal(Session::new(None));
    
    // network selection (default to Mainnet for production use)
    let selected_network = create_rw_signal(NetworkType::Mainnet);

    // check if wallet exists when app starts
    spawn_local(async move {
        if Wallet::exists().await {
            set_current_step.set(CreateWalletStep::Login);
        }
    });

    view! {
        <main class="container">
            {move || {
                if show_main_page.get() {
                    view! { <MainPage session=session /> }
                } else {
                    match current_step.get() {
                        CreateWalletStep::Initial => view! {
                            <InitialStep
                                set_current_step=set_current_step
                                selected_network=selected_network
                            />
                        },
                        CreateWalletStep::Login => view! {
                            <LoginStep
                                set_current_step=set_current_step
                                session=session
                                set_show_main_page=set_show_main_page
                                selected_network=selected_network
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
                                set_encrypted_seed=set_encrypted_seed
                            />
                        },
                        CreateWalletStep::Complete => view! {
                            <CompleteStep
                                wallet_address=wallet_address
                                set_show_main_page=set_show_main_page
                                session=session
                                encrypted_seed=encrypted_seed.get()
                                password=password.get()
                            />
                        }
                    }
                }
            }}
        </main>
    }
}
