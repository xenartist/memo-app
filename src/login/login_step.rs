use leptos::*;
use crate::core::wallet::Wallet;
use crate::core::encrypt;
use crate::core::session::Session;
use crate::core::{NetworkType, initialize_network};
use crate::core::rpc_base::RpcConnection;
use crate::core::rpc_burn::LatestBurn;
use crate::pages::pixel_view::LazyPixelView;
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
    let (latest_burn, set_latest_burn) = create_signal(Option::<LatestBurn>::None);
    
    // Fetch latest burn on component mount
    create_effect(move |_| {
        spawn_local(async move {
            match RpcConnection::get_latest_burn().await {
                Ok(Some(burn)) => {
                    log::info!("Loaded latest {} burn by {}", burn.burn_type, burn.user_pubkey);
                    set_latest_burn.set(Some(burn));
                }
                Ok(None) => {
                    log::info!("No recent burns found");
                }
                Err(e) => {
                    log::warn!("Failed to fetch latest burn: {}", e);
                }
            }
        });
    });

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
        <>
        // Latest burn card (outside login container)
        {move || {
            if let Some(burn) = latest_burn.get() {
                view! {
                    <div class="latest-burn-card-external">
                        <div class="latest-burn-content-external">
                            {if let Some(ref image) = burn.image {
                                view! {
                                    <div class="burn-avatar">
                                        <LazyPixelView
                                            art={image.clone()}
                                            size=64
                                        />
                                    </div>
                                }.into_view()
                            } else {
                                view! { <></> }.into_view()
                            }}
                            <div class="burn-info">
                                <div class="burn-header-line">
                                    <span class="burn-label">"Latest Burn"</span>
                                </div>
                                {if let Some(ref username) = burn.username {
                                    view! {
                                        <div class="burn-username">{username.clone()}</div>
                                    }.into_view()
                                } else {
                                    view! { <></> }.into_view()
                                }}
                                {if let Some(ref desc) = burn.description {
                                    view! {
                                        <div class="burn-description">{desc.clone()}</div>
                                    }.into_view()
                                } else {
                                    view! { <></> }.into_view()
                                }}
                            </div>
                            <div class="burn-amount-corner">
                                <i class="fas fa-fire-alt"></i>
                                " "
                                {format!("{}", burn.burn_amount)}
                                " MEMO"
                            </div>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! { <></> }.into_view()
            }
        }}
        
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
                <label class="network-label" for="network-select">
                    <i class="fas fa-network-wired"></i>
                    " Select X1 Network:"
                </label>
                <select
                    id="network-select"
                    class=move || match selected_network.get() {
                        NetworkType::Testnet => "network-select network-testnet",
                        NetworkType::ProdStaging => "network-select network-staging",
                        NetworkType::Mainnet => "network-select network-mainnet",
                    }
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        match value.as_str() {
                            "testnet" => selected_network.set(NetworkType::Testnet),
                            "prod-staging" => selected_network.set(NetworkType::ProdStaging),
                            "mainnet" => selected_network.set(NetworkType::Mainnet),
                            _ => {}
                        }
                    }
                >
                    <option value="mainnet" selected=move || selected_network.get() == NetworkType::Mainnet>
                        "Mainnet - Production"
                    </option>
                    <option value="prod-staging" selected=move || selected_network.get() == NetworkType::ProdStaging>
                        "Prod Staging"
                    </option>
                    <option value="testnet" selected=move || selected_network.get() == NetworkType::Testnet>
                        "Testnet - Dev/Test"
                    </option>
                </select>
                <div class="network-description-box">
                    {move || match selected_network.get() {
                        NetworkType::Testnet => view! {
                            <div class="network-description network-desc-testnet">
                                <span class="network-badge network-badge-testnet">"DEV/TEST"</span>
                                <span>{NetworkType::Testnet.description()}</span>
                            </div>
                        },
                        NetworkType::ProdStaging => view! {
                            <div class="network-description network-desc-staging">
                                <span class="network-badge network-badge-staging">"STAGING"</span>
                                <span>{NetworkType::ProdStaging.description()}</span>
                            </div>
                        },
                        NetworkType::Mainnet => view! {
                            <div class="network-description network-desc-mainnet">
                                <span class="network-badge network-badge-mainnet">"PRODUCTION"</span>
                                <span>{NetworkType::Mainnet.description()}</span>
                            </div>
                        },
                    }}
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
        </>
    }
} 