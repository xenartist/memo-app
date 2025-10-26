use leptos::*;
use crate::core::wallet::Wallet;
use crate::core::encrypt;
use crate::core::session::Session;
use crate::core::{NetworkType, initialize_network};
use crate::CreateWalletStep;
use wasm_bindgen::JsCast;

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
    selected_network: RwSignal<NetworkType>,
) -> impl IntoView {
    let (password, set_password) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(String::new());
    let (reset_state, set_reset_state) = create_signal(ResetState::None);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        
        let password = password.clone();
        let session = session.clone();
        
        let button = ev.submitter()
            .unwrap()
            .dyn_into::<web_sys::HtmlButtonElement>()
            .unwrap();
        
        button.set_disabled(true);
        button.set_text_content(Some("Unlocking..."));
        
        spawn_local(async move {
            let password_value = password.get_untracked();
            
            match Wallet::load().await {
                Ok(wallet) => {
                    spawn_local(async move {
                        match encrypt::decrypt_async(&wallet.get_encrypted_seed(), &password_value).await {
                            Ok(seed) => {
                                // Initialize network first
                                let network = selected_network.get_untracked();
                                if initialize_network(network) {
                                    let mut current_session = session.get_untracked();
                                    // Set network in session
                                    current_session.set_network(network);
                                    
                                    match current_session.initialize_with_seed(&seed).await {
                                        Ok(()) => {
                                            session.set(current_session);
                                            set_show_main_page.set(true);
                                        }
                                        Err(_) => {
                                            set_error_message.set("Failed to initialize session".to_string());
                                            button.set_disabled(false);
                                            button.set_text_content(Some("Login"));
                                        }
                                    }
                                } else {
                                    set_error_message.set("Failed to initialize network".to_string());
                                    button.set_disabled(false);
                                    button.set_text_content(Some("Login"));
                                }
                            }
                            Err(_) => {
                                set_error_message.set("Invalid password".to_string());
                                button.set_disabled(false);
                                button.set_text_content(Some("Login"));
                            }
                        }
                    });
                }
                Err(_) => {
                    set_error_message.set("Failed to load wallet".to_string());
                    button.set_disabled(false);
                    button.set_text_content(Some("Login"));
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
            // MEMO Token Logo
            <div class="logo-container">
                <img 
                    src="https://raw.githubusercontent.com/xenartist/memo-token/refs/heads/main/metadata/memo_token-logo.png" 
                    alt="MEMO Token Logo" 
                    class="memo-logo"
                />
            </div>
            
            <h1 class="app-title">"MEMO Engraves Memories Onchain"</h1>
            
            // Wallet type indicator
            <div class="wallet-type-indicator">
                <div class="wallet-type-badge">
                    <i class="fas fa-wallet wallet-icon"></i>
                    <span class="wallet-type-text">"Internal Wallet"</span>
                </div>
            </div>
            
            // Network selector
            <div class="network-selector-container">
                <label class="network-label">
                    <i class="fas fa-network-wired"></i>
                    " Select Network:"
                </label>
                <div class="network-options">
                    <button
                        class=move || if selected_network.get() == NetworkType::Testnet {
                            "network-option active network-testnet"
                        } else {
                            "network-option network-testnet"
                        }
                        on:click=move |_| selected_network.set(NetworkType::Testnet)
                    >
                        <div class="network-option-header">
                            <span class="network-name">"Testnet"</span>
                            <span class="network-badge network-badge-testnet">"DEV/TEST"</span>
                        </div>
                        <div class="network-description">
                            {NetworkType::Testnet.description()}
                        </div>
                    </button>
                    
                    <button
                        class=move || if selected_network.get() == NetworkType::ProdStaging {
                            "network-option active network-staging"
                        } else {
                            "network-option network-staging"
                        }
                        on:click=move |_| selected_network.set(NetworkType::ProdStaging)
                    >
                        <div class="network-option-header">
                            <span class="network-name">"Prod Staging"</span>
                            <span class="network-badge network-badge-staging">"STAGING"</span>
                        </div>
                        <div class="network-description">
                            {NetworkType::ProdStaging.description()}
                        </div>
                    </button>
                    
                    <button
                        class=move || if selected_network.get() == NetworkType::Mainnet {
                            "network-option active network-mainnet"
                        } else {
                            "network-option network-mainnet"
                        }
                        on:click=move |_| selected_network.set(NetworkType::Mainnet)
                    >
                        <div class="network-option-header">
                            <span class="network-name">"Mainnet"</span>
                            <span class="network-badge network-badge-mainnet">"PROD"</span>
                        </div>
                        <div class="network-description">
                            {NetworkType::Mainnet.description()}
                        </div>
                    </button>
                </div>
            </div>
            
            <div>
                {move || {
                    match reset_state.get() {
                        ResetState::Confirming => view! {
                            <div class="reset-confirm-dialog">
                                <div class="reset-confirm-content">
                                    <h3>
                                        <i class="fas fa-exclamation-triangle"></i>
                                        " Reset Wallet Confirmation"
                                    </h3>
                                    <p class="warning-text">
                                        <i class="fas fa-exclamation-circle"></i>
                                        " Warning: This action will delete current wallet and cannot be undone. "
                                        "Make sure you have backed up your mnemonic phrase before proceeding."
                                    </p>
                                    <div class="button-group">
                                        <button 
                                            class="cancel-btn"
                                            on:click=move |_| set_reset_state.set(ResetState::None)
                                        >
                                            <i class="fas fa-times"></i>
                                            " Cancel"
                                        </button>
                                        <button 
                                            class="reset-btn"
                                            on:click=handle_reset
                                        >
                                            <i class="fas fa-trash"></i>
                                            " Reset Wallet"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        },
                        ResetState::Success => view! {
                            <div class="reset-confirm-dialog">
                                <div class="reset-confirm-content">
                                    <h3>
                                        <i class="fas fa-check-circle" style="color: #059669;"></i>
                                        " Wallet Reset Successfully"
                                    </h3>
                                    <p>
                                        <i class="fas fa-info-circle"></i>
                                        " Your wallet has been reset successfully. You can now create a new wallet or import an existing one."
                                    </p>
                                    <div class="button-group">
                                        <button 
                                            class="wallet-btn"
                                            on:click=move |_| set_current_step.set(CreateWalletStep::Initial)
                                        >
                                            <i class="fas fa-arrow-right"></i>
                                            " Continue"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        },
                        ResetState::None => view! {
                            <div>
                                <form on:submit=on_submit>
                                    <div class="password-section">
                                        <label style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem; color: #555;">
                                            <i class="fas fa-lock"></i>
                                            <span>"Wallet Password"</span>
                                        </label>
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
                                        type="submit" 
                                        class="wallet-btn"
                                        id="login-button"
                                    >
                                        <i class="fas fa-sign-in-alt"></i>
                                        " Login"
                                    </button>

                                    <div class="reset-link">
                                        <a 
                                            href="#"
                                            on:click=move |ev| {
                                                ev.prevent_default();
                                                set_reset_state.set(ResetState::Confirming);
                                            }
                                        >
                                            <i class="fas fa-redo"></i>
                                            " Forget password and reset wallet"
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