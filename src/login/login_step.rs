use leptos::*;
use crate::core::wallet::Wallet;
use crate::core::encrypt;
use crate::core::session::Session;
use crate::CreateWalletStep;

#[derive(Clone, PartialEq)]
enum ResetState {
    None,
    Confirming,
    Success,
}

#[component]
pub fn LoginStep(
    set_current_step: WriteSignal<CreateWalletStep>,
    session: RwSignal<Session>,
    set_show_main_page: WriteSignal<bool>,
) -> impl IntoView {
    let (password, set_password) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(String::new());
    let (reset_state, set_reset_state) = create_signal(ResetState::None);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        let password = password.clone();
        let session = session.clone();
        
        spawn_local(async move {
            let password_value = password.get_untracked();
            
            match Wallet::load().await {
                Ok(wallet) => {
                    match encrypt::decrypt(wallet.get_encrypted_seed(), &password_value) {
                        Ok(seed) => {
                            let mut current_session = session.get_untracked();
                            if let Ok(()) = current_session.initialize(wallet.get_encrypted_seed(), &password_value) {
                                session.set(current_session);
                                set_show_main_page.set(true);
                            } else {
                                set_error_message.set("Failed to initialize session".to_string());
                            }
                        }
                        Err(_) => {
                            set_error_message.set("Invalid password".to_string());
                        }
                    }
                }
                Err(_) => {
                    set_error_message.set("Failed to load wallet".to_string());
                }
            }
        });
    };

    // handle the reset wallet
    let handle_reset = move |_| {
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    // clear the wallet data
                    storage.remove_item("wallet").ok();
                    // show the success message
                    set_reset_state.set(ResetState::Success);
                }
            }
        });
    };

    view! {
        <div class="login-container">
            <h2>"Login to Your Wallet"</h2>
            
            <div>
                {move || {
                    match reset_state.get() {
                        ResetState::Confirming => view! {
                            <div class="reset-confirm-dialog">
                                <div class="reset-confirm-content">
                                    <h3>"Reset Wallet Confirmation"</h3>
                                    <p class="warning-text">
                                        "Warning: This action will delete current wallet and cannot be undone. "
                                        "Make sure you have backed up your mnemonic phrase before proceeding."
                                    </p>
                                    <div class="button-group">
                                        <button 
                                            class="cancel-btn"
                                            on:click=move |_| set_reset_state.set(ResetState::None)
                                        >
                                            "Cancel"
                                        </button>
                                        <button 
                                            class="reset-btn"
                                            on:click=handle_reset
                                        >
                                            "Reset Wallet"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        },
                        ResetState::Success => view! {
                            <div class="reset-confirm-dialog">
                                <div class="reset-confirm-content">
                                    <h3>"Wallet Reset Successfully"</h3>
                                    <p>
                                        "Your wallet has been reset successfully. You can now create a new wallet or import an existing one."
                                    </p>
                                    <div class="button-group">
                                        <button 
                                            class="wallet-btn"
                                            on:click=move |_| set_current_step.set(CreateWalletStep::Initial)
                                        >
                                            "Continue"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        },
                        ResetState::None => view! {
                            <div>
                                <form on:submit=on_submit>
                                    <div class="password-section">
                                        <input
                                            type="password"
                                            placeholder="Enter your wallet password"
                                            on:input=move |ev| {
                                                set_password.set(event_target_value(&ev));
                                            }
                                            required
                                        />
                                    </div>

                                    <div class="error-message">
                                        {move || error_message.get()}
                                    </div>

                                    <button type="submit" class="wallet-btn">
                                        "Login"
                                    </button>

                                    <div class="reset-link">
                                        <a 
                                            href="#"
                                            on:click=move |ev| {
                                                ev.prevent_default();
                                                set_reset_state.set(ResetState::Confirming);
                                            }
                                        >
                                            "Forget password and reset wallet"
                                        </a>
                                    </div>
                                </form>
                            </div>
                        }
                    }
                }}
            </div>
        </div>
    }
} 