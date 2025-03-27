use leptos::*;
use crate::CreateWalletStep;

#[component]
pub fn InitialStep(
    set_current_step: WriteSignal<CreateWalletStep>
) -> impl IntoView {
    view! {
        <div class="login-container">
            <h1 class="app-title">"Memo App"</h1>
            <div class="button-group">
                <button 
                    class="wallet-btn new-wallet" 
                    on:click=move |_| set_current_step.set(CreateWalletStep::ShowMnemonic(String::new()))
                >
                    "New Wallet"
                </button>
                <button class="wallet-btn import-wallet">
                    "Import Wallet"
                </button>
            </div>
        </div>
    }
}