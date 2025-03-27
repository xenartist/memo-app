use leptos::*;
use crate::CreateWalletStep;

#[component]
pub fn VerifyMnemonicStep(
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