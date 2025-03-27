use leptos::*;

#[component]
pub fn CompleteStep(
    wallet_address: ReadSignal<String>,
) -> impl IntoView {
    view! {
        <div class="login-container">
            <h2>"Wallet Created Successfully!"</h2>
            
            <div class="wallet-info">
                <h3>"Your X1 Wallet Address"</h3>
                <div class="address-container">
                    <code class="wallet-address">
                        {move || wallet_address.get()}
                    </code>
                </div>
                <p class="info-text">
                    "This is your X1 wallet address. You can use it to receive X1 and other tokens on the X1 network."
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
        </div>
    }
} 