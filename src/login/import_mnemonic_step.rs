use leptos::*;
use crate::CreateWalletStep;
use crate::wallet::verify_mnemonic;

#[component]
pub fn ImportMnemonicStep(
    set_current_step: WriteSignal<CreateWalletStep>,
    set_mnemonic: WriteSignal<String>,
) -> impl IntoView {
    let (mnemonic_input, set_mnemonic_input) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(String::new());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        let mnemonic = mnemonic_input.get().trim().to_string();
        
        // verify mnemonic format
        if mnemonic.split_whitespace().count() != 12 && mnemonic.split_whitespace().count() != 24 {
            set_error_message.set("Please enter 12 or 24 words".to_string());
            return;
        }

        // verify mnemonic validity
        if !verify_mnemonic(&mnemonic) {
            set_error_message.set("Invalid mnemonic phrase".to_string());
            return;
        }

        // save mnemonic and enter set password step
        set_mnemonic.set(mnemonic);
        set_current_step.set(CreateWalletStep::SetPassword);
    };

    view! {
        <div class="login-container">
            <div class="header-with-back">
                <button 
                    class="back-btn"
                    on:click=move |_| set_current_step.set(CreateWalletStep::Initial)
                >
                    "← Back"
                </button>
                <h2>"Import Wallet"</h2>
            </div>
            
            <form on:submit=on_submit>
                <div class="mnemonic-input-section">
                    <p class="instruction-text">
                        "Enter your 12 or 24 word recovery phrase"
                    </p>
                    
                    <textarea
                        class="mnemonic-textarea"
                        placeholder="Enter your recovery phrase (each word separated by a space)"
                        on:input=move |ev| {
                            set_mnemonic_input.set(event_target_value(&ev));
                        }
                        required
                    />
                </div>

                <div class="error-message">
                    {move || error_message.get()}
                </div>

                <button type="submit" class="wallet-btn">
                    "Continue"
                </button>
            </form>
        </div>
    }
}