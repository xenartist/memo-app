use leptos::*;
use crate::core::session::Session;
use crate::core::{NetworkType, initialize_network};
use wasm_bindgen_futures::spawn_local;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn CompleteStep(
    wallet_address: ReadSignal<String>,
    set_show_main_page: WriteSignal<bool>,
    session: RwSignal<Session>,
    encrypted_seed: String,
    password: String,
    selected_network: RwSignal<NetworkType>,
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

            // Initialize network first
            let network = selected_network.get_untracked();
            if initialize_network(network) {
                let mut current_session = session.get_untracked();
                // Set network in session
                current_session.set_network(network);
                
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
            } else {
                set_error_message.set("Failed to initialize network".to_string());
                set_is_initializing.set(false);
            }
        });
    };

    view! {
        <div class="login-container">
            <div class="success-header" style="text-align: center; margin-bottom: 2rem;">
                <i class="fas fa-check-circle" style="font-size: 4rem; color: #059669; margin-bottom: 1rem;"></i>
                <h2>"Wallet Created Successfully!"</h2>
            </div>
            
            <div class="wallet-info" style="display: flex; flex-direction: column; align-items: center;">
                <h3 style="display: flex; align-items: center; justify-content: center; gap: 0.5rem; width: 100%; text-align: center; margin-bottom: 12px;">
                    <i class="fas fa-wallet"></i>
                    "Your Wallet Address"
                </h3>
                <div class="address-container" style="display: block; width: 100%; max-width: 500px; margin: 16px 0; padding: 16px; background: linear-gradient(135deg, #f8f9fa 0%, #e9ecef 100%); border: 2px solid #dee2e6; border-radius: 8px; box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);">
                    <code class="wallet-address" style="display: block; width: 100%; text-align: center; font-family: 'Courier New', monospace; word-break: break-all; font-size: 0.9rem; color: #333;">
                        {move || wallet_address.get()}
                    </code>
                </div>
                <p class="info-text" style="display: flex; align-items: center; justify-content: center; gap: 0.5rem; width: 100%; text-align: center; margin-top: 12px; font-size: 14px; color: #6c757d;">
                    <i class="fas fa-info-circle"></i>
                    "This is your wallet address. You can use it to receive tokens."
                </p>
            </div>

            <div class="security-tips" style="max-width: 500px; margin: 2rem auto; padding: 1.5rem; background: linear-gradient(135deg, #fff3e0 0%, #ffe0b2 100%); border-left: 4px solid #ea580c; border-radius: 8px; box-shadow: 0 2px 8px rgba(234, 88, 12, 0.15);">
                <h3 style="display: flex; align-items: center; gap: 0.5rem; color: #ea580c; margin-bottom: 1rem;">
                    <i class="fas fa-shield-alt"></i>
                    "Security Tips"
                </h3>
                <ul style="list-style: none; padding: 0; margin: 0;">
                    <li style="display: flex; align-items: flex-start; gap: 0.75rem; margin-bottom: 0.75rem; color: #333;">
                        <i class="fas fa-lock" style="color: #ea580c; margin-top: 0.2rem;"></i>
                        <span>"Never share your mnemonic phrase or password with anyone"</span>
                    </li>
                    <li style="display: flex; align-items: flex-start; gap: 0.75rem; margin-bottom: 0.75rem; color: #333;">
                        <i class="fas fa-save" style="color: #ea580c; margin-top: 0.2rem;"></i>
                        <span>"Make sure to store your mnemonic phrase in a safe place"</span>
                    </li>
                    <li style="display: flex; align-items: flex-start; gap: 0.75rem; color: #333;">
                        <i class="fas fa-usb" style="color: #ea580c; margin-top: 0.2rem;"></i>
                        <span>"Consider using a hardware wallet for large amounts"</span>
                    </li>
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
            <div class="error-message">
                {move || if !error_message.get().is_empty() {
                    view! {
                        <i class="fas fa-exclamation-circle"></i>
                        <span>{error_message.get()}</span>
                    }.into_view()
                } else {
                    view! { <></> }.into_view()
                }}
            </div>

            <button 
                class="wallet-btn"
                on:click=handle_enter
                prop:disabled=move || is_initializing.get()
            >
                {move || if is_initializing.get() { 
                    view! {
                        <i class="fas fa-spinner fa-spin"></i>
                        " Initializing..."
                    }.into_view()
                } else { 
                    view! {
                        <i class="fas fa-rocket"></i>
                        " Let's GO!"
                    }.into_view()
                }}
            </button>
        </div>
    }
} 