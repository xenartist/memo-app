use leptos::*;
use crate::CreateWalletStep;

#[component]
pub fn InitialStep(
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