use leptos::*;
use crate::CreateWalletStep;
use crate::core::session::Session;

#[component]
pub fn CompleteStep(
    wallet_address: ReadSignal<String>,
    set_show_main_page: WriteSignal<bool>,
    session: RwSignal<Session>,
    encrypted_seed: String,
    password: String,
) -> impl IntoView {
    let handle_enter = move |_| async move {
        let mut current_session = session.get();
        if let Ok(()) = current_session.initialize(&encrypted_seed, &password).await {
            session.set(current_session);
            set_show_main_page.set(true);
        }
    };

    view! {
        <div class="login-container">
            <h2>"Wallet Created Successfully!"</h2>
            
            <div class="wallet-info" style="display: flex; flex-direction: column; align-items: center;">
                <h3 style="display: block; width: 100%; text-align: center; margin-bottom: 12px;">
                    "Your Wallet Address"
                </h3>
                <div class="address-container" style="display: block; width: 100%; margin: 16px 0; padding: 16px; background-color: #f8f9fa; border: 1px solid #e9ecef; border-radius: 8px;">
                    <code class="wallet-address" style="display: block; width: 100%; text-align: center; font-family: monospace; word-break: break-all;">
                        {move || wallet_address.get()}
                    </code>
                </div>
                <p class="info-text" style="display: block; width: 100%; text-align: center; margin-top: 12px; font-size: 14px; color: #6c757d;">
                    "This is your wallet address. You can use it to receive tokens."
                </p>
            </div>

            <div class="security-tips">
                <h3>"Security Tips"</h3>
                <ul>
                    <li>"Never share your mnemonic phrase or password with anyone"</li>
                    <li>"Make sure to store your mnemonic phrase in a safe place"</li>
                    <li>"Consider using a hardware wallet for large amounts"</li>
                </ul>
            </div>

            <button 
                class="wallet-btn"
                on:click=move |e| {
                    let handle = handle_enter.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        handle(e).await;
                    });
                }
            >
                "Let's GO!"
            </button>
        </div>
    }
} 