use leptos::*;
use crate::CreateWalletStep;

#[component]
pub fn SetPasswordStep(
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