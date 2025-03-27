use leptos::*;
use crate::CreateWalletStep;

#[component]
pub fn ShowMnemonicStep(
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