use leptos::*;
use crate::CreateWalletStep;
use crate::core::session::Session;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn CompleteStep(
    wallet_address: ReadSignal<String>,
    set_show_main_page: WriteSignal<bool>,
    session: RwSignal<Session>,
    encrypted_seed: String,
    password: String,
) -> impl IntoView {
    // add loading status
    let (is_initializing, set_is_initializing) = create_signal(false);
    let (error_message, set_error_message) = create_signal(String::new());

    let handle_enter = move |_| {
        // set loading status
        set_is_initializing.set(true);
        set_error_message.set(String::new());

        let encrypted_seed_clone = encrypted_seed.clone();
        let password_clone = password.clone();

        spawn_local(async move {
            // give UI some time to update status
            TimeoutFuture::new(100).await;

            let mut current_session = session.get_untracked();
            
            match current_session.initialize(&encrypted_seed_clone, &password_clone).await {
                Ok(()) => {
                    // give UI some time to display "success" status
                    TimeoutFuture::new(200).await;
                    
                    session.set(current_session);
                    set_show_main_page.set(true);
                }
                Err(e) => {
                    set_error_message.set(format!("Failed to initialize session: {}", e));
                    set_is_initializing.set(false);
                }
            }
        });
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

            // display initializing status
            {move || {
                if is_initializing.get() {
                    view! {
                        <div class="initializing-status">
                            <i class="fas fa-spinner fa-spin"></i>
                            <span>"Initializing wallet session..."</span>
                        </div>
                    }
                } else {
                    view! { <div></div> }
                }
            }}

            // display error message
            {move || {
                let error = error_message.get();
                if !error.is_empty() {
                    view! {
                        <div class="error-message" style="color: #dc3545; text-align: center; margin: 12px 0; font-size: 14px;">
                            {error}
                        </div>
                    }
                } else {
                    view! { <div></div> }
                }
            }}

            <button 
                class="wallet-btn"
                on:click=handle_enter
                prop:disabled=move || is_initializing.get()
            >
                {move || if is_initializing.get() { "Initializing..." } else { "Let's GO!" }}
            </button>
        </div>
    }
} 