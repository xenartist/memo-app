use leptos::*;
use crate::core::session::WalletType;

#[component]
pub fn LockScreen(
    on_unlock: impl Fn(String) + 'static,
    wallet_type: impl Fn() -> WalletType + 'static,
) -> impl IntoView {
    let (password, set_password) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(String::new());
    let (is_unlocking, set_is_unlocking) = create_signal(false);
    
    log::info!("LockScreen component initialized");
    
    // Store callbacks in values that can be accessed without moving
    let on_unlock = store_value(on_unlock);
    let wallet_type = store_value(wallet_type);
    
    let handle_unlock = move |_| {
        let pwd = password.get();
        if pwd.is_empty() {
            set_error_message.set("Please enter your password".to_string());
            return;
        }
        
        set_is_unlocking.set(true);
        set_error_message.set(String::new());
        
        on_unlock.with_value(|f| f(pwd));
    };
    
    let handle_keydown = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" {
            let pwd = password.get();
            if pwd.is_empty() {
                set_error_message.set("Please enter your password".to_string());
                return;
            }
            
            set_is_unlocking.set(true);
            set_error_message.set(String::new());
            
            on_unlock.with_value(|f| f(pwd));
        }
    };

    view! {
        <div class="lock-screen-overlay">
            <div class="lock-screen-content">
                <div class="lock-icon">
                    <i class="fas fa-lock"></i>
                </div>
                
                <h2>"Screen Locked"</h2>
                
                <p class="lock-message">
                    "Enter your password to unlock"
                </p>
                
                <Show when=move || matches!(wallet_type.with_value(|f| f()), WalletType::Internal)>
                    <div class="unlock-form">
                        <input
                            type="password"
                            class="password-input"
                            placeholder="Enter your password"
                            prop:value=move || password.get()
                            on:input=move |ev| set_password.set(event_target_value(&ev))
                            on:keydown=handle_keydown
                            disabled=move || is_unlocking.get()
                            autofocus
                        />
                        
                        <Show when=move || !error_message.get().is_empty()>
                            <p class="error-message">{error_message}</p>
                        </Show>
                        
                        <button
                            class="unlock-btn"
                            on:click=handle_unlock
                            disabled=move || is_unlocking.get()
                        >
                            <Show
                                when=move || is_unlocking.get()
                                fallback=|| view! { 
                                    <>
                                        <i class="fas fa-unlock"></i>
                                        <span>"Unlock"</span>
                                    </>
                                }
                            >
                                <i class="fas fa-spinner fa-spin"></i>
                                <span>"Unlocking..."</span>
                            </Show>
                        </button>
                    </div>
                </Show>
                
                // Backpack wallet message
                <Show when=move || matches!(wallet_type.with_value(|f| f()), WalletType::Backpack)>
                    <div class="backpack-lock-message">
                        <p>"Lock screen is not available for Backpack wallet."</p>
                        <p class="hint">"Please use Backpack's built-in security features."</p>
                    </div>
                </Show>
            </div>
        </div>
    }
}

