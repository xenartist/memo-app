use leptos::*;
use crate::CreateWalletStep;

#[component]
pub fn CompleteStep(
    wallet_address: ReadSignal<String>,
    set_show_main_page: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="login-container">
            <h2>"Wallet Created Successfully!"</h2>
            
            <div class="wallet-info">
                <h3>"Your Wallet Address"</h3>
                <div class="address-container">
                    <code class="wallet-address">
                        {move || wallet_address.get()}
                    </code>
                </div>
                <p class="info-text">
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
                on:click=move |_| set_show_main_page.set(true)
            >
                "Let's GO!"
            </button>
        </div>
    }
} 